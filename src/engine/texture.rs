//! # Texture Utilities
//!
//! Helper functions for wgpu texture management.

use wgpu::util::DeviceExt;

/// Texture wrapper with common operations
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    /// Create a Texture from an existing wgpu texture
    /// 
    /// This is useful for zero-copy integration with external texture sources like Syphon
    pub fn from_wgpu_texture(
        texture: wgpu::Texture,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Self {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }
    
    /// Create a texture from raw RGBA data (for input textures with COPY_SRC for preview)
    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        label: &str,
        data: &[u8],
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING 
                | wgpu::TextureUsages::COPY_DST 
                | wgpu::TextureUsages::COPY_SRC,  // Added COPY_SRC for preview
            view_formats: &[],
        });
        
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }
    
    /// Create a render target texture (default format: Bgra8Unorm)
    pub fn create_render_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        Self::create_render_target_with_format(
            device,
            width,
            height,
            label,
            wgpu::TextureFormat::Bgra8Unorm,
        )
    }
    
    /// Create a render target texture with specific format
    pub fn create_render_target_with_format(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
        format: wgpu::TextureFormat,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING 
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }
    
    /// Update texture data (for video frames)
    pub fn update(&self, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width * 4),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
    
    /// Clear texture to black
    pub fn clear_to_black(&self, queue: &wgpu::Queue) {
        let black = vec![0u8; (self.width * self.height * 4) as usize];
        self.update(queue, &black);
    }
}

/// Input texture manager for multiple video sources
pub struct InputTextureManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pub input1: Option<Texture>,
    pub input2: Option<Texture>,
    input1_has_data: bool,
    input2_has_data: bool,
}

use std::sync::Arc;

impl InputTextureManager {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            input1: None,
            input2: None,
            input1_has_data: false,
            input2_has_data: false,
        }
    }
    
    /// Initialize or resize input 1 texture
    pub fn ensure_input1(&mut self, width: u32, height: u32) {
        match &self.input1 {
            Some(tex) if tex.width == width && tex.height == height => {
                // Size matches, nothing to do
            }
            _ => {
                log::info!("Creating input 1 texture: {}x{}", width, height);
                self.input1 = Some(Texture::from_rgba(
                    &self.device,
                    &self.queue,
                    width,
                    height,
                    "Input 1 Texture",
                    &vec![0u8; (width * height * 4) as usize],
                ));
            }
        }
    }
    
    /// Initialize or resize input 2 texture
    pub fn ensure_input2(&mut self, width: u32, height: u32) {
        match &self.input2 {
            Some(tex) if tex.width == width && tex.height == height => {
                // Size matches, nothing to do
            }
            _ => {
                log::info!("Creating input 2 texture: {}x{}", width, height);
                self.input2 = Some(Texture::from_rgba(
                    &self.device,
                    &self.queue,
                    width,
                    height,
                    "Input 2 Texture",
                    &vec![0u8; (width * height * 4) as usize],
                ));
            }
        }
    }
    
    /// Update input 1 with new frame data
    pub fn update_input1(&mut self, data: &[u8], width: u32, height: u32) {
        self.ensure_input1(width, height);
        if let Some(ref tex) = self.input1 {
            tex.update(&self.queue, data);
            self.input1_has_data = true;
        }
    }
    
    /// Update input 2 with new frame data
    pub fn update_input2(&mut self, data: &[u8], width: u32, height: u32) {
        self.ensure_input2(width, height);
        if let Some(ref tex) = self.input2 {
            tex.update(&self.queue, data);
            self.input2_has_data = true;
        }
    }
    
    /// Update input 1 from a wgpu texture (GPU-to-GPU copy, zero-copy CPU-wise)
    /// 
    /// This is the most efficient path for Syphon input - copies directly from
    /// the received Syphon texture to the input texture without CPU readback
    pub fn update_input1_from_texture(&mut self, source: &wgpu::Texture) {
        let width = source.width();
        let height = source.height();
        
        // Ensure our input texture exists with correct size
        self.ensure_input1(width, height);
        
        if let Some(ref dest) = self.input1 {
            // Create a command encoder for the copy
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Input1 Texture Copy"),
            });
            
            // GPU-to-GPU copy (zero CPU involvement)
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: source,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &dest.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            
            // Submit the copy command
            self.queue.submit(std::iter::once(encoder.finish()));
            self.input1_has_data = true;
        }
    }
    
    /// Update input 2 from a wgpu texture (GPU-to-GPU copy, zero-copy CPU-wise)
    pub fn update_input2_from_texture(&mut self, source: &wgpu::Texture) {
        let width = source.width();
        let height = source.height();
        
        self.ensure_input2(width, height);
        
        if let Some(ref dest) = self.input2 {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Input2 Texture Copy"),
            });
            
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: source,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &dest.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            
            self.queue.submit(std::iter::once(encoder.finish()));
            self.input2_has_data = true;
        }
    }
    
    /// Get input 1 texture view (or placeholder)
    pub fn get_input1_view(&self) -> &wgpu::TextureView {
        self.input1.as_ref()
            .map(|t| &t.view)
            .expect("Input 1 not initialized")
    }
    
    /// Get input 2 texture view (or placeholder)
    pub fn get_input2_view(&self) -> &wgpu::TextureView {
        self.input2.as_ref()
            .map(|t| &t.view)
            .expect("Input 2 not initialized")
    }
    
    /// Check if input 1 has received data
    pub fn input1_has_data(&self) -> bool {
        self.input1_has_data
    }
    
    /// Check if input 2 has received data
    pub fn input2_has_data(&self) -> bool {
        self.input2_has_data
    }
    
    /// Get input 1 resolution
    pub fn get_input1_resolution(&self) -> (u32, u32) {
        self.input1.as_ref()
            .map(|t| (t.width, t.height))
            .unwrap_or((1920, 1080))
    }
    
    /// Get input 2 resolution
    pub fn get_input2_resolution(&self) -> (u32, u32) {
        self.input2.as_ref()
            .map(|t| (t.width, t.height))
            .unwrap_or((1920, 1080))
    }
}
