//! # Video Wall Render Example
//!
//! This example demonstrates the video wall runtime renderer by creating
//! a test pattern and rendering it through a video wall configuration.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example videowall_render --no-default-features
//! ```
//!
//! Press '1', '2', '3', '4' to toggle individual displays.
//! Press 'R' to reset the configuration.
//! Press 'F' to toggle fullscreen.
//! Press 'ESC' to exit.

use rusty_mapper::videowall::{
    DisplayConfig, DisplayQuad, GridSize, Rect, VideoWallConfig, VideoWallRenderer,
};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("Video Wall Render Example");
    println!("=========================\n");
    println!("Controls:");
    println!("  1-4 - Toggle displays");
    println!("  R - Reset config");
    println!("  F - Toggle fullscreen");
    println!("  ESC - Exit\n");
    
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    renderer: Option<VideoWallRenderer>,
    config: VideoWallConfig,
    test_pattern: Option<wgpu::Texture>,
}

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl App {
    fn new() -> Self {
        let config = create_test_config();
        
        Self {
            window: None,
            gpu: None,
            renderer: None,
            config,
            test_pattern: None,
        }
    }
    
    fn create_test_pattern(&mut self) {
        let gpu = self.gpu.as_ref().unwrap();
        let device = &gpu.device;
        let queue = &gpu.queue;
        
        // Create a colorful test pattern
        let width = 1920u32;
        let height = 1080u32;
        
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Test Pattern"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Generate colorful test pattern
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                // Create a gradient pattern
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128u8;
                let a = 255u8;
                data.extend_from_slice(&[r, g, b, a]);
            }
        }
        
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        
        self.test_pattern = Some(texture);
    }
    
    fn toggle_display(&mut self, id: u32) {
        if let Some(display) = self.config.displays.iter_mut().find(|d| d.id == id) {
            display.enabled = !display.enabled;
            println!("Display {}: {}", id + 1, if display.enabled { "ON" } else { "OFF" });
        }
        self.update_renderer_config();
    }
    
    fn reset_config(&mut self) {
        self.config = create_test_config();
        println!("Configuration reset");
        self.update_renderer_config();
    }
    
    fn update_renderer_config(&mut self) {
        if let (Some(ref mut renderer), Some(ref gpu)) = (self.renderer.as_mut(), self.gpu.as_ref()) {
            renderer.update_config(&self.config, &gpu.device, &gpu.queue);
        }
    }
    
    fn render(&mut self) {
        let Some(ref gpu) = self.gpu else { return };
        let Some(ref renderer) = self.renderer else { return };
        let Some(ref pattern) = self.test_pattern else { return };
        
        let output = match gpu.surface.get_current_texture() {
            Ok(output) => output,
            Err(_) => return,
        };
        
        let pattern_view = pattern.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Video Wall Render Encoder"),
        });
        
        renderer.render(&mut encoder, &pattern_view, &output_view, &gpu.device);
        
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
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
        let window = Arc::new(event_loop.create_window(WindowAttributes::default()
            .with_title("Video Wall Render Example")
            .with_inner_size(winit::dpi::LogicalSize::new(1920u32, 1080u32))
            .with_resizable(true)
        ).unwrap());
        
        self.window = Some(Arc::clone(&window));
        
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(window).unwrap();
        
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("Failed to find adapter");
        
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Video Wall Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        )).expect("Failed to create device");
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: 1920,
            height: 1080,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        
        surface.configure(&device, &config);
        
        self.gpu = Some(GpuState { surface, device, queue, config });
        
        // Create renderer
        let gpu = self.gpu.as_ref().unwrap();
        let mut renderer = VideoWallRenderer::new(&gpu.device, &gpu.queue, surface_format);
        renderer.update_config(&self.config, &gpu.device, &gpu.queue);
        self.renderer = Some(renderer);
        
        // Create test pattern
        self.create_test_pattern();
    }
    
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == winit::event::ElementState::Pressed {
                    match event.logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                            event_loop.exit();
                        }
                        winit::keyboard::Key::Character(ch) => {
                            match ch.as_str() {
                                "1" => self.toggle_display(0),
                                "2" => self.toggle_display(1),
                                "3" => self.toggle_display(2),
                                "4" => self.toggle_display(3),
                                "r" | "R" => self.reset_config(),
                                "f" | "F" => self.toggle_fullscreen(),
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(ref mut gpu) = self.gpu {
                    gpu.config.width = size.width;
                    gpu.config.height = size.height;
                    gpu.surface.configure(&gpu.device, &gpu.config);
                }
            }
            WindowEvent::RedrawRequested => {
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

fn create_test_config() -> VideoWallConfig {
    use rusty_mapper::videowall::CalibrationInfo;
    use glam::Vec2;
    
    let quads = vec![
        // Top-left (Display 0)
        DisplayQuad {
            display_id: 0,
            grid_position: (0, 0),
            source_rect: Rect::new(0.0, 0.0, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.05, 0.05),
                Vec2::new(0.45, 0.05),
                Vec2::new(0.45, 0.45),
                Vec2::new(0.05, 0.45),
            ],
            perspective_matrix: None,
        },
        // Top-right (Display 1)
        DisplayQuad {
            display_id: 1,
            grid_position: (1, 0),
            source_rect: Rect::new(0.5, 0.0, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.55, 0.05),
                Vec2::new(0.95, 0.05),
                Vec2::new(0.95, 0.45),
                Vec2::new(0.55, 0.45),
            ],
            perspective_matrix: None,
        },
        // Bottom-left (Display 2)
        DisplayQuad {
            display_id: 2,
            grid_position: (0, 1),
            source_rect: Rect::new(0.0, 0.5, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.05, 0.55),
                Vec2::new(0.45, 0.55),
                Vec2::new(0.45, 0.95),
                Vec2::new(0.05, 0.95),
            ],
            perspective_matrix: None,
        },
        // Bottom-right (Display 3)
        DisplayQuad {
            display_id: 3,
            grid_position: (1, 1),
            source_rect: Rect::new(0.5, 0.5, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.55, 0.55),
                Vec2::new(0.95, 0.95),
                Vec2::new(0.95, 0.95),
                Vec2::new(0.55, 0.95),
            ],
            perspective_matrix: None,
        },
    ];
    
    VideoWallConfig::from_quads(
        quads,
        GridSize::two_by_two(),
        (1920, 1080),
        CalibrationInfo::default(),
    )
}
