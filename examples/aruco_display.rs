//! # ArUco Pattern Display Example
//!
//! This example generates ArUco calibration patterns and displays them
//! on the output window. Useful for testing video wall calibration.
//!
//! ## Usage
//!
//! ```bash
//! # Without OpenCV (uses fallback pattern generation)
//! cargo run --example aruco_display --no-default-features
//!
//! # With OpenCV (proper ArUco markers)
//! cargo run --example aruco_display --features opencv
//!
//! # Specify custom grid size
//! cargo run --example aruco_display -- --grid 3x3
//! ```

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use rusty_mapper::videowall::{ArUcoGenerator, ArUcoDictionary, GridSize};

/// Grid size from command line args
struct Args {
    grid_columns: u32,
    grid_rows: u32,
    output_width: u32,
    output_height: u32,
}

impl Args {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        
        let mut grid = (2u32, 2u32);
        let mut resolution = (1920u32, 1080u32);
        
        for i in 1..args.len() {
            match args[i].as_str() {
                "--grid" => {
                    if i + 1 < args.len() {
                        let parts: Vec<&str> = args[i + 1].split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(cols), Ok(rows)) = (parts[0].parse(), parts[1].parse()) {
                                grid = (cols, rows);
                            }
                        }
                    }
                }
                "--resolution" => {
                    if i + 1 < args.len() {
                        let parts: Vec<&str> = args[i + 1].split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                                resolution = (w, h);
                            }
                        }
                    }
                }
                "--help" | "-h" => {
                    println!("ArUco Pattern Display Example\n");
                    println!("Usage: cargo run --example aruco_display -- [OPTIONS]\n");
                    println!("Options:");
                    println!("  --grid <COLSxROWS>      Grid size (default: 2x2)");
                    println!("  --resolution <WxH>      Output resolution (default: 1920x1080)");
                    println!("  --help, -h              Show this help\n");
                    println!("Examples:");
                    println!("  cargo run --example aruco_display -- --grid 3x3");
                    println!("  cargo run --example aruco_display -- --grid 4x4 --resolution 3840x2160");
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        
        Self {
            grid_columns: grid.0,
            grid_rows: grid.1,
            output_width: resolution.0,
            output_height: resolution.1,
        }
    }
}

/// GPU resources managed separately to avoid borrow issues
struct GpuResources {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pattern_texture: Option<wgpu::Texture>,
    pattern_bind_group: Option<wgpu::BindGroup>,
    render_pipeline: wgpu::RenderPipeline,
}

/// Main application state
struct App {
    args: Args,
    
    // Window
    window: Option<Arc<Window>>,
    
    // GPU resources
    gpu: Option<GpuResources>,
    
    // Pattern generation
    generator: ArUcoGenerator,
    patterns: Vec<image::RgbaImage>,
    current_pattern: usize,
    
    // Timing for pattern cycling
    last_switch: std::time::Instant,
    switch_interval: std::time::Duration,
    auto_cycle: bool,
}

impl App {
    fn new(args: Args) -> Self {
        // Select appropriate dictionary for grid size
        let dictionary = ArUcoDictionary::for_grid_size(args.grid_columns, args.grid_rows);
        let generator = ArUcoGenerator::new(dictionary);
        
        // Generate all patterns
        let grid_size = (args.grid_columns, args.grid_rows);
        let resolution = (args.output_width, args.output_height);
        
        println!("Generating ArUco patterns...");
        println!("  Grid: {}x{}", args.grid_columns, args.grid_rows);
        println!("  Resolution: {}x{}", args.output_width, args.output_height);
        println!("  Dictionary: {:?}", dictionary);
        println!("  Total patterns: {}", args.grid_columns * args.grid_rows);
        
        let patterns = generator.generate_all_calibration_frames(grid_size, resolution)
            .expect("Failed to generate patterns");
        
        println!("Patterns generated successfully!");
        println!("Controls:");
        println!("  SPACE - Next pattern");
        println!("  P - Previous pattern");
        println!("  A - Auto-cycle (every 2 seconds)");
        println!("  S - Stop auto-cycle");
        println!("  F - Toggle fullscreen");
        println!("  ESC - Exit");
        
        Self {
            args,
            window: None,
            gpu: None,
            generator,
            patterns,
            current_pattern: 0,
            last_switch: std::time::Instant::now(),
            switch_interval: std::time::Duration::from_secs(2),
            auto_cycle: false,
        }
    }
    
    fn next_pattern(&mut self) {
        self.current_pattern = (self.current_pattern + 1) % self.patterns.len();
        self.upload_current_pattern();
    }
    
    fn prev_pattern(&mut self) {
        if self.current_pattern == 0 {
            self.current_pattern = self.patterns.len() - 1;
        } else {
            self.current_pattern -= 1;
        }
        self.upload_current_pattern();
    }
    
    fn upload_current_pattern(&mut self) {
        let Some(ref mut gpu) = self.gpu else { return };
        
        let pattern = &self.patterns[self.current_pattern];
        let width = pattern.width();
        let height = pattern.height();
        
        log::debug!("Uploading pattern {}: {}x{}", self.current_pattern, width, height);
        
        // Create or recreate texture if size changed
        let needs_new_texture = gpu.pattern_texture.as_ref().map_or(true, |t| {
            log::debug!("Existing texture: {}x{}, pattern: {}x{}", t.width(), t.height(), width, height);
            t.width() != width || t.height() != height
        });
        
        log::debug!("Needs new texture: {}", needs_new_texture);
        
        if needs_new_texture {
            let bind_group_layout = gpu.render_pipeline.get_bind_group_layout(0);
            
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Pattern Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            
            let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Pattern Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Pattern Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });
            
            gpu.pattern_texture = Some(texture);
            gpu.pattern_bind_group = Some(bind_group);
        }
        
        // Upload pattern data
        if let Some(ref texture) = gpu.pattern_texture {
            let texture_width = texture.width();
            let texture_height = texture.height();
            
            // Ensure pattern matches texture size
            if width != texture_width || height != texture_height {
                log::error!("Pattern size mismatch! Pattern: {}x{}, Texture: {}x{}", 
                    width, height, texture_width, texture_height);
                return;
            }
            
            let rgba_data: Vec<u8> = pattern.pixels().flat_map(|p| [p[0], p[1], p[2], p[3]]).collect();
            log::debug!("Uploading {} bytes ({}x{})", rgba_data.len(), width, height);
            
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    
    fn render(&mut self) {
        let Some(ref mut gpu) = self.gpu else { return };
        let Some(ref bind_group) = gpu.pattern_bind_group else { return };
        
        let output = match gpu.surface.get_current_texture() {
            Ok(output) => output,
            Err(_) => return,
        };
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Pattern Render Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Pattern Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(&gpu.render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
        
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        // Update window title with current pattern
        if let Some(ref window) = self.window {
            let display_id = self.current_pattern;
            let grid_pos = GridSize::new(self.args.grid_columns, self.args.grid_rows)
                .position_from_id(display_id as u32);
            window.set_title(&format!(
                "ArUco Pattern - Display {} (Col {}, Row {}) - {} of {}",
                display_id + 1,
                grid_pos.0,
                grid_pos.1,
                self.current_pattern + 1,
                self.patterns.len()
            ));
        }
    }
    
    fn toggle_fullscreen(&self) {
        if let Some(ref window) = self.window {
            let is_fullscreen = window.fullscreen().is_some();
            window.set_fullscreen(if is_fullscreen {
                None
            } else {
                Some(winit::window::Fullscreen::Borderless(None))
            });
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window = Arc::new(event_loop.create_window(WindowAttributes::default()
            .with_title("ArUco Pattern Display")
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.args.output_width,
                self.args.output_height,
            ))
            .with_resizable(true)
        ).unwrap());
        
        self.window = Some(Arc::clone(&window));
        
        // Get actual window size (may differ from requested due to HiDPI)
        let actual_size = window.inner_size();
        log::info!("Window created: requested {}x{}, actual {}x{}", 
            self.args.output_width, self.args.output_height,
            actual_size.width, actual_size.height);
        
        // Update args to match actual size
        self.args.output_width = actual_size.width;
        self.args.output_height = actual_size.height;
        
        // Regenerate patterns at actual resolution
        let grid_size = (self.args.grid_columns, self.args.grid_rows);
        let resolution = (self.args.output_width, self.args.output_height);
        println!("Regenerating {} patterns for actual resolution {}x{}...", 
            grid_size.0 * grid_size.1, resolution.0, resolution.1);
        self.patterns = self.generator.generate_all_calibration_frames(grid_size, resolution)
            .expect("Failed to generate patterns");
        println!("Regenerated {} patterns", self.patterns.len());
        self.current_pattern = 0;
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface
        let surface = instance.create_surface(window).unwrap();
        
        // Request adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("Failed to find suitable GPU adapter");
        
        // Create device and queue
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Pattern Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        )).expect("Failed to create device");
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: actual_size.width,
            height: actual_size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        
        surface.configure(&device, &config);
        
        // Create render pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Pattern Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("aruco_display_shader.wgsl").into()),
        });
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pattern Bind Group Layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pattern Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pattern Pipeline"),
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
        
        self.gpu = Some(GpuResources {
            surface,
            device,
            queue,
            config,
            pattern_texture: None,
            pattern_bind_group: None,
            render_pipeline,
        });
        
        // Upload initial pattern
        self.upload_current_pattern();
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    match event.logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                            self.next_pattern();
                        }
                        winit::keyboard::Key::Character(ch) => {
                            match ch.to_lowercase().as_str() {
                                "p" => self.prev_pattern(),
                                "n" => self.next_pattern(),
                                "a" => {
                                    println!("Auto-cycle enabled (2 second intervals)");
                                    self.auto_cycle = true;
                                }
                                "s" => {
                                    println!("Auto-cycle stopped");
                                    self.auto_cycle = false;
                                }
                                "f" => self.toggle_fullscreen(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::Resized(size) => {
                log::info!("Window resized to {}x{}", size.width, size.height);
                
                // Update GPU surface
                if let Some(ref mut gpu) = self.gpu {
                    gpu.config.width = size.width;
                    gpu.config.height = size.height;
                    gpu.surface.configure(&gpu.device, &gpu.config);
                }
                
                // Regenerate patterns at new size
                self.args.output_width = size.width;
                self.args.output_height = size.height;
                let grid_size = (self.args.grid_columns, self.args.grid_rows);
                let resolution = (size.width, size.height);
                
                println!("Regenerating {} patterns for {}x{}...", 
                    grid_size.0 * grid_size.1, size.width, size.height);
                
                match self.generator.generate_all_calibration_frames(grid_size, resolution) {
                    Ok(new_patterns) => {
                        println!("Regenerated {} patterns", new_patterns.len());
                        self.patterns = new_patterns;
                        self.current_pattern = self.current_pattern.min(self.patterns.len().saturating_sub(1));
                        self.upload_current_pattern();
                    }
                    Err(e) => {
                        log::error!("Failed to regenerate patterns: {}", e);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Check for auto-cycle
                if self.auto_cycle && self.last_switch.elapsed() >= self.switch_interval {
                    self.next_pattern();
                    self.last_switch = std::time::Instant::now();
                }
                
                self.render();
            }
            _ => {}
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let args = Args::parse();
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::new(args);
    event_loop.run_app(&mut app)?;
    
    Ok(())
}
