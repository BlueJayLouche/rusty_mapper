//! # NDI Output Sender
//!
//! Sends video frames as an NDI stream.
//!
//! Architecture:
//! - Dedicated send thread to avoid blocking render loop
//! - Bounded channel for frame queue (drops old frames if consumer is slow)
//! - Low-latency design: minimal buffering, immediate send

use grafton_ndi::{NDI, Sender, SenderOptions, VideoFrame, VideoFrameBuilder, PixelFormat};
use crossbeam::channel::{self, Sender as ChannelSender, Receiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::Instant;

/// NDI video frame data (CPU side)
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // BGRA or BGRX format
    pub has_alpha: bool,
    pub timestamp: Instant,
}

/// NDI output sender
pub struct NdiOutputSender {
    name: String,
    width: u32,
    height: u32,
    include_alpha: bool,
    frame_tx: ChannelSender<FrameData>,
    running: Arc<AtomicBool>,
    /// Whether this is the original sender (owner) or a clone
    /// Only the owner should stop the thread on drop
    is_owner: bool,
}

impl NdiOutputSender {
    /// Create and start a new NDI output sender
    ///
    /// # Arguments
    /// * `name` - The NDI source name that will appear to receivers
    /// * `width` - Output width in pixels
    /// * `height` - Output height in pixels
    /// * `include_alpha` - Whether to include alpha channel (BGRA vs BGRX)
    pub fn new(name: impl Into<String>, width: u32, height: u32, include_alpha: bool) -> anyhow::Result<Self> {
        let name = name.into();
        // Validate dimensions
        if width == 0 || height == 0 {
            return Err(anyhow::anyhow!("Invalid dimensions: {}x{}", width, height));
        }
        
        let ndi = NDI::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize NDI: {:?}", e))?;
        
        // Create bounded channel for frame queue (keep only latest 2 frames)
        let (frame_tx, frame_rx) = channel::bounded(2);
        
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        
        let name_clone = name.clone();
        
        // Spawn send thread
        let thread_handle = thread::spawn(move || {
            Self::send_thread(
                ndi,
                name_clone,
                width,
                height,
                include_alpha,
                frame_rx,
                running_clone,
            );
        });
        
        // Leak the thread handle to prevent it from being dropped
        // This keeps the thread running even if the handle goes out of scope
        Box::leak(Box::new(thread_handle));
        
        Ok(Self {
            name,
            width,
            height,
            include_alpha,
            frame_tx,
            running,
            is_owner: true,  // This is the original sender
        })
    }
    
    /// Send thread that owns the NDI sender and processes frames
    fn send_thread(
        ndi: NDI,
        name: String,
        width: u32,
        height: u32,
        include_alpha: bool,
        frame_rx: Receiver<FrameData>,
        running: Arc<AtomicBool>,
    ) {
        // Create NDI sender (video clock enabled as required by NDI SDK)
        let options = SenderOptions::builder(&name)
            .clock_video(true)
            .clock_audio(false)
            .build();
        
        let sender = match Sender::new(&ndi, &options) {
            Ok(s) => s,
            Err(e) => {
                log::error!("[NDI OUTPUT] Failed to create NDI sender: {:?}", e);
                return;
            }
        };
        
        let pixel_format = if include_alpha {
            PixelFormat::BGRA
        } else {
            PixelFormat::BGRX
        };
        
        let mut frame_count = 0u64;
        let mut last_log = Instant::now();
        
        while running.load(Ordering::SeqCst) {
            // Try to receive frame with timeout
            match frame_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(frame_data) => {
                    frame_count += 1;
                    // Silently process frames - no per-frame logging in normal operation
                    
                    // Calculate expected buffer size
                    let buffer_size = pixel_format.buffer_size(frame_data.width as i32, frame_data.height as i32);
                    
                    // Validate data length
                    if frame_data.data.len() < buffer_size {
                        log::warn!("[NDI OUTPUT] Frame {} data too small (expected {}, got {})", 
                            frame_count, buffer_size, frame_data.data.len());
                        continue;
                    }
                    
                    // Create and send NDI video frame
                    let mut frame = match VideoFrameBuilder::new()
                        .resolution(frame_data.width as i32, frame_data.height as i32)
                        .pixel_format(pixel_format)
                        .frame_rate(60, 1)
                        .aspect_ratio(frame_data.width as f32 / frame_data.height as f32)
                        .build() {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("[NDI OUTPUT] Failed to build video frame: {:?}", e);
                            continue;
                        }
                    };
                    
                    // Copy data and send
                    let copy_len = buffer_size.min(frame.data.len());
                    frame.data[..copy_len].copy_from_slice(&frame_data.data[..copy_len]);
                    sender.send_video(&frame);
                    
                    // Log stats periodically (every 30 seconds in production)
                    if last_log.elapsed().as_secs() >= 30 {
                        log::info!("[NDI OUTPUT] {} frames sent to '{}'", frame_count, name);
                        last_log = Instant::now();
                    }
                }
                Err(channel::RecvTimeoutError::Timeout) => {
                    // No frame available, continue loop
                }
                Err(channel::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
        
    }
    
    /// Submit a frame for sending
    /// 
    /// The data should be in RGBA format. It will be converted to BGRA/BGRX internally.
    /// If the channel is full, the oldest frame will be dropped.
    pub fn submit_frame(&self, rgba_data: &[u8], width: u32, height: u32) {
        // Validate dimensions match
        if width != self.width || height != self.height {
            log::warn!("[NDI OUTPUT] Frame size mismatch: expected {}x{}, got {}x{}",
                self.width, self.height, width, height);
            return;
        }
        
        // Validate data is not empty
        if rgba_data.is_empty() {
            log::warn!("[NDI OUTPUT] Empty frame data received");
            return;
        }
        
        // Convert RGBA to BGRA/BGRX
        let bgra_data = convert_rgba_to_bgra(rgba_data, width, height, self.include_alpha);
        
        let frame = FrameData {
            width,
            height,
            data: bgra_data,
            has_alpha: self.include_alpha,
            timestamp: Instant::now(),
        };
        
        // Try to send (non-blocking)
        match self.frame_tx.try_send(frame) {
            Ok(_) => {
                log::debug!("[NDI OUTPUT] Frame queued: {}x{}", width, height);
            }
            Err(channel::TrySendError::Full(_)) => {
                // Channel full, drop this frame for low latency
                log::debug!("[NDI OUTPUT] Frame dropped - channel full");
            }
            Err(channel::TrySendError::Disconnected(_)) => {
                log::warn!("[NDI OUTPUT] Frame channel disconnected");
            }
        }
    }
    
    /// Stop the NDI sender
    /// Only the owner should call this - clones share the running flag
    pub fn stop(&mut self) {
        if !self.is_owner {
            // Clones don't stop the thread - only the owner does
            return;
        }
        self.running.store(false, Ordering::SeqCst);
    }
    
    /// Check if sender is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// Get output dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Clone for NdiOutputSender {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            width: self.width,
            height: self.height,
            include_alpha: self.include_alpha,
            frame_tx: self.frame_tx.clone(),
            running: Arc::clone(&self.running),
            is_owner: false,  // Clones don't own the thread
        }
    }
}

impl Drop for NdiOutputSender {
    fn drop(&mut self) {
        if self.is_owner {
            self.stop();
        }
        // Clones don't control the thread - silently drop them
    }
}

/// Convert RGBA data to BGRA/BGRX
/// 
/// wgpu typically uses BGRA on macOS, so this might be a simple copy in some cases.
/// But we always do the conversion to be safe.
fn convert_rgba_to_bgra(rgba_data: &[u8], width: u32, height: u32, include_alpha: bool) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut bgra_data = vec![0u8; pixel_count * 4];
    
    // Check if data size matches
    let expected_size = pixel_count * 4;
    if rgba_data.len() != expected_size {
        log::warn!("[NDI OUTPUT] RGBA data size mismatch: got {}, expected {}", 
            rgba_data.len(), expected_size);
        // Use minimum of both sizes
        let safe_pixels = (rgba_data.len() / 4).min(pixel_count);
        for i in 0..safe_pixels {
            let src_idx = i * 4;
            let dst_idx = i * 4;
            bgra_data[dst_idx] = rgba_data[src_idx + 2];     // B <- R
            bgra_data[dst_idx + 1] = rgba_data[src_idx + 1]; // G <- G
            bgra_data[dst_idx + 2] = rgba_data[src_idx];     // R <- B
            bgra_data[dst_idx + 3] = if include_alpha { rgba_data[src_idx + 3] } else { 0xFF }; // A
        }
    } else {
        // Fast path: sizes match
        for i in 0..pixel_count {
            let src_idx = i * 4;
            let dst_idx = i * 4;
            bgra_data[dst_idx] = rgba_data[src_idx + 2];     // B <- R
            bgra_data[dst_idx + 1] = rgba_data[src_idx + 1]; // G <- G
            bgra_data[dst_idx + 2] = rgba_data[src_idx];     // R <- B
            bgra_data[dst_idx + 3] = if include_alpha { rgba_data[src_idx + 3] } else { 0xFF }; // A
        }
    }
    
    bgra_data
}

/// Check if NDI output is available
pub fn is_ndi_output_available() -> bool {
    NDI::new().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_bgra_conversion() {
        // Test data: 2x1 pixel RGBA image
        let rgba = vec![
            255, 0, 0, 255,    // Red (RGBA) -> Blue (BGRA)
            0, 255, 0, 255,    // Green stays green
        ];
        
        let bgra = convert_rgba_to_bgra(&rgba, 2, 1, true);
        
        assert_eq!(bgra[0], 0);      // B
        assert_eq!(bgra[1], 0);      // G
        assert_eq!(bgra[2], 255);    // R (was B in RGBA)
        assert_eq!(bgra[3], 255);    // A
        
        assert_eq!(bgra[4], 0);      // B
        assert_eq!(bgra[5], 255);    // G
        assert_eq!(bgra[6], 0);      // R
        assert_eq!(bgra[7], 255);    // A
    }
    
    #[test]
    fn test_rgba_to_bgrx_conversion() {
        // Test data: 1x1 pixel RGBA image
        let rgba = vec![255, 0, 0, 128]; // Red with 50% alpha
        
        let bgrx = convert_rgba_to_bgra(&rgba, 1, 1, false);
        
        assert_eq!(bgrx[0], 0);      // B
        assert_eq!(bgrx[1], 0);      // G
        assert_eq!(bgrx[2], 255);    // R
        assert_eq!(bgrx[3], 0xFF);   // X (forced to 255)
    }
}
