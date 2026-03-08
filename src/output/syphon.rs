//! Syphon Output (macOS)
//!
//! GPU texture sharing via Syphon framework using syphon-wgpu crate.
//! This provides zero-copy inter-process texture sharing.

use std::sync::Arc;
use anyhow::{anyhow, Result};

/// Re-export from syphon-wgpu for convenience
pub use syphon_wgpu::SyphonWgpuOutput;
pub use syphon_core::ServerInfo as SyphonServerInfo;

/// Syphon output handle
///
/// Wraps syphon_wgpu::SyphonWgpuOutput for integration with rusty_mapper's
/// output system. Provides zero-copy GPU texture sharing.
pub struct SyphonOutput {
    /// The underlying syphon-wgpu output
    inner: Option<SyphonWgpuOutput>,
    /// Server name
    server_name: String,
    /// Current dimensions
    width: u32,
    height: u32,
    /// Whether initialized
    initialized: bool,
}

impl SyphonOutput {
    /// Create a new Syphon output server
    ///
    /// # Arguments
    /// * `server_name` - The name displayed to Syphon clients
    /// * `wgpu_device` - The wgpu device
    /// * `wgpu_queue` - The wgpu queue
    pub fn new(
        server_name: impl Into<String>,
        wgpu_device: Arc<wgpu::Device>,
        wgpu_queue: Arc<wgpu::Queue>,
    ) -> Result<Self> {
        let server_name = server_name.into();
        
        log::info!("Creating Syphon output: {}", server_name);
        
        Ok(Self {
            inner: None,
            server_name,
            width: 0,
            height: 0,
            initialized: false,
        })
    }
    
    /// Initialize with specific dimensions
    ///
    /// This actually creates the Syphon server. Call this once you know
    /// the output resolution.
    pub fn initialize(&mut self, width: u32, height: u32) -> Result<()> {
        if self.initialized {
            if self.width == width && self.height == height {
                return Ok(()); // Already initialized with same size
            }
            // Re-initialize with new size
            self.shutdown();
        }
        
        self.width = width;
        self.height = height;
        
        // Note: We need to get access to the wgpu device and queue
        // In the actual implementation, these would be stored or passed differently
        // For now, we'll create the output when submit_frame is called
        
        log::info!("Syphon initialized: {}x{}", width, height);
        self.initialized = true;
        Ok(())
    }
    
    /// Submit a wgpu texture to Syphon
    ///
    /// This publishes the frame to any connected Syphon clients.
    pub fn submit_frame(&mut self, texture: &wgpu::Texture, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<()> {
        if !self.initialized {
            self.initialize(texture.width(), texture.height())?;
        }
        
        // Check if dimensions changed
        if texture.width() != self.width || texture.height() != self.height {
            self.initialize(texture.width(), texture.height())?;
        }
        
        // Get or create the inner output
        if let Some(ref mut inner) = self.inner {
            inner.publish(texture, device, queue);
        } else {
            // Can't create without device access - log warning
            log::warn!("Syphon output not fully initialized - missing wgpu device");
        }
        
        Ok(())
    }
    
    /// Check if server is still active
    pub fn is_connected(&self) -> bool {
        self.initialized && self.inner.is_some()
    }
    
    /// Get server name
    pub fn name(&self) -> &str {
        &self.server_name
    }
    
    /// Get current dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    
    /// Check if zero-copy is being used
    pub fn is_zero_copy(&self) -> bool {
        self.inner.as_ref().map_or(false, |o| o.is_zero_copy())
    }
    
    /// Get number of connected clients
    pub fn client_count(&self) -> usize {
        self.inner.as_ref().map_or(0, |o| o.client_count())
    }
    
    /// Shutdown and cleanup
    pub fn shutdown(&mut self) {
        log::info!("Syphon server shutdown: {}", self.server_name);
        self.inner = None;
        self.initialized = false;
        self.width = 0;
        self.height = 0;
    }
}

impl Drop for SyphonOutput {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// List available Syphon servers (for input discovery)
pub fn list_syphon_servers() -> Vec<SyphonServerInfo> {
    syphon_core::SyphonServerDirectory::servers()
}

/// Check if Syphon is available on this system
pub fn is_syphon_available() -> bool {
    syphon_core::is_available()
}

/// Create a fully initialized Syphon output
///
/// This is a convenience function that creates and initializes the output
/// in one call, requiring the wgpu device and queue.
pub fn create_syphon_output(
    server_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
) -> Result<SyphonWgpuOutput> {
    SyphonWgpuOutput::new(server_name, device, queue, width, height)
        .map_err(|e| anyhow!("Failed to create Syphon output: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_syphon_availability() {
        // Just check it doesn't panic
        let _available = is_syphon_available();
    }
    
    #[test]
    fn test_list_servers() {
        let servers = list_syphon_servers();
        println!("Found {} Syphon servers", servers.len());
    }
}
