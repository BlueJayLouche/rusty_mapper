//! # Webcam Capture
//!
//! Webcam video capture using nokhwa 0.10.
//!
//! Feature flag: `webcam` (enabled by default)
//! Disable with: `cargo build --no-default-features`

use nokhwa::Camera;
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType, Resolution};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// Convert YUY2 (YUV 4:2:2) to RGBA
/// Input: YUY2 format (4 bytes for 2 pixels)
/// Output: RGBA format (4 bytes per pixel)
fn yuy2_to_rgba(yuy2_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgba = vec![0u8; pixel_count * 4];
    
    for i in 0..pixel_count {
        let yuy2_idx = (i / 2) * 4;
        if yuy2_idx + 3 >= yuy2_data.len() {
            break;
        }
        
        // YUY2 layout: [Y0, U, Y1, V]
        let y = if i % 2 == 0 { yuy2_data[yuy2_idx] } else { yuy2_data[yuy2_idx + 2] };
        let u = yuy2_data[yuy2_idx + 1];
        let v = yuy2_data[yuy2_idx + 3];
        
        // Convert YUV to RGB
        let y = y as f32;
        let u = u as f32 - 128.0;
        let v = v as f32 - 128.0;
        
        let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
        let g = (y - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
        let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
        
        let rgba_idx = i * 4;
        rgba[rgba_idx] = r;
        rgba[rgba_idx + 1] = g;
        rgba[rgba_idx + 2] = b;
        rgba[rgba_idx + 3] = 255; // Alpha
    }
    
    rgba
}

/// A webcam video frame
pub struct WebcamFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA format
    pub timestamp: Instant,
}

/// Webcam capture handler
pub struct WebcamCapture {
    device_index: usize,
    width: u32,
    height: u32,
    fps: u32,
    capture_thread: Option<JoinHandle<()>>,
    stop_signal: Option<Sender<()>>,
}

impl WebcamCapture {
    /// Create a new webcam capture (does not start)
    pub fn new(device_index: usize, width: u32, height: u32, fps: u32) -> anyhow::Result<Self> {
        Ok(Self {
            device_index,
            width,
            height,
            fps,
            capture_thread: None,
            stop_signal: None,
        })
    }
    
    /// Start capturing frames
    pub fn start(&mut self) -> anyhow::Result<Receiver<WebcamFrame>> {
        if self.capture_thread.is_some() {
            return Err(anyhow::anyhow!("Webcam already started"));
        }
        
        let (frame_tx, frame_rx) = mpsc::channel::<WebcamFrame>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        
        let device_index = self.device_index;
        let width = self.width;
        let height = self.height;
        let fps = self.fps;
        
        let thread_handle = thread::spawn(move || {
            // Create camera index
            let index = CameraIndex::Index(device_index as u32);
            
            // Create requested format - use RgbAFormat decoder
            let format = RequestedFormat::new::<RgbAFormat>(
                RequestedFormatType::AbsoluteHighestResolution
            );
            
            let mut camera = match Camera::new(index, format) {
                Ok(cam) => cam,
                Err(e) => {
                    log::error!("[Webcam] Failed to open camera {}: {:?}", device_index, e);
                    return;
                }
            };
            
            if let Err(e) = camera.open_stream() {
                log::error!("[Webcam] Failed to open stream: {:?}", e);
                return;
            }
            
            // Get actual camera resolution after opening
            let actual_resolution = camera.resolution();
            let actual_width = actual_resolution.width() as u32;
            let actual_height = actual_resolution.height() as u32;
            
            log::info!("[Webcam] Camera {} opened at {}x{}", 
                device_index, actual_width, actual_height);
            
            // Capture loop
            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                
                // Capture frame
                match camera.frame() {
                    Ok(frame) => {
                        let buffer = frame.buffer();
                        let expected_rgba_size = (actual_width * actual_height * 4) as usize;
                        
                        // Convert to RGBA if needed
                        let rgba_data = if buffer.len() == expected_rgba_size {
                            // Already RGBA
                            buffer.to_vec()
                        } else if buffer.len() == (actual_width * actual_height * 2) as usize {
                            // YUY2 format - convert to RGBA
                            yuy2_to_rgba(buffer, actual_width, actual_height)
                        } else {
                            // Unknown format, try to use as-is (may cause visual issues)
                            log::warn!("[Webcam] Unknown frame format: {} bytes for {}x{}", 
                                buffer.len(), actual_width, actual_height);
                            buffer.to_vec()
                        };
                        
                        // Use actual camera resolution, not requested
                        let webcam_frame = WebcamFrame {
                            width: actual_width,
                            height: actual_height,
                            data: rgba_data,
                            timestamp: Instant::now(),
                        };
                        
                        // Send frame (drop if channel full/closed)
                        if frame_tx.send(webcam_frame).is_err() {
                            // Channel closed
                            break;
                        }
                    }
                    Err(e) => {
                        log::warn!("[Webcam] Frame capture error: {:?}", e);
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
            
            let _ = camera.stop_stream();
            log::info!("[Webcam] Camera {} stopped", device_index);
        });
        
        self.capture_thread = Some(thread_handle);
        self.stop_signal = Some(stop_tx);
        
        Ok(frame_rx)
    }
    
    /// Stop capturing
    pub fn stop(&mut self) -> anyhow::Result<()> {
        // Send stop signal
        if let Some(stop_tx) = self.stop_signal.take() {
            let _ = stop_tx.send(());
        }
        
        // Wait for thread to finish
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
        
        Ok(())
    }
}

impl Drop for WebcamCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// List available webcam devices
pub fn list_cameras() -> Vec<String> {
    // nokhwa 0.10 uses native backends for device enumeration
    // Try to detect cameras by attempting to open them with index 0-3
    let mut cameras = Vec::new();
    
    for i in 0..4 {
        let index = CameraIndex::Index(i as u32);
        let format = RequestedFormat::new::<RgbAFormat>(
            RequestedFormatType::AbsoluteHighestResolution
        );
        
        match Camera::new(index, format) {
            Ok(cam) => {
                let name = format!("Camera {}", i);
                cameras.push(name);
                // Don't keep the camera open
                drop(cam);
            }
            Err(_) => {
                // Camera not available
            }
        }
    }
    
    cameras
}
