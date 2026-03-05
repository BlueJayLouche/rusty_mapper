//! # NDI Input Implementation
//!
//! Provides NDI video input support using grafton-ndi.
//! 
//! Architecture:
//! - NdiSourceFinder: Discovers available NDI sources on the network
//! - NdiReceiver: Receives video frames from an NDI source in a background thread
//! - Frame queue with latest-frame-only semantics to prevent buildup

use grafton_ndi::{NDI, Finder, FinderOptions, Receiver, ReceiverOptions, ReceiverColorFormat, ReceiverBandwidth};
use crossbeam::channel::{self, Sender, Receiver as CrossbeamReceiver};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Information about an available NDI source
#[derive(Debug, Clone)]
pub struct NdiSourceInfo {
    pub name: String,
    pub url: String,
}

/// A received NDI video frame
pub struct NdiFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA format
    pub timestamp: Instant,
}

/// NDI source finder for discovering sources on the network
pub struct NdiSourceFinder;

impl NdiSourceFinder {
    /// Find available NDI sources on the network
    /// 
    /// # Arguments
    /// * `timeout_ms` - How long to wait for sources to appear
    /// 
    /// # Returns
    /// List of available NDI source names
    pub fn find_sources(timeout_ms: u32) -> Vec<NdiSourceInfo> {
        let ndi = match NDI::new() {
            Ok(ndi) => ndi,
            Err(e) => {
                log::error!("Failed to initialize NDI: {:?}", e);
                return Vec::new();
            }
        };

        let options = FinderOptions::builder()
            .show_local_sources(true)
            .build();

        let finder = match Finder::new(&ndi, &options) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to create NDI finder: {:?}", e);
                return Vec::new();
            }
        };

        // Get sources with timeout
        match finder.find_sources(Duration::from_millis(timeout_ms as u64)) {
            Ok(sources) => {
                sources.into_iter()
                    .map(|source| {
                        let name = source.name.clone();
                        let url = format!("{:?}", source.address);
                        NdiSourceInfo { name, url }
                    })
                    .collect()
            }
            Err(e) => {
                log::error!("Failed to find NDI sources: {:?}", e);
                Vec::new()
            }
        }
    }
}

/// NDI receiver that captures video frames from a source
pub struct NdiReceiver {
    source_name: String,
    receiver_thread: Option<JoinHandle<()>>,
    frame_tx: Sender<NdiFrame>,
    frame_rx: CrossbeamReceiver<NdiFrame>,
    running: Arc<AtomicBool>,
    resolution: (u32, u32),
}

impl NdiReceiver {
    /// Create a new NDI receiver (does not start receiving yet)
    pub fn new(source_name: impl Into<String>) -> Self {
        let (frame_tx, frame_rx) = channel::bounded(5); // Bounded channel with 5 frame capacity
        
        Self {
            source_name: source_name.into(),
            receiver_thread: None,
            frame_tx,
            frame_rx,
            running: Arc::new(AtomicBool::new(false)),
            resolution: (1920, 1080), // Default, updated on first frame
        }
    }

    /// Start receiving from the NDI source
    /// 
    /// This spawns a background thread that continuously receives frames
    /// from the NDI source and sends them through the channel.
    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.receiver_thread.is_some() {
            return Err(anyhow::anyhow!("NDI receiver already started"));
        }

        let ndi = NDI::new().map_err(|e| {
            anyhow::anyhow!("Failed to initialize NDI: {:?}", e)
        })?;

        let source_name = self.source_name.clone();
        let frame_tx = self.frame_tx.clone();
        let running = Arc::clone(&self.running);
        running.store(true, Ordering::SeqCst);

        let thread_handle = thread::spawn(move || {

            // Find the source
            let options = FinderOptions::builder()
                .show_local_sources(true)
                .build();

            let finder = match Finder::new(&ndi, &options) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("[NDI] Failed to create finder: {:?}", e);
                    return;
                }
            };

            // Wait for the specific source
            let mut found_source = None;
            let search_start = Instant::now();
            
            while running.load(Ordering::SeqCst) && search_start.elapsed().as_secs() < 10 {
                match finder.find_sources(Duration::from_millis(100)) {
                    Ok(sources) => {
                        for source in sources {
                            if source.name.contains(&source_name) || source_name.contains(&source.name) {
                                found_source = Some(source);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!("[NDI] Error finding sources: {:?}", e);
                    }
                }
                
                if found_source.is_some() {
                    break;
                }
                
                thread::sleep(Duration::from_millis(50));
            }

            let source = match found_source {
                Some(s) => s,
                None => {
                    log::error!("[NDI] Could not find source: {}", source_name);
                    return;
                }
            };

            // Create receiver with BGRA format (we'll convert to RGBA)
            let options = ReceiverOptions::builder(source)
                .color(ReceiverColorFormat::BGRX_BGRA)
                .bandwidth(ReceiverBandwidth::Highest)
                .build();

            let receiver = match Receiver::new(&ndi, &options) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("[NDI] Failed to create receiver: {:?}", e);
                    return;
                }
            };

            log::info!("[NDI] Connected to: {}", source_name);

            // Receive loop
            while running.load(Ordering::SeqCst) {
                match receiver.capture_video_ref(Duration::from_millis(100)) {
                    Ok(Some(video_frame)) => {
                        // Get frame dimensions
                        let width = video_frame.width() as u32;
                        let height = video_frame.height() as u32;
                        
                        // Get frame data
                        let frame_data = video_frame.data();
                        
                        // Convert BGRA to RGBA
                        let data = convert_bgra_to_rgba(frame_data, width, height);

                        let frame = NdiFrame {
                            width,
                            height,
                            data,
                            timestamp: Instant::now(),
                        };

                        // Send frame (non-blocking, drop if channel full)
                        let _ = frame_tx.try_send(frame);
                    }
                    Ok(None) => {
                        // No frame available (timeout)
                    }
                    Err(e) => {
                        log::error!("[NDI] Frame capture error: {:?}", e);
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }

        });

        self.receiver_thread = Some(thread_handle);
        Ok(())
    }

    /// Stop receiving frames
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        
        if let Some(handle) = self.receiver_thread.take() {
            // Wait for thread to finish (with timeout)
            let _ = handle.join();
        }

        log::info!("[NDI] Receiver stopped for source: {}", self.source_name);
    }

    /// Get the latest frame (non-blocking, consumes the frame)
    pub fn get_latest_frame(&mut self) -> Option<NdiFrame> {
        // Drain all available frames and return only the most recent
        let mut latest: Option<NdiFrame> = None;
        while let Ok(frame) = self.frame_rx.try_recv() {
            self.resolution = (frame.width, frame.height);
            latest = Some(frame);
        }
        latest
    }

    /// Check if a new frame is available
    pub fn has_frame(&self) -> bool {
        !self.frame_rx.is_empty()
    }

    /// Get current resolution
    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }

    /// Check if receiver is running
    pub fn is_running(&self) -> bool {
        self.receiver_thread.is_some() && self.running.load(Ordering::SeqCst)
    }
}

impl Drop for NdiReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Convert BGRA data to RGBA
/// 
/// NDI typically sends BGRA, but wgpu/shaders expect RGBA
/// Note: NDI frame data may have row stride/padding
fn convert_bgra_to_rgba(bgra_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgba_data = vec![0u8; pixel_count * 4];
    
    // Calculate stride - NDI often uses aligned rows
    // The data length divided by height gives us the actual stride
    let actual_stride = if height > 0 {
        bgra_data.len() / height as usize
    } else {
        width as usize * 4
    };
    
    let expected_stride = width as usize * 4;
    
    log::debug!("[NDI] Converting frame: {}x{}, data_len={}, actual_stride={}, expected_stride={}",
        width, height, bgra_data.len(), actual_stride, expected_stride);

    for y in 0..height as usize {
        for x in 0..width as usize {
            let src_idx = y * actual_stride + x * 4;
            let dst_idx = (y * width as usize + x) * 4;
            
            if src_idx + 3 < bgra_data.len() && dst_idx + 3 < rgba_data.len() {
                // BGRA -> RGBA: swap B and R
                rgba_data[dst_idx] = bgra_data[src_idx + 2];     // R <- B
                rgba_data[dst_idx + 1] = bgra_data[src_idx + 1]; // G <- G
                rgba_data[dst_idx + 2] = bgra_data[src_idx];     // B <- R
                rgba_data[dst_idx + 3] = bgra_data[src_idx + 3]; // A <- A
            }
        }
    }

    rgba_data
}

/// Global NDI availability check
pub fn is_ndi_available() -> bool {
    NDI::new().is_ok()
}

/// Quick function to list available NDI sources
pub fn list_ndi_sources(timeout_ms: u32) -> Vec<String> {
    NdiSourceFinder::find_sources(timeout_ms)
        .into_iter()
        .map(|info| info.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgra_to_rgba_conversion() {
        // Test data: 2x1 pixel BGRA image
        let bgra = vec![
            255, 0, 0, 255,    // Blue (BGRA) -> Red (RGBA)
            0, 255, 0, 255,    // Green stays green
        ];
        
        let rgba = convert_bgra_to_rgba(&bgra, 2, 1);
        
        assert_eq!(rgba[0], 0);      // R
        assert_eq!(rgba[1], 0);      // G
        assert_eq!(rgba[2], 255);    // B (was R in BGRA)
        assert_eq!(rgba[3], 255);    // A
        
        assert_eq!(rgba[4], 0);      // R
        assert_eq!(rgba[5], 255);    // G
        assert_eq!(rgba[6], 0);      // B
        assert_eq!(rgba[7], 255);    // A
    }
}
