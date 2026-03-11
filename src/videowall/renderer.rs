//! # Video Wall Runtime Renderer
//!
//! GPU-accelerated renderer for video wall output. Manages the multi-quad
//! shader pipeline and uniform buffers for display configuration.
//!
//! ## Architecture
//!
//! The renderer uses a single-pass approach where the fragment shader
//! checks which display quad each output pixel belongs to and samples
//! the source texture accordingly.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rusty_mapper::videowall::{VideoWallRenderer, VideoWallConfig};
//!
//! let mut renderer = VideoWallRenderer::new(&device, &queue, surface_format);
//! renderer.update_config(&config);
//!
//! // In render loop
//! renderer.render(&mut encoder, &source_texture_view, &output_view);
//! ```

use super::{DisplayQuad, VideoWallConfig};
use bytemuck::{Pod, Zeroable};
use glam::Vec4;

/// Maximum number of displays supported by the shader
pub const MAX_DISPLAYS: usize = 16;

/// GPU uniform data for a display quad
/// Must match the WGSL struct exactly (16-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DisplayQuadUniform {
    /// Source rectangle (x, y, width, height)
    pub source_rect: [f32; 4],
    /// Destination corners: TL, TR, BR, BL
    pub dest_tl: [f32; 2],
    pub dest_tr: [f32; 2],
    pub dest_br: [f32; 2],
    pub dest_bl: [f32; 2],
    /// Color adjustments (must match WGSL struct)
    pub brightness: f32,    // Multiplier (0.0 - 2.0)
    pub contrast: f32,      // Multiplier (0.0 - 2.0)
    pub gamma: f32,         // Exponent (0.1 - 3.0)
    /// Enabled flag
    pub enabled: u32,
}

impl DisplayQuadUniform {
    /// Create from a DisplayQuad with color adjustments
    pub fn from_quad(
        quad: &DisplayQuad,
        enabled: bool,
        brightness: f32,
        contrast: f32,
        gamma: f32,
    ) -> Self {
        Self {
            source_rect: [
                quad.source_rect.x,
                quad.source_rect.y,
                quad.source_rect.width,
                quad.source_rect.height,
            ],
            dest_tl: [quad.dest_corners[0].x, quad.dest_corners[0].y],
            dest_tr: [quad.dest_corners[1].x, quad.dest_corners[1].y],
            dest_br: [quad.dest_corners[2].x, quad.dest_corners[2].y],
            dest_bl: [quad.dest_corners[3].x, quad.dest_corners[3].y],
            brightness: brightness.clamp(0.0, 2.0),
            contrast: contrast.clamp(0.0, 2.0),
            gamma: gamma.clamp(0.1, 3.0),
            enabled: if enabled { 1 } else { 0 },
        }
    }

    /// Create from DisplayConfig (includes color adjustments)
    pub fn from_config(config: &super::DisplayConfig, quad: &DisplayQuad) -> Self {
        Self::from_quad(
            quad,
            config.enabled,
            config.brightness,
            config.contrast,
            config.gamma,
        )
    }

    /// Create a disabled placeholder
    pub fn disabled() -> Self {
        Self {
            source_rect: [0.0; 4],
            dest_tl: [0.0; 2],
            dest_tr: [0.0; 2],
            dest_br: [0.0; 2],
            dest_bl: [0.0; 2],
            brightness: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            enabled: 0,
        }
    }
}

impl Default for DisplayQuadUniform {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Video wall uniform data updated per frame
/// Must match the WGSL struct layout (16-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VideoWallUniforms {
    /// Number of active displays
    pub display_count: u32,
    /// Output resolution
    pub output_width: f32,
    pub output_height: f32,
    /// Padding to align background_color to 16 bytes
    _padding: u32,
    /// Background color (RGBA)
    pub background_color: [f32; 4],
}

impl Default for VideoWallUniforms {
    fn default() -> Self {
        Self {
            display_count: 0,
            output_width: 1920.0,
            output_height: 1080.0,
            _padding: 0,
            background_color: [0.0, 0.0, 0.0, 1.0], // Black
        }
    }
}

/// Video wall renderer
pub struct VideoWallRenderer {
    /// Render pipeline
    pipeline: wgpu::RenderPipeline,
    /// Bind group layout for uniforms
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    /// Current uniform bind group
    uniform_bind_group: Option<wgpu::BindGroup>,
    /// Uniform buffer for video wall settings
    uniforms_buffer: wgpu::Buffer,
    /// Storage buffer for display quads
    displays_buffer: wgpu::Buffer,
    /// Current configuration
    config: Option<VideoWallConfig>,
    /// Output resolution
    output_resolution: (u32, u32),
    /// Background color
    background_color: [f32; 4],
}

impl VideoWallRenderer {
    /// Create a new video wall renderer
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Video Wall Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create bind group layout for source texture
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video Wall Texture Bind Group Layout"),
            entries: &[
                // Source texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Create bind group layout for uniforms
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video Wall Uniform Bind Group Layout"),
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Display quads storage buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Video Wall Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Video Wall Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Video Wall Uniforms Buffer"),
            size: std::mem::size_of::<VideoWallUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create display quads storage buffer
        let displays_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Video Wall Displays Buffer"),
            size: (std::mem::size_of::<DisplayQuadUniform>() * MAX_DISPLAYS) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
            uniform_bind_group: None,
            uniforms_buffer,
            displays_buffer,
            config: None,
            output_resolution: (1920, 1080),
            background_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// Update the video wall configuration
    pub fn update_config(&mut self, config: &VideoWallConfig, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.config = Some(config.clone());

        // Build uniform data for displays with color adjustments
        let mut display_uniforms: Vec<DisplayQuadUniform> = config
            .displays
            .iter()
            .map(|d| DisplayQuadUniform::from_config(
                d,
                &DisplayQuad {
                    display_id: d.id,
                    grid_position: d.grid_position,
                    source_rect: d.source_uv,
                    dest_corners: d.dest_corners_vec2(),
                    perspective_matrix: None,
                },
            ))
            .collect();

        // Pad to MAX_DISPLAYS
        while display_uniforms.len() < MAX_DISPLAYS {
            display_uniforms.push(DisplayQuadUniform::disabled());
        }

        // Update displays buffer
        queue.write_buffer(
            &self.displays_buffer,
            0,
            bytemuck::cast_slice(&display_uniforms),
        );

        // Update uniforms
        let uniforms = VideoWallUniforms {
            display_count: config.displays.len() as u32,
            output_width: self.output_resolution.0 as f32,
            output_height: self.output_resolution.1 as f32,
            _padding: 0,
            background_color: self.background_color,
        };

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Create bind group
        self.uniform_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Wall Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.displays_buffer.as_entire_binding(),
                },
            ],
        }));

        log::info!("Video wall config updated: {} displays", config.displays.len());
    }

    /// Set output resolution
    pub fn set_output_resolution(&mut self, width: u32, height: u32) {
        self.output_resolution = (width, height);
    }
    
    /// Update uniforms buffer with current config and output resolution
    fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        if let Some(ref config) = self.config {
            let uniforms = VideoWallUniforms {
                display_count: config.displays.len() as u32,
                output_width: self.output_resolution.0 as f32,
                output_height: self.output_resolution.1 as f32,
                _padding: 0,
                background_color: self.background_color,
            };
            queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
        }
    }

    /// Set background color
    pub fn set_background_color(&mut self, color: [f32; 4]) {
        self.background_color = color;
    }

    /// Check if a configuration is loaded
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Render the video wall
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source_texture_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_width: u32,
        output_height: u32,
    ) {
        // Update output resolution and uniforms if changed
        if self.output_resolution != (output_width, output_height) {
            self.output_resolution = (output_width, output_height);
            self.update_uniforms(queue);
        }
        // Create texture bind group for this frame
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Video Wall Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Wall Texture Bind Group"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Video Wall Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: self.background_color[0] as f64,
                        g: self.background_color[1] as f64,
                        b: self.background_color[2] as f64,
                        a: self.background_color[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &texture_bind_group, &[]);
        
        if let Some(ref bind_group) = self.uniform_bind_group {
            render_pass.set_bind_group(1, bind_group, &[]);
        }
        
        // Draw fullscreen triangle (3 vertices for triangle strip covering screen)
        render_pass.draw(0..3, 0..1);
    }

    /// Get current configuration
    pub fn config(&self) -> Option<&VideoWallConfig> {
        self.config.as_ref()
    }

    /// Get output resolution
    pub fn output_resolution(&self) -> (u32, u32) {
        self.output_resolution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_quad_uniform_from_quad() {
        let quad = DisplayQuad {
            display_id: 0,
            grid_position: (0, 0),
            source_rect: super::super::Rect::new(0.0, 0.0, 0.5, 0.5),
            dest_corners: [
                glam::Vec2::new(0.1, 0.1),
                glam::Vec2::new(0.9, 0.1),
                glam::Vec2::new(0.9, 0.9),
                glam::Vec2::new(0.1, 0.9),
            ],
            perspective_matrix: None,
        };

        let uniform = DisplayQuadUniform::from_quad(&quad, true, 1.0, 1.0, 1.0);

        assert_eq!(uniform.enabled, 1);
        assert!((uniform.source_rect[0] - 0.0).abs() < 0.001);
        assert!((uniform.source_rect[2] - 0.5).abs() < 0.001);
        assert!((uniform.dest_tl[0] - 0.1).abs() < 0.001);
        assert!((uniform.brightness - 1.0).abs() < 0.001);
        assert!((uniform.contrast - 1.0).abs() < 0.001);
        assert!((uniform.gamma - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_display_quad_uniform_disabled() {
        let uniform = DisplayQuadUniform::disabled();
        assert_eq!(uniform.enabled, 0);
        assert_eq!(uniform.brightness, 1.0);
        assert_eq!(uniform.contrast, 1.0);
        assert_eq!(uniform.gamma, 1.0);
    }

    #[test]
    fn test_video_wall_uniforms_default() {
        let uniforms = VideoWallUniforms::default();
        assert_eq!(uniforms.display_count, 0);
        assert_eq!(uniforms.background_color, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_max_displays_constant() {
        assert_eq!(MAX_DISPLAYS, 16);
    }
}
