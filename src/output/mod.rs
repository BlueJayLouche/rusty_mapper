//! # Output Module
//!
//! Handles local video output to other applications via:
//! - Syphon (macOS)
//! - Spout (Windows)
//! - v4l2loopback (Linux)
//!
//! Also includes NDI output (network-based).

use std::sync::Arc;

/// Trait for all local video output mechanisms
pub trait LocalOutput: Send {
    /// Initialize the output with dimensions and format
    fn initialize(&mut self, width: u32, height: u32) -> anyhow::Result<()>;
    
    /// Submit a frame from wgpu texture
    fn submit_frame(&mut self, texture: &wgpu::Texture, queue: &wgpu::Queue) -> anyhow::Result<()>;
    
    /// Check if output is still connected/active
    fn is_connected(&self) -> bool;
    
    /// Get output name for UI
    fn name(&self) -> &str;
    
    /// Shutdown/cleanup
    fn shutdown(&mut self);
}

#[cfg(target_os = "macos")]
pub mod syphon;
#[cfg(target_os = "macos")]
pub use syphon::SyphonOutput;

// TODO: Add Windows (Spout) and Linux (v4l2loopback) implementations

/// Output manager that handles all output types
pub struct OutputManager {
    /// NDI network output
    ndi_output: Option<crate::ndi::NdiOutputSender>,
    
    /// Local GPU sharing output (platform-specific)
    #[cfg(target_os = "macos")]
    syphon_output: Option<SyphonOutput>,
    
    /// Frame counter for performance tracking
    frame_count: u64,
}

impl OutputManager {
    pub fn new() -> Self {
        Self {
            ndi_output: None,
            #[cfg(target_os = "macos")]
            syphon_output: None,
            frame_count: 0,
        }
    }
    
    /// Start NDI output
    pub fn start_ndi(&mut self, name: &str, width: u32, height: u32, include_alpha: bool) -> anyhow::Result<()> {
        let sender = crate::ndi::NdiOutputSender::new(name, width, height, include_alpha)?;
        self.ndi_output = Some(sender);
        log::info!("NDI output started: {} ({}x{})", name, width, height);
        Ok(())
    }
    
    /// Stop NDI output
    pub fn stop_ndi(&mut self) {
        if self.ndi_output.take().is_some() {
            log::info!("NDI output stopped");
        }
    }
    
    /// Start Syphon output (macOS only)
    #[cfg(target_os = "macos")]
    pub fn start_syphon(&mut self, server_name: &str, device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> anyhow::Result<()> {
        let mut syphon = SyphonOutput::new(server_name, device, queue)?;
        syphon.initialize(1920, 1080)?; // Default size, will update on first frame
        self.syphon_output = Some(syphon);
        log::info!("Syphon output started: {}", server_name);
        Ok(())
    }
    
    /// Stop Syphon output (macOS only)
    #[cfg(target_os = "macos")]
    pub fn stop_syphon(&mut self) {
        if let Some(mut syphon) = self.syphon_output.take() {
            syphon.shutdown();
            log::info!("Syphon output stopped");
        }
    }
    
    /// Check if Syphon is active (macOS only)
    #[cfg(target_os = "macos")]
    pub fn is_syphon_active(&self) -> bool {
        self.syphon_output.is_some()
    }
    
    /// Submit frame to all active outputs
    pub fn submit_frame(&mut self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.frame_count += 1;
        
        // NDI output (runs on separate thread)
        if let Some(ndi) = &self.ndi_output {
            // NDI handles its own readback
            // TODO: Integrate with texture
        }
        
        // Syphon output (macOS only, zero copy)
        #[cfg(target_os = "macos")]
        if let Some(syphon) = &mut self.syphon_output {
            if let Err(e) = syphon.submit_frame(texture, device, queue) {
                log::error!("Syphon output error: {}", e);
            }
        }
    }
    
    /// Shutdown all outputs
    pub fn shutdown(&mut self) {
        self.stop_ndi();
        #[cfg(target_os = "macos")]
        self.stop_syphon();
    }
}

impl Drop for OutputManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}
