//! # Video Matrix Renderer
//!
//! GPU-accelerated renderer for grid-based video matrix mapping.
//! Renders input grid cells to output positions with aspect ratio and orientation handling.
//!
//! ## Architecture
//!
//! 1. **Input Grid Subdivision**: Input texture is divided into N×M cells
//! 2. **Cell Mapping**: Each cell is mapped to an output position
//! 3. **Orientation Handling**: UV coordinates are rotated based on detected orientation
//! 4. **Black Fill**: Unmapped output cells are filled with black
//!
//! ## Shader Strategy
//!
//! The fragment shader checks which output grid cell the pixel belongs to,
/// then samples from the corresponding input grid cell with orientation transform.

use super::{VideoMatrixConfig, GridCellMapping, GridSize, InputGridConfig};
use bytemuck::{Pod, Zeroable};

/// Maximum number of cell mappings supported
pub const MAX_MAPPINGS: usize = 16;

/// GPU uniform data for a cell mapping
/// Must match the WGSL struct exactly (16-byte aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CellMappingUniform {
    /// Source rectangle in input texture (x, y, width, height)
    pub source_rect: [f32; 4],
    /// Destination rectangle in output (x, y, width, height)
    pub dest_rect: [f32; 4],
    /// Orientation: 0=0°, 1=90°, 2=180°, 3=270°
    pub orientation: u32,
    /// Aspect ratio (width / height) for display sizing
    pub aspect_ratio: f32,
    /// Enabled flag (0 or 1)
    pub enabled: u32,
    /// Padding to align to 16 bytes
    pub _padding: u32,
}

impl CellMappingUniform {
    /// Create from a GridCellMapping
    pub fn from_mapping(mapping: &GridCellMapping, input_grid: &InputGridConfig, output_grid: &GridSize) -> Self {
        let source_rect = mapping.get_source_rect(input_grid.grid_size);
        let dest_rect = mapping.get_dest_rect(*output_grid);
        
        let orientation_idx = match mapping.orientation {
            super::Orientation::Normal => 0,
            super::Orientation::Rotated90 => 1,
            super::Orientation::Rotated180 => 2,
            super::Orientation::Rotated270 => 3,
        };
        
        Self {
            source_rect: [source_rect.x, source_rect.y, source_rect.width, source_rect.height],
            dest_rect: [dest_rect.x, dest_rect.y, dest_rect.width, dest_rect.height],
            orientation: orientation_idx,
            aspect_ratio: mapping.aspect_ratio.as_f32(),
            enabled: if mapping.enabled { 1 } else { 0 },
            _padding: 0,
        }
    }

    /// Create a disabled placeholder
    pub fn disabled() -> Self {
        Self {
            source_rect: [0.0; 4],
            dest_rect: [0.0; 4],
            orientation: 0,
            aspect_ratio: 1.0,
            enabled: 0,
            _padding: 0,
        }
    }
}

/// Video matrix uniform data updated per frame
/// Must match WGSL struct layout exactly (96 bytes total)
/// WGSL struct size is always rounded up to 16 bytes
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct MatrixUniforms {
    // First 16 bytes (0-15)
    pub mapping_count: u32,
    pub input_cols: u32,
    pub input_rows: u32,
    pub output_cols: u32,
    
    // Second 16 bytes (16-31)
    pub output_rows: u32,
    pub output_width: u32,   // Actual pixel width for UV calculation
    pub output_height: u32,  // Actual pixel height for UV calculation
    pub _padding1: u32,
    
    // Third 16 bytes (32-47)
    pub _padding2_0: u32,
    pub _padding2_1: u32,
    pub _padding2_2: u32,
    pub _align_pad0: u32,
    
    // Fourth 16 bytes (48-63) - background_color at 16-byte boundary
    pub background_color: [f32; 4],
    
    // Fifth 16 bytes (64-79)
    pub _final_pad0: u32,
    pub _final_pad1: u32,
    pub _final_pad2: u32,
    pub _final_pad3: u32,
    
    // Sixth 16 bytes (80-95) - ensure struct is 96 bytes
    pub _final_pad4: u32,
    pub _final_pad5: u32,
    pub _final_pad6: u32,
    pub _final_pad7: u32,
}

// Manual Pod impl since we have align(16)
unsafe impl bytemuck::Pod for MatrixUniforms {}

impl Default for MatrixUniforms {
    fn default() -> Self {
        Self {
            mapping_count: 0,
            input_cols: 3,
            input_rows: 3,
            output_cols: 3,
            output_rows: 3,
            output_width: 1920,
            output_height: 1080,
            _padding1: 0,
            _padding2_0: 0,
            _padding2_1: 0,
            _padding2_2: 0,
            _align_pad0: 0,
            background_color: [0.0, 0.0, 0.0, 1.0], // Black
            _final_pad0: 0,
            _final_pad1: 0,
            _final_pad2: 0,
            _final_pad3: 0,
            _final_pad4: 0,
            _final_pad5: 0,
            _final_pad6: 0,
            _final_pad7: 0,
        }
    }
}

/// Video matrix renderer
pub struct VideoMatrixRenderer {
    /// Render pipeline
    pipeline: wgpu::RenderPipeline,
    /// Bind group layout for uniforms
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    /// Current uniform bind group
    uniform_bind_group: Option<wgpu::BindGroup>,
    /// Uniform buffer for matrix settings
    uniforms_buffer: wgpu::Buffer,
    /// Storage buffer for cell mappings
    mappings_buffer: wgpu::Buffer,
    /// Current configuration
    config: Option<VideoMatrixConfig>,
    /// Output resolution
    output_resolution: (u32, u32),
}

impl VideoMatrixRenderer {
    /// Create a new video matrix renderer
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Video Matrix Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("matrix_shader.wgsl").into()),
        });

        // Create bind group layout for source texture
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Video Matrix Texture Bind Group Layout"),
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
            label: Some("Video Matrix Uniform Bind Group Layout"),
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
                // Cell mappings storage buffer
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
            label: Some("Video Matrix Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Video Matrix Pipeline"),
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
            label: Some("Video Matrix Uniforms Buffer"),
            size: std::mem::size_of::<MatrixUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create cell mappings storage buffer
        let mappings_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Video Matrix Mappings Buffer"),
            size: (std::mem::size_of::<CellMappingUniform>() * MAX_MAPPINGS) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_bind_group_layout,
            uniform_bind_group: None,
            uniforms_buffer,
            mappings_buffer,
            config: None,
            output_resolution: (1920, 1080),
        }
    }

    /// Update the video matrix configuration
    pub fn update_config(&mut self, config: &VideoMatrixConfig, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.config = Some(config.clone());

        // Build uniform data for cell mappings
        let mut mapping_uniforms: Vec<CellMappingUniform> = config
            .input_grid
            .mappings
            .iter()
            .filter(|m| m.enabled)
            .take(MAX_MAPPINGS)
            .map(|m| {
                let uniform = CellMappingUniform::from_mapping(m, &config.input_grid, &config.output_grid);
                // Log source rect for debugging
                if m.custom_source_rect.is_some() {
                    log::info!(
                        "Mapping {}: source rect [{:.3}, {:.3}, {:.3}, {:.3}] (custom)",
                        m.input_cell, uniform.source_rect[0], uniform.source_rect[1],
                        uniform.source_rect[2], uniform.source_rect[3]
                    );
                }
                uniform
            })
            .collect();

        // Pad to MAX_MAPPINGS
        while mapping_uniforms.len() < MAX_MAPPINGS {
            mapping_uniforms.push(CellMappingUniform::disabled());
        }

        // Update mappings buffer
        queue.write_buffer(
            &self.mappings_buffer,
            0,
            bytemuck::cast_slice(&mapping_uniforms),
        );

        // Update uniforms
        let uniforms = MatrixUniforms {
            mapping_count: config.input_grid.mappings.iter().filter(|m| m.enabled).count() as u32,
            input_cols: config.input_grid.grid_size.columns,
            input_rows: config.input_grid.grid_size.rows,
            output_cols: config.output_grid.columns,
            output_rows: config.output_grid.rows,
            output_width: self.output_resolution.0,
            output_height: self.output_resolution.1,
            _padding1: 0,
            _padding2_0: 0,
            _padding2_1: 0,
            _padding2_2: 0,
            _align_pad0: 0,
            background_color: config.background_color,
            _final_pad0: 0,
            _final_pad1: 0,
            _final_pad2: 0,
            _final_pad3: 0,
            _final_pad4: 0,
            _final_pad5: 0,
            _final_pad6: 0,
            _final_pad7: 0,
        };

        queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Create bind group
        self.uniform_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Matrix Uniform Bind Group"),
            layout: &self.uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniforms_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.mappings_buffer.as_entire_binding(),
                },
            ],
        }));

        log::debug!("Video matrix config updated: {} mappings", uniforms.mapping_count);
    }

    /// Set output resolution
    pub fn set_output_resolution(&mut self, width: u32, height: u32) {
        self.output_resolution = (width, height);
    }
    
    /// Update uniforms buffer with current config and output resolution
    fn update_uniforms(&mut self, queue: &wgpu::Queue) {
        if let Some(ref config) = self.config {
            let uniforms = MatrixUniforms {
                mapping_count: config.input_grid.mappings.iter().filter(|m| m.enabled).count() as u32,
                input_cols: config.input_grid.grid_size.columns,
                input_rows: config.input_grid.grid_size.rows,
                output_cols: config.output_grid.columns,
                output_rows: config.output_grid.rows,
                output_width: self.output_resolution.0,
                output_height: self.output_resolution.1,
                _padding1: 0,
                _padding2_0: 0,
                _padding2_1: 0,
                _padding2_2: 0,
                _align_pad0: 0,
                background_color: config.background_color,
                _final_pad0: 0,
                _final_pad1: 0,
                _final_pad2: 0,
                _final_pad3: 0,
                _final_pad4: 0,
                _final_pad5: 0,
                _final_pad6: 0,
                _final_pad7: 0,
            };
            queue.write_buffer(&self.uniforms_buffer, 0, bytemuck::bytes_of(&uniforms));
        }
    }

    /// Check if a configuration is loaded
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }
    
    /// Check if uniform bind group is ready
    pub fn has_uniform_bind_group(&self) -> bool {
        self.uniform_bind_group.is_some()
    }

    /// Get current configuration
    pub fn config(&self) -> Option<&VideoMatrixConfig> {
        self.config.as_ref()
    }

    /// Render the video matrix
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
            label: Some("Video Matrix Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Video Matrix Texture Bind Group"),
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

        // Get background color from config
        let bg_color = self.config.as_ref()
            .map(|c| c.background_color)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);

        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Video Matrix Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg_color[0] as f64,
                        g: bg_color[1] as f64,
                        b: bg_color[2] as f64,
                        a: bg_color[3] as f64,
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
        
        let has_uniforms = self.uniform_bind_group.is_some();
        let mapping_count = self.config.as_ref().map(|c| c.input_grid.mappings.len()).unwrap_or(0);
        
        if let Some(ref bind_group) = self.uniform_bind_group {
            render_pass.set_bind_group(1, bind_group, &[]);
        } else {
            log::warn!("Video matrix: no uniform bind group! Config has {} mappings", mapping_count);
        }
        
        // Draw fullscreen triangle (3 vertices for triangle strip covering screen)
        render_pass.draw(0..3, 0..1);
        
        log::debug!("Video matrix rendered: has_uniforms={}, mappings={}, output={}x{}", 
            has_uniforms, mapping_count, output_width, output_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{GridCellMapping, GridPosition, AspectRatio, Orientation, GridSize};

    #[test]
    fn test_cell_mapping_uniform() {
        let mapping = GridCellMapping::new(0, GridPosition::new(0.0, 0.0, 1.0, 1.0))
            .with_aspect_ratio(AspectRatio::Ratio4_3)
            .with_orientation(Orientation::Rotated90);
        
        let input_grid = InputGridConfig::new(GridSize::new(3, 3));
        let output_grid = GridSize::new(3, 3);
        
        let uniform = CellMappingUniform::from_mapping(&mapping, &input_grid, &output_grid);
        
        assert!(uniform.enabled == 1);
        assert!(uniform.orientation == 1); // 90°
        assert!((uniform.aspect_ratio - 1.333).abs() < 0.01);
        
        // Source rect should be top-left cell of 3x3 grid
        assert!((uniform.source_rect[0] - 0.0).abs() < 0.01); // x
        assert!((uniform.source_rect[1] - 0.0).abs() < 0.01); // y
        assert!((uniform.source_rect[2] - 0.333).abs() < 0.01); // width
        assert!((uniform.source_rect[3] - 0.333).abs() < 0.01); // height
    }

    #[test]
    fn test_matrix_uniforms_default() {
        let uniforms = MatrixUniforms::default();
        assert_eq!(uniforms.mapping_count, 0);
        assert_eq!(uniforms.input_cols, 3);
        assert_eq!(uniforms.input_rows, 3);
        assert_eq!(uniforms.output_width, 1920);
        assert_eq!(uniforms.output_height, 1080);
        assert_eq!(uniforms.background_color, [0.0, 0.0, 0.0, 1.0]);
        // Verify struct size is exactly 96 bytes (6 * 16-byte chunks)
        assert_eq!(std::mem::size_of::<MatrixUniforms>(), 96);
    }

    #[test]
    fn test_max_mappings_constant() {
        assert_eq!(MAX_MAPPINGS, 16);
    }
}
