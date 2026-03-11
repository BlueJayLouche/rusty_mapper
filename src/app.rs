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
use crate::videowall::{CalibrationController, CalibrationStatus, VideoMatrixConfig};

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
    
    // Track last uploaded matrix pattern to avoid re-uploading
    last_matrix_pattern: Option<(u32, u32)>, // (width, height)
    
    // Cache last video matrix config to avoid redundant updates
    last_video_matrix_config: Option<VideoMatrixConfig>,
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
            last_matrix_pattern: None,
            last_video_matrix_config: None,
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
            #[cfg(target_os = "macos")]
            InputChangeRequest::StartSyphon { server_name } => {
                log::info!("Starting Syphon on input 1: {}", server_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input1_syphon(&server_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input1.is_active = true;
                            state.ndi_input1.source_name = server_name;
                        }
                        Err(e) => log::error!("Failed to start Syphon: {:?}", e),
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
            #[cfg(target_os = "macos")]
            InputChangeRequest::StartSyphon { server_name } => {
                log::info!("Starting Syphon on input 2: {}", server_name);
                if let Some(ref mut manager) = self.input_manager {
                    match manager.start_input2_syphon(&server_name) {
                        Ok(_) => {
                            let mut state = self.shared_state.lock().unwrap();
                            state.ndi_input2.is_active = true;
                            state.ndi_input2.source_name = server_name;
                        }
                        Err(e) => log::error!("Failed to start Syphon: {:?}", e),
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
            
            // Check if calibration is waiting for capture
            let calibration_waiting = {
                let state = self.shared_state.lock().unwrap();
                state.videowall_calibration.as_ref()
                    .map(|c| c.is_ready_for_capture())
                    .unwrap_or(false)
            };
            
            // Check if we're showing matrix test pattern (skip normal input)
            let showing_matrix_pattern = {
                let state = self.shared_state.lock().unwrap();
                state.matrix_showing_test_pattern
            };
            
            // Upload input 1 frame if available (unless showing test pattern)
            if manager.input1.has_frame() && !showing_matrix_pattern {
                // Check if this is Syphon input (zero-copy texture path)
                #[cfg(target_os = "macos")]
                if manager.input1.input_type() == crate::input::InputType::Syphon {
                    // Use zero-copy texture path for Syphon
                    if let Some(texture) = manager.input1.take_syphon_texture() {
                        let width = texture.width();
                        let height = texture.height();
                        
                        if let Some(ref mut engine) = self.output_engine {
                            // GPU-to-GPU copy (zero CPU involvement)
                            engine.input_texture_manager.update_input1_from_texture(&texture);
                        }
                        
                        // Update shared state
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input1.width = width;
                        state.ndi_input1.height = height;
                    }
                } else {
                    // CPU fallback path for NDI/Webcam
                    if let Some(frame_data) = manager.input1.take_frame() {
                        let (width, height) = manager.input1.resolution();
                        
                        // If calibration is waiting, submit this frame
                        if calibration_waiting {
                            let mut state = self.shared_state.lock().unwrap();
                            if let Some(ref mut calibration) = state.videowall_calibration {
                                log::info!("Auto-submitting camera frame {}x{} for calibration", width, height);
                                calibration.submit_frame(frame_data.clone(), width, height);
                            }
                        }
                        
                        if let Some(ref mut engine) = self.output_engine {
                            engine.input_texture_manager.update_input1(&frame_data, width, height);
                        }
                        // Update shared state
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input1.width = width;
                        state.ndi_input1.height = height;
                    }
                }
                
                #[cfg(not(target_os = "macos"))]
                {
                    // Non-macOS: use CPU path only
                    if let Some(frame_data) = manager.input1.take_frame() {
                        let (width, height) = manager.input1.resolution();
                        
                        if calibration_waiting {
                            let mut state = self.shared_state.lock().unwrap();
                            if let Some(ref mut calibration) = state.videowall_calibration {
                                log::info!("Auto-submitting camera frame {}x{} for calibration", width, height);
                                calibration.submit_frame(frame_data.clone(), width, height);
                            }
                        }
                        
                        if let Some(ref mut engine) = self.output_engine {
                            engine.input_texture_manager.update_input1(&frame_data, width, height);
                        }
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input1.width = width;
                        state.ndi_input1.height = height;
                    }
                }
            } else if showing_matrix_pattern {
                // Discard frame without processing when showing test pattern
                if manager.input1.has_frame() {
                    let _ = manager.input1.take_frame();
                    #[cfg(target_os = "macos")]
                    let _ = manager.input1.take_syphon_texture();
                }
            }
            
            // Upload input 2 frame if available
            if manager.input2.has_frame() {
                // Check if this is Syphon input (zero-copy texture path)
                #[cfg(target_os = "macos")]
                if manager.input2.input_type() == crate::input::InputType::Syphon {
                    if let Some(texture) = manager.input2.take_syphon_texture() {
                        let width = texture.width();
                        let height = texture.height();
                        
                        if let Some(ref mut engine) = self.output_engine {
                            engine.input_texture_manager.update_input2_from_texture(&texture);
                        }
                        
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input2.width = width;
                        state.ndi_input2.height = height;
                    }
                } else {
                    if let Some(frame_data) = manager.input2.take_frame() {
                        let (width, height) = manager.input2.resolution();
                        if let Some(ref mut engine) = self.output_engine {
                            engine.input_texture_manager.update_input2(&frame_data, width, height);
                        }
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input2.width = width;
                        state.ndi_input2.height = height;
                    }
                }
                
                #[cfg(not(target_os = "macos"))]
                {
                    if let Some(frame_data) = manager.input2.take_frame() {
                        let (width, height) = manager.input2.resolution();
                        if let Some(ref mut engine) = self.output_engine {
                            engine.input_texture_manager.update_input2(&frame_data, width, height);
                        }
                        let mut state = self.shared_state.lock().unwrap();
                        state.ndi_input2.width = width;
                        state.ndi_input2.height = height;
                    }
                }
            }
        }
    }
    
    /// Process video wall calibration updates
    fn process_videowall_calibration(&mut self) {
        // Check if calibration is active in shared state
        let calibration_active = {
            let mut state = self.shared_state.lock().unwrap();
            if let Some(ref mut calibration) = state.videowall_calibration {
                if calibration.is_active() {
                    // Update calibration
                    match calibration.update() {
                        CalibrationStatus::InProgress | CalibrationStatus::ReadyForCapture | CalibrationStatus::Processing => {
                            // Get current pattern and upload to output
                            if let Some(pattern) = calibration.current_pattern() {
                                // Upload pattern to GPU texture
                                let (width, height) = (pattern.width(), pattern.height());
                                let rgba_data: Vec<u8> = pattern.pixels()
                                    .flat_map(|p| [p[0], p[1], p[2], p[3]])
                                    .collect();
                                
                                if let Some(ref mut engine) = self.output_engine {
                                    engine.upload_calibration_pattern(&rgba_data, width, height);
                                }
                            }
                            true
                        }
                        CalibrationStatus::Complete(config) => {
                            log::info!("Calibration complete! {} displays configured", config.displays.len());
                            state.videowall_config = Some(config.clone());
                            state.videowall_enabled = true;
                            false
                        }
                        CalibrationStatus::Error(ref e) => {
                            log::error!("Calibration error: {}", e);
                            false
                        }
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        // Clear calibration if not active
        if !calibration_active {
            let mut state = self.shared_state.lock().unwrap();
            if let Some(ref calibration) = state.videowall_calibration {
                if !calibration.is_active() {
                    state.videowall_calibration = None;
                }
            }
        }
        
        // Process matrix test pattern (AprilTag pattern display)
        self.process_matrix_test_pattern();
    }
    
    /// Process matrix test pattern display
    fn process_matrix_test_pattern(&mut self) {
        let pattern_to_display = {
            let state = self.shared_state.lock().unwrap();
            state.matrix_test_pattern.clone()
        };
        
        if let Some((rgba_data, width, height)) = pattern_to_display {
            // Only upload if pattern dimensions changed (avoid re-uploading same pattern)
            let should_upload = self.last_matrix_pattern != Some((width, height));
            
            if should_upload {
                if let Some(ref mut engine) = self.output_engine {
                    // Upload pattern to output for display
                    if let Err(e) = engine.upload_test_pattern(&rgba_data, width, height) {
                        log::error!("Failed to upload matrix test pattern: {}", e);
                    } else {
                        self.last_matrix_pattern = Some((width, height));
                        log::info!("Uploaded matrix test pattern: {}x{}", width, height);
                    }
                }
            }
        } else {
            // Pattern cleared
            self.last_matrix_pattern = None;
        }
    }
    
    /// Sync video wall state from shared state to engine
    fn sync_video_wall_state(&mut self) {
        let (enabled, config) = {
            let state = self.shared_state.lock().unwrap();
            (state.videowall_enabled, state.videowall_config.clone())
        };
        
        if let Some(ref mut engine) = self.output_engine {
            // Enable/disable video wall rendering
            engine.set_video_wall_enabled(enabled);
            
            // Update config if available
            if enabled {
                if let Some(ref cfg) = config {
                    engine.update_video_wall_config(cfg);
                }
            }
        }
    }
    
    /// Sync video matrix state from shared state to engine
    fn sync_video_matrix_state(&mut self) {
        let (enabled, config) = {
            let state = self.shared_state.lock().unwrap();
            (state.video_matrix_enabled, state.video_matrix_config.clone())
        };
        
        let mapping_count = config.input_grid.mappings.len();
        
        if let Some(ref mut engine) = self.output_engine {
            // Enable/disable video matrix rendering
            let was_enabled = engine.is_video_matrix_enabled();
            engine.set_video_matrix_enabled(enabled);
            
            // Check if config changed
            let config_changed = self.last_video_matrix_config.as_ref() != Some(&config);
            
            // Update config if enabled and changed
            if enabled && config_changed {
                log::info!("Video matrix config updated: {} mappings", mapping_count);
                engine.update_video_matrix_config(&config);
                self.last_video_matrix_config = Some(config);
            }
            
            // Log state change
            if enabled != was_enabled {
                log::info!("Video matrix {} ({} mappings)", 
                    if enabled { "ENABLED" } else { "DISABLED" },
                    mapping_count);
            }
        }
    }
    
    /// Update preview textures for GUI display
    /// Copies input and output textures to ImGui preview textures
    fn update_preview_textures(&mut self) {
        // Need both renderer and output engine
        let (input_tex, output_tex) = {
            if let Some(ref engine) = self.output_engine {
                // Get input texture from texture manager
                let input = engine.input_texture_manager().input1.as_ref()
                    .map(|t| &t.texture);
                
                // Get output texture (video matrix or render target)
                let output = if engine.is_video_matrix_enabled() {
                    engine.video_matrix_output_texture()
                        .map(|t| &t.texture)
                        .or_else(|| Some(&engine.render_target().texture))
                } else {
                    Some(&engine.render_target().texture)
                };
                
                (input, output)
            } else {
                (None, None)
            }
        };
        
        // Update preview textures via ImGui renderer
        if let Some(ref mut renderer) = self.imgui_renderer {
            if let (Some(input), Some(gui)) = (input_tex, self.control_gui.as_ref()) {
                if let Some(preview_id) = gui.input_preview_texture_id {
                    // Create command encoder for copy
                    let mut encoder = renderer.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Preview Update Encoder"),
                    });
                    
                    renderer.update_preview_texture(preview_id, input, &mut encoder);
                    renderer.queue().submit(std::iter::once(encoder.finish()));
                }
            }
            
            if let (Some(output), Some(gui)) = (output_tex, self.control_gui.as_ref()) {
                if let Some(preview_id) = gui.output_preview_texture_id {
                    // Create command encoder for copy
                    let mut encoder = renderer.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Preview Update Encoder"),
                    });
                    
                    renderer.update_preview_texture(preview_id, output, &mut encoder);
                    renderer.queue().submit(std::iter::once(encoder.finish()));
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
                    Ok(mut renderer) => {
                        match ControlGui::new(&self.config, Arc::clone(&self.shared_state)) {
                            Ok(mut gui) => {
                                // Create preview textures for input and output
                                // Use full resolution so the entire texture is visible
                                let internal_width = self.config.resolution.internal_width;
                                let internal_height = self.config.resolution.internal_height;
                                let input_preview_id = renderer.create_preview_texture(internal_width, internal_height);
                                let output_preview_id = renderer.create_preview_texture(internal_width, internal_height);
                                
                                gui.set_input_preview_texture(input_preview_id);
                                gui.set_output_preview_texture(output_preview_id);
                                
                                log::info!("Created preview textures: input={:?}, output={:?} ({}x{})", 
                                    input_preview_id, output_preview_id, internal_width, internal_height);
                                
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
                            
                            // Update preview textures after rendering
                            self.update_preview_textures();
                        }
                    }
                    // Mouse handling for corner adjustment
                    WindowEvent::MouseInput { state: button_state, button, .. } => {
                        if button == winit::event::MouseButton::Left {
                            let mut shared_state = self.shared_state.lock().unwrap();
                            if shared_state.videowall_edit_mode {
                                if button_state == winit::event::ElementState::Pressed {
                                    // Start dragging selected corner
                                    // (Selection is done via GUI)
                                } else {
                                    // Release - stop dragging
                                    shared_state.videowall_edit_corner = None;
                                    shared_state.videowall_edit_display = None;
                                }
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let mut shared_state = self.shared_state.lock().unwrap();
                        if shared_state.videowall_edit_mode {
                            if let (Some(display_id), Some(corner_idx)) = 
                                (shared_state.videowall_edit_display, shared_state.videowall_edit_corner) 
                            {
                                // Get window size for normalization
                                if let Some(ref output_window) = self.output_window {
                                    let window_size = output_window.inner_size();
                                    let x = position.x as f32 / window_size.width as f32;
                                    let y = position.y as f32 / window_size.height as f32;
                                    
                                    // Update the corner position
                                    if let Some(ref mut config) = shared_state.videowall_config {
                                        if let Some(display) = config.displays.iter_mut().find(|d| d.id == display_id) {
                                            if corner_idx < 4 {
                                                display.dest_quad[corner_idx] = [x, y];
                                            }
                                        }
                                    }
                                }
                            }
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
            let mut manager = InputManager::new();
            
            // Initialize with wgpu device/queue for Syphon support
            if let (Some(ref device), Some(ref queue)) = (&self.wgpu_device, &self.wgpu_queue) {
                manager.initialize(device, queue);
                log::info!("InputManager initialized with wgpu resources");
            } else {
                log::warn!("InputManager initialized without wgpu resources - Syphon will not be available");
            }
            
            self.input_manager = Some(manager);
        }
        
        // Process input change requests
        self.process_input_requests();
        
        // Handle NDI output commands
        self.process_ndi_output_commands();
        
        // Update video wall calibration
        self.process_videowall_calibration();
        
        // Sync video wall state to engine
        self.sync_video_wall_state();
        
        // Sync video matrix state to engine
        self.sync_video_matrix_state();
        
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
