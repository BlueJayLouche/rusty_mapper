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
            
            log::info!("[Webcam] Camera {} opened", device_index);
            
            // Capture loop
            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                
                // Capture frame
                match camera.frame() {
                    Ok(frame) => {
                        let rgba_data = frame.buffer().to_vec();
                        
                        let webcam_frame = WebcamFrame {
                            width,
                            height,
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
