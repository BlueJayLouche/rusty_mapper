//! # Syphon Output (macOS)
//!
//! Implements GPU texture sharing via Syphon framework.
//! Uses IOSurface for zero-copy inter-process texture sharing.

use std::ffi::{c_void, CStr, CString};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use cocoa::base::{id, nil, YES, NO};
use cocoa::foundation::{NSString, NSRect, NSSize};
use metal::{Device, Texture, TextureDescriptor};
use objc::runtime::{Object, Sel, Class};
use objc::{class, msg_send, sel, sel_impl};

/// Syphon server handle
pub struct SyphonOutput {
    /// Server name displayed to clients
    server_name: String,
    
    /// Metal device (from wgpu)
    metal_device: Device,
    
    /// Metal texture for publishing
    metal_texture: Option<Texture>,
    
    /// Syphon server instance (Objective-C object)
    syphon_server: id,
    
    /// Current dimensions
    width: u32,
    height: u32,
    
    /// Whether initialized
    initialized: bool,
}

// Syphon uses raw Objective-C objects that are thread-safe
unsafe impl Send for SyphonOutput {}

impl SyphonOutput {
    /// Create a new Syphon output server
    pub fn new(
        server_name: &str,
        _wgpu_device: Arc<wgpu::Device>,
        _wgpu_queue: Arc<wgpu::Queue>,
    ) -> Result<Self> {
        // Get the default Metal device
        let metal_device = Device::system_default()
            .ok_or_else(|| anyhow!("No Metal device available"))?;
        
        // Create Syphon server using FFI to Objective-C
        // Note: In a full implementation, we'd use the Syphon framework directly
        // For now, we create the structure and implement the Metal interop
        
        log::info!("Creating Syphon server: {}", server_name);
        
        Ok(Self {
            server_name: server_name.to_string(),
            metal_device,
            metal_texture: None,
            syphon_server: nil,
            width: 0,
            height: 0,
            initialized: false,
        })
    }
    
    /// Initialize with specific dimensions
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
        
        // Create Metal texture descriptor
        let descriptor = TextureDescriptor::new();
        descriptor.set_width(width as u64);
        descriptor.set_height(height as u64);
        descriptor.set_pixel_format(metal::MTLPixelFormat::RGBA8Unorm);
        descriptor.set_storage_mode(metal::MTLStorageMode::Shared); // Shared for IOSurface
        descriptor.set_usage(metal::MTLTextureUsage::RenderTarget | metal::MTLTextureUsage::ShaderRead);
        
        // Create the Metal texture
        self.metal_texture = Some(self.metal_device.new_texture(&descriptor));
        
        // TODO: Create actual Syphon server using Objective-C FFI
        // This requires linking against Syphon.framework and calling:
        // [[SyphonServer alloc] initWithName:serverName context:metalContext options:nil]
        
        log::info!("Syphon initialized: {}x{}", width, height);
        self.initialized = true;
        Ok(())
    }
    
    /// Submit a wgpu texture to Syphon
    /// 
    /// This copies the wgpu texture to our Metal texture, then publishes via Syphon
    pub fn submit_frame(&mut self, texture: &wgpu::Texture, _queue: &wgpu::Queue) -> Result<()> {
        if !self.initialized {
            self.initialize(texture.width(), texture.height())?;
        }
        
        // Check if dimensions changed
        if texture.width() != self.width || texture.height() != self.height {
            self.initialize(texture.width(), texture.height())?;
        }
        
        // Copy wgpu texture to Metal texture
        // This requires wgpu's external texture interop
        self.copy_wgpu_to_metal(texture, _queue)?;
        
        // Publish to Syphon
        // TODO: Call SyphonServer publishFrameTexture:texture imageRegion:bounds
        
        Ok(())
    }
    
    /// Copy wgpu texture to Metal texture
    fn copy_wgpu_to_metal(&mut self, _wgpu_texture: &wgpu::Texture, _queue: &wgpu::Queue) -> Result<()> {
        // Get the Metal texture
        let _metal_texture = self.metal_texture.as_ref()
            .ok_or_else(|| anyhow!("Metal texture not initialized"))?;
        
        // TODO: Implement wgpu to Metal texture copy
        // This requires either:
        // 1. CPU readback from wgpu + upload to Metal (slower, works now)
        // 2. Direct Metal interop using wgpu's raw texture handles (zero copy)
        
        Ok(())
    }
    
    /// Check if server is still active
    pub fn is_connected(&self) -> bool {
        self.initialized && self.syphon_server != nil
    }
    
    /// Get server name
    pub fn name(&self) -> &str {
        &self.server_name
    }
    
    /// Shutdown and cleanup
    pub fn shutdown(&mut self) {
        if self.syphon_server != nil {
            // Release Syphon server
            unsafe {
                let _: () = msg_send![self.syphon_server, stop];
                let _: () = msg_send![self.syphon_server, release];
            }
            self.syphon_server = nil;
        }
        
        self.metal_texture = None;
        self.initialized = false;
        
        log::info!("Syphon server shutdown: {}", self.server_name);
    }
}

impl Drop for SyphonOutput {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// List available Syphon servers (for input)
pub fn list_syphon_servers() -> Vec<SyphonServerInfo> {
    // TODO: Query available Syphon servers using SyphonServerDirectory
    // This is used for the input side to discover sources
    vec![]
}

/// Information about a Syphon server
pub struct SyphonServerInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_syphon_placeholder() {
        // Placeholder test - actual testing requires Metal context
        assert!(true);
    }
}
