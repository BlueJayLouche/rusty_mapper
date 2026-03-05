//! # Application Handler
//!
//! Dual-window application handler implementing winit's ApplicationHandler.
//! 
//! Manages:
//! - Output window: Fullscreen-capable, hidden cursor
//! - Control window: ImGui-based UI
//! - Shared wgpu resources between windows

use crate::config::AppConfig;
use crate::core::{SharedState, NdiOutputCommand, InputChangeRequest};
use crate::engine::WgpuEngine;
use crate::gui::{ControlGui, ImGuiRenderer};
use crate::input::InputManager;
use crate::ndi::NdiOutputSender;

use anyhow::Result;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// Run the application
pub fn run_app(
    config: AppConfig,
    shared_state: Arc<std::sync::Mutex<SharedState>>,
) -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::new(config, shared_state);
    event_loop.run_app(&mut app)?;
    
    Ok(())
}

/// Main application state
struct App {
    config: AppConfig,
    shared_state: Arc<std::sync::Mutex<SharedState>>,
    
    // Shared wgpu resources
    wgpu_instance: Option<wgpu::Instance>,
    wgpu_adapter: Option<wgpu::Adapter>,
    wgpu_device: Option<Arc<wgpu::Device>>,
    wgpu_queue: Option<Arc<wgpu::Queue>>,
    
    // Output window
    output_window: Option<Arc<Window>>,
    output_engine: Option<WgpuEngine>,
    
    // Control window
    control_window: Option<Arc<Window>>,
    control_gui: Option<ControlGui>,
    imgui_renderer: Option<ImGuiRenderer>,
    
    // Input manager (handles webcam, NDI, OBS)
    input_manager: Option<InputManager>,
    
    // NDI output
    ndi_output: Option<NdiOutputSender>,
    
    // Modifier state
    shift_pressed: bool,
}

impl App {
    fn new(config: AppConfig, shared_state: Arc<std::sync::Mutex<SharedState>>) -> Self {
        Self {
            config,
            shared_state,
            wgpu_instance: None,
            wgpu_adapter: None,
            wgpu_device: None,
            wgpu_queue: None,
            output_window: None,
            output_engine: None,
            control_window: None,
            control_gui: None,
            imgui_renderer: None,
            input_manager: None,
            ndi_output: None,
            shift_pressed: false,
        }
    }
    
    /// Toggle fullscreen on output window
    fn toggle_fullscreen(&mut self) {
        if let Some(ref output_window) = self.output_window {
            let mut state = self.shared_state.lock().unwrap();
            state.toggle_fullscreen();
            
            let fullscreen_mode = if state.output_fullscreen {
                Some(winit::window::Fullscreen::Borderless(None))
            } else {
                None
            };
            
            output_window.set_fullscreen(fullscreen_mode);
            log::info!("Fullscreen: {}", state.output_fullscreen);
        }
    }
    
    /// Process input change requests from the UI
    fn process_input_requests(&mut self) {
        let (input1_request, input2_request) = {
            let mut state = self.shared_state.lock().unwrap();
            let req1 = std::mem::replace(&mut state.input1_request, InputChangeRequest::None);
            let req2 = std::mem::replace(&mut state.input2_request, InputChangeRequest::None);
            (req1, req2)
        };
        
        // Handle input 1 request
        match input1_request {
            InputChangeRequest::StartWebcam { device_index, width, height, fps } => {
                log::info!("Starting webcam on input 1: device={}", device_index);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input1_webcam(device_index, width, height, fps) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input1.is_active = true;
                            state.ndi_input1.source_name = format!("Webcam {}", device_index);
                        }
                        Err(e) => log::error!("Failed to start webcam: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StartNdi { source_name } => {
                log::info!("Starting NDI on input 1: {}", source_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input1_ndi(&source_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input1.is_active = true;
                            state.ndi_input1.source_name = source_name;
                        }
                        Err(e) => log::error!("Failed to start NDI: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StartObs { source_name } => {
                log::info!("Starting OBS on input 1: {}", source_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input1_obs(&source_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input1.is_active = true;
                            state.ndi_input1.source_name = source_name;
                        }
                        Err(e) => log::error!("Failed to start OBS: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StopInput => {
                if let Some(ref mut manager) = self.input_manager {
                    manager.stop_input1();
                    let mut state = self.shared_state.lock().unwrap();
                    state.ndi_input1.is_active = false;
                    state.ndi_input1.source_name.clear();
                }
            }
            InputChangeRequest::RefreshDevices => {
                if let Some(ref mut manager) = self.input_manager {
                    manager.invalidate_devices();
                }
            }
            _ => {}
        }
        
        // Handle input 2 request
        match input2_request {
            InputChangeRequest::StartWebcam { device_index, width, height, fps } => {
                log::info!("Starting webcam on input 2: device={}", device_index);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input2_webcam(device_index, width, height, fps) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input2.is_active = true;
                            state.ndi_input2.source_name = format!("Webcam {}", device_index);
                        }
                        Err(e) => log::error!("Failed to start webcam: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StartNdi { source_name } => {
                log::info!("Starting NDI on input 2: {}", source_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input2_ndi(&source_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input2.is_active = true;
                            state.ndi_input2.source_name = source_name;
                        }
                        Err(e) => log::error!("Failed to start NDI: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StartObs { source_name } => {
                log::info!("Starting OBS on input 2: {}", source_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input2_obs(&source_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input2.is_active = true;
                            state.ndi_input2.source_name = source_name;
                        }
                        Err(e) => log::error!("Failed to start OBS: {:?}", e),
                    }
                }
            }
            InputChangeRequest::StopInput => {
                if let Some(ref mut manager) = self.input_manager {
                    manager.stop_input2();
                    let mut state = self.shared_state.lock().unwrap();
                    state.ndi_input2.is_active = false;
                    state.ndi_input2.source_name.clear();
                }
            }
            _ => {}
        }
    }
    
    /// Process NDI output commands
    fn process_ndi_output_commands(&mut self) {
        let command = {
            let mut state = self.shared_state.lock().unwrap();
            std::mem::replace(&mut state.ndi_output_command, NdiOutputCommand::None)
        };
        
        match command {
            NdiOutputCommand::Start => {
                if self.ndi_output.is_none() {
                    let (name, include_alpha) = {
                        let state = self.shared_state.lock().unwrap();
                        (state.ndi_output.stream_name.clone(), state.ndi_output.include_alpha)
                    };
                    
                    if let Some(ref mut engine) = self.output_engine {
                        if let Err(e) = engine.start_ndi_output(&name, include_alpha, 0) {
                            log::error!("Failed to start NDI output: {:?}", e);
                        } else {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_output.is_active = true;
                        }
                    }
                }
            }
            NdiOutputCommand::Stop => {
                if let Some(ref mut engine) = self.output_engine {
                    engine.stop_ndi_output();
                }
                let mut state = self.shared_state.lock().unwrap();
                state.ndi_output.is_active = false;
            }
            _ => {}
        }
    }
    
    /// Update all inputs and upload frames to GPU
    fn update_inputs(&mut self) {
        if let Some(ref mut manager) = self.input_manager {
            // Update input manager (poll for new frames)
            manager.update();
            
            // Upload input 1 frame if available
            if manager.input1.has_frame() {
                if let Some(frame_data) = manager.input1.take_frame() {
                    let (width, height) = manager.input1.resolution();
                    if let Some(ref mut engine) = self.output_engine {
                        engine.input_texture_manager.update_input1(&frame_data, width, height);
                    }
                    // Update shared state
                    let mut state = self.shared_state.lock().unwrap();
                    state.ndi_input1.width = width;
                    state.ndi_input1.height = height;
                }
            }
            
            // Upload input 2 frame if available
            if manager.input2.has_frame() {
                if let Some(frame_data) = manager.input2.take_frame() {
                    let (width, height) = manager.input2.resolution();
                    if let Some(ref mut engine) = self.output_engine {
                        engine.input_texture_manager.update_input2(&frame_data, width, height);
                    }
                    // Update shared state
                    let mut state = self.shared_state.lock().unwrap();
                    state.ndi_input2.width = width;
                    state.ndi_input2.height = height;
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create shared wgpu instance
        if self.wgpu_instance.is_none() {
            self.wgpu_instance = Some(wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            }));
        }
        let instance = self.wgpu_instance.as_ref().unwrap();
        
        // Create output window
        if self.output_window.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title(&self.config.output_window.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.config.output_window.width,
                    self.config.output_window.height,
                ))
                .with_resizable(self.config.output_window.resizable)
                .with_decorations(self.config.output_window.decorated);
            
            let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
            
            // Hide cursor by default for output window
            window.set_cursor_visible(false);
            
            self.output_window = Some(Arc::clone(&window));
            
            // Initialize output engine
            let shared_state = Arc::clone(&self.shared_state);
            let config = self.config.clone();
            
            match pollster::block_on(WgpuEngine::new(
                instance,
                window,
                &config,
                shared_state,
            )) {
                Ok(engine) => {
                    log::info!("Output engine initialized");
                    // Store adapter for control window
                    self.wgpu_adapter = Some(engine.adapter.clone());
                    self.wgpu_device = Some(Arc::clone(&engine.device));
                    self.wgpu_queue = Some(Arc::clone(&engine.queue));
                    self.output_engine = Some(engine);
                }
                Err(err) => {
                    log::error!("Failed to create output engine: {}", err);
                    event_loop.exit();
                    return;
                }
            }
        }
        
        // Create control window
        if self.control_window.is_none() {
            if let Some(ref engine) = self.output_engine {
                let device = Arc::clone(&engine.device);
                let queue = Arc::clone(&engine.queue);
                
                let window_attrs = WindowAttributes::default()
                    .with_title(&self.config.control_window.title)
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        self.config.control_window.width,
                        self.config.control_window.height,
                    ))
                    .with_resizable(true)
                    .with_decorations(true);
                
                let window = Arc::new(event_loop.create_window(window_attrs).unwrap());
                self.control_window = Some(Arc::clone(&window));
                
                // Get adapter from stored resources
                let adapter = self.wgpu_adapter.as_ref().unwrap();
                
                // Initialize ImGui renderer
                match pollster::block_on(ImGuiRenderer::new(
                    instance,
                    adapter,
                    device,
                    queue,
                    window,
                    1.0,
                )) {
                    Ok(renderer) => {
                        match ControlGui::new(&self.config, Arc::clone(&self.shared_state)) {
                            Ok(gui) => {
                                self.control_gui = Some(gui);
                                self.imgui_renderer = Some(renderer);
                            }
                            Err(err) => {
                                log::error!("Failed to create control GUI: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to create ImGui renderer: {}", err);
                    }
                }
            }
        }
    }
    
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Handle output window events
        if let Some(ref output_window) = self.output_window {
            if window_id == output_window.id() {
                match event {
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }
                    WindowEvent::CursorEntered { .. } => {
                        output_window.set_cursor_visible(false);
                    }
                    WindowEvent::CursorLeft { .. } => {
                        output_window.set_cursor_visible(true);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Track shift key
                        match &event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Shift) => {
                                self.shift_pressed = event.state == winit::event::ElementState::Pressed;
                            }
                            _ => {}
                        }
                        
                        if event.state == winit::event::ElementState::Pressed {
                            match &event.logical_key {
                                winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                    event_loop.exit();
                                }
                                winit::keyboard::Key::Character(ch) => {
                                    let key = ch.to_lowercase();
                                    if self.shift_pressed && key == "f" {
                                        self.toggle_fullscreen();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(ref mut engine) = self.output_engine {
                            engine.resize(size.width, size.height);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        if let Some(ref mut engine) = self.output_engine {
                            engine.render();
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
        
        // Handle control window events
        if let Some(ref control_window) = self.control_window {
            if window_id == control_window.id() {
                if let Some(ref mut renderer) = self.imgui_renderer {
                    renderer.handle_event(&event);
                }
                
                match event {
                    WindowEvent::CloseRequested => {
                        // Just close control window, keep output running
                        self.control_window = None;
                        self.control_gui = None;
                        self.imgui_renderer = None;
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(ref mut renderer) = self.imgui_renderer {
                            renderer.resize(size.width, size.height);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        // Render ImGui
                        if let (Some(ref mut renderer), Some(ref mut gui)) = 
                            (self.imgui_renderer.as_mut(), self.control_gui.as_mut()) 
                        {
                            let window_size = control_window.inner_size();
                            renderer.set_display_size(window_size.width as f32, window_size.height as f32);
                            
                            if let Err(err) = renderer.render_frame(|ui| gui.build_ui(ui)) {
                                log::error!("ImGui render error: {}", err);
                            }
                            
                            // Actual rendering would go here in a full implementation
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Initialize input manager if needed (after engine is ready)
        if self.input_manager.is_none() {
            self.input_manager = Some(InputManager::new());
            log::info!("InputManager initialized");
        }
        
        // Process input change requests
        self.process_input_requests();
        
        // Handle NDI output commands
        self.process_ndi_output_commands();
        
        // Update inputs and upload frames to GPU
        self.update_inputs();
        
        // Request redraws
        if let Some(ref window) = self.output_window {
            window.request_redraw();
        }
        
        if let Some(ref window) = self.control_window {
            window.request_redraw();
        }
    }
}
