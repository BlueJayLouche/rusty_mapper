//! # Control GUI
//!
//! ImGui-based control interface for the application.
//! Supports multiple input types: Webcam, NDI, OBS (via NDI)

// Allow deprecated ComboBox API - imgui 0.12 uses the older API
#![allow(deprecated)]

use crate::config::AppConfig;
use crate::core::{SharedState, NdiOutputCommand, InputChangeRequest, InputMapping};
use std::sync::{Arc, Mutex};

/// Main GUI tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Inputs,
    Mapping,
    Output,
    Settings,
}

/// Control GUI state
pub struct ControlGui {
    shared_state: Arc<Mutex<SharedState>>,
    
    // Current tab
    current_tab: MainTab,
    
    // Device lists
    webcam_devices: Vec<String>,
    ndi_sources: Vec<String>,
    
    // Selection state
    selected_webcam1: i32,
    selected_webcam2: i32,
    selected_ndi1: i32,
    selected_ndi2: i32,
    
    // Device selector popup
    show_device_selector: bool,
    selector_for_input: i32, // 1 or 2
    
    // Mapping tab input selection (0 = Input 1, 1 = Input 2)
    mapping_tab_input: i32,
    
    // Output
    ndi_output_name: String,
    syphon_server_name: String,
    
    // Mapping edit state (local copy to reduce lock contention)
    mapping_edit_input1: InputMapping,
    mapping_edit_input2: InputMapping,
    mapping_needs_update: bool,
}

impl ControlGui {
    pub fn new(_config: &AppConfig, shared_state: Arc<Mutex<SharedState>>) -> anyhow::Result<Self> {
        let (ndi_output_name, syphon_server_name, mapping1, mapping2) = {
            let state = shared_state.lock().unwrap();
            (
                state.ndi_output.stream_name.clone(),
                state.syphon_output.server_name.clone(),
                state.input1_mapping,
                state.input2_mapping,
            )
        };
        
        Ok(Self {
            shared_state,
            current_tab: MainTab::Inputs,
            webcam_devices: Vec::new(),
            ndi_sources: Vec::new(),
            selected_webcam1: 0,
            selected_webcam2: 0,
            selected_ndi1: 0,
            selected_ndi2: 0,
            show_device_selector: false,
            selector_for_input: 1,
            mapping_tab_input: 0,
            ndi_output_name,
            syphon_server_name,
            mapping_edit_input1: mapping1,
            mapping_edit_input2: mapping2,
            mapping_needs_update: false,
        })
    }
    
    /// Refresh all device lists
    pub fn refresh_devices(&mut self) {
        #[cfg(feature = "webcam")]
        {
            self.webcam_devices = crate::input::list_cameras();
            log::info!("Found {} webcam devices", self.webcam_devices.len());
        }
        
        self.ndi_sources = crate::input::list_ndi_sources(2000);
        log::info!("Found {} NDI sources", self.ndi_sources.len());
    }
    
    /// Sync mapping edits back to shared state
    fn sync_mapping_to_state(&mut self) {
        if self.mapping_needs_update {
            let mut state = self.shared_state.lock().unwrap();
            state.input1_mapping = self.mapping_edit_input1;
            state.input2_mapping = self.mapping_edit_input2;
            self.mapping_needs_update = false;
        }
    }
    
    /// Build the ImGui UI - builds directly to root (no wrapper window)
    pub fn build_ui(&mut self, ui: &mut imgui::Ui) {
        // Sync mapping changes to shared state
        self.sync_mapping_to_state();
        
        // Menu bar at the top
        self.build_menu_bar(ui);
        
        // Main tab bar with content
        self.build_main_tabs(ui);
        
        // Device selector window (separate popup window)
        if self.show_device_selector {
            self.build_device_selector(ui);
        }
    }
    
    /// Build the menu bar
    fn build_menu_bar(&mut self, ui: &imgui::Ui) {
        ui.menu_bar(|| {
            ui.menu("File", || {
                if ui.menu_item("Exit") {
                    // Exit handled by app
                }
            });
            
            ui.menu("Devices", || {
                if ui.menu_item("Refresh All") {
                    self.refresh_devices();
                }
            });
        });
    }
    
    /// Build main tab bar - uses imgui 0.12 tab API
    fn build_main_tabs(&mut self, ui: &imgui::Ui) {
        let tab_labels = [("Inputs", MainTab::Inputs), 
                          ("Mapping", MainTab::Mapping), 
                          ("Output", MainTab::Output),
                          ("Settings", MainTab::Settings)];
        
        // Use tab_bar/tab_item for proper tab behavior in imgui 0.12
        if let Some(_tab_bar) = ui.tab_bar("##main_tabs") {
            for (label, tab) in tab_labels.iter() {
                let is_selected = self.current_tab == *tab;
                
                if let Some(_tab) = ui.tab_item(label) {
                    if !is_selected {
                        self.current_tab = *tab;
                    }
                }
            }
        }
        
        ui.separator();
        
        // Build content for current tab
        match self.current_tab {
            MainTab::Inputs => self.build_inputs_tab(ui),
            MainTab::Mapping => self.build_mapping_tab(ui),
            MainTab::Output => self.build_output_tab(ui),
            MainTab::Settings => self.build_settings_tab(ui),
        }
    }
    
    /// Build the Inputs tab
    fn build_inputs_tab(&mut self, ui: &imgui::Ui) {
        ui.text("Input Sources");
        ui.separator();
        
        // Input 1 section
        ui.text_colored([0.0, 1.0, 1.0, 1.0], "Input 1 (Primary)");
        self.build_input_section(ui, 1);
        
        ui.spacing();
        ui.separator();
        ui.spacing();
        
        // Input 2 section
        ui.text_colored([1.0, 0.5, 0.0, 1.0], "Input 2 (Secondary)");
        self.build_input_section(ui, 2);
        
        ui.spacing();
        ui.separator();
        
        // Mix controls
        ui.text("Mix Controls");
        let mut mix_amount = {
            let state = self.shared_state.lock().unwrap();
            state.mix_amount
        };
        let old_mix = mix_amount;
        if ui.slider("Mix Amount", 0.0, 1.0, &mut mix_amount) {
            let mut state = self.shared_state.lock().unwrap();
            state.mix_amount = mix_amount;
            log::debug!("Mix slider changed: {:.2} -> {:.2}", old_mix, mix_amount);
        }
        ui.same_line();
        ui.text(format!("{:.0}% Input 2 (current: {:.2})", mix_amount * 100.0, mix_amount));
    }
    
    /// Build a single input section
    fn build_input_section(&mut self, ui: &imgui::Ui, input_num: i32) {
        let (is_active, source_name, input_type_str) = {
            let state = self.shared_state.lock().unwrap();
            let input_state = if input_num == 1 { &state.ndi_input1 } else { &state.ndi_input2 };
            (
                input_state.is_active,
                input_state.source_name.clone(),
                if input_state.is_active { "Active" } else { "None" },
            )
        };
        
        // Status display
        ui.text(format!("Status: {}", input_type_str));
        if is_active {
            ui.text(format!("Source: {}", source_name));
        }
        
        // Action buttons
        if ui.button(format!("Select Source##{}", input_num)) {
            self.selector_for_input = input_num;
            self.show_device_selector = true;
            self.refresh_devices();
        }
        
        if is_active {
            ui.same_line();
            if ui.button(format!("Stop##{}", input_num)) {
                let mut state = self.shared_state.lock().unwrap();
                if input_num == 1 {
                    state.input1_request = InputChangeRequest::StopInput;
                } else {
                    state.input2_request = InputChangeRequest::StopInput;
                }
            }
            
            ui.same_line();
            if ui.button(format!("Edit Mapping##{}", input_num)) {
                self.current_tab = MainTab::Mapping;
            }
        }
    }
    
    /// Build the Mapping tab
    fn build_mapping_tab(&mut self, ui: &imgui::Ui) {
        ui.text("Projection Mapping");
        ui.separator();
        
        // Select which input to map
        ui.text("Select Input to Map:");
        ui.radio_button("Input 1", &mut self.mapping_tab_input, 0);
        ui.same_line();
        ui.radio_button("Input 2", &mut self.mapping_tab_input, 1);
        
        ui.separator();
        
        // Get the mapping to edit
        let mapping = if self.mapping_tab_input == 0 {
            &mut self.mapping_edit_input1
        } else {
            &mut self.mapping_edit_input2
        };
        
        // Corner pinning section
        ui.text_colored([1.0, 1.0, 0.0, 1.0], "Corner Pinning (UV Coordinates)");
        ui.text("Drag corners to warp the input");
        
        // Top row
        ui.columns(2, "corners_top", false);
        ui.text("Top-Left");
        if ui.slider("TL X", 0.0, 1.0, &mut mapping.corner0[0]) { self.mapping_needs_update = true; }
        if ui.slider("TL Y", 0.0, 1.0, &mut mapping.corner0[1]) { self.mapping_needs_update = true; }
        ui.next_column();
        ui.text("Top-Right");
        if ui.slider("TR X", 0.0, 1.0, &mut mapping.corner1[0]) { self.mapping_needs_update = true; }
        if ui.slider("TR Y", 0.0, 1.0, &mut mapping.corner1[1]) { self.mapping_needs_update = true; }
        ui.columns(1, "", false);
        
        // Bottom row
        ui.columns(2, "corners_bottom", false);
        ui.text("Bottom-Left");
        if ui.slider("BL X", 0.0, 1.0, &mut mapping.corner3[0]) { self.mapping_needs_update = true; }
        if ui.slider("BL Y", 0.0, 1.0, &mut mapping.corner3[1]) { self.mapping_needs_update = true; }
        ui.next_column();
        ui.text("Bottom-Right");
        if ui.slider("BR X", 0.0, 1.0, &mut mapping.corner2[0]) { self.mapping_needs_update = true; }
        if ui.slider("BR Y", 0.0, 1.0, &mut mapping.corner2[1]) { self.mapping_needs_update = true; }
        ui.columns(1, "", false);
        
        ui.separator();
        
        // Global transforms
        ui.text_colored([1.0, 1.0, 0.0, 1.0], "Global Transform");
        if ui.slider("Scale X", 0.1, 3.0, &mut mapping.scale[0]) { self.mapping_needs_update = true; }
        if ui.slider("Scale Y", 0.1, 3.0, &mut mapping.scale[1]) { self.mapping_needs_update = true; }
        if ui.slider("Offset X", -1.0, 1.0, &mut mapping.offset[0]) { self.mapping_needs_update = true; }
        if ui.slider("Offset Y", -1.0, 1.0, &mut mapping.offset[1]) { self.mapping_needs_update = true; }
        if ui.slider("Rotation", -180.0, 180.0, &mut mapping.rotation) { self.mapping_needs_update = true; }
        
        ui.separator();
        
        // Opacity and blend
        ui.text_colored([1.0, 1.0, 0.0, 1.0], "Blend Settings");
        if ui.slider("Opacity", 0.0, 1.0, &mut mapping.opacity) { self.mapping_needs_update = true; }
        
        let blend_modes = ["Normal", "Add", "Multiply", "Screen"];
        ui.text("Blend Mode:");
        for (i, mode) in blend_modes.iter().enumerate() {
            if ui.radio_button(mode, &mut mapping.blend_mode, i as i32) {
                self.mapping_needs_update = true;
            }
            if i < blend_modes.len() - 1 {
                ui.same_line();
            }
        }
        
        ui.separator();
        
        // Reset button
        if ui.button("Reset to Default") {
            mapping.reset();
            self.mapping_needs_update = true;
        }
        ui.same_line();
        if ui.button("Reset Corners Only") {
            mapping.corner0 = [0.0, 0.0];
            mapping.corner1 = [1.0, 0.0];
            mapping.corner2 = [1.0, 1.0];
            mapping.corner3 = [0.0, 1.0];
            self.mapping_needs_update = true;
        }
    }
    
    /// Build the Output tab
    fn build_output_tab(&mut self, ui: &imgui::Ui) {
        ui.text("Output Settings");
        ui.separator();
        
        // Fullscreen toggle
        let mut fullscreen = {
            let state = self.shared_state.lock().unwrap();
            state.output_fullscreen
        };
        
        if ui.checkbox("Fullscreen Output", &mut fullscreen) {
            let mut state = self.shared_state.lock().unwrap();
            state.output_fullscreen = fullscreen;
        }
        
        ui.separator();
        
        // NDI Output section
        ui.text_colored([0.0, 1.0, 0.5, 1.0], "NDI Output");
        
        ui.input_text("Stream Name", &mut self.ndi_output_name)
            .build();
        
        let ndi_active = {
            let state = self.shared_state.lock().unwrap();
            state.ndi_output.is_active
        };
        
        if !ndi_active {
            if ui.button("Start NDI Output") {
                let mut state = self.shared_state.lock().unwrap();
                state.ndi_output.stream_name = self.ndi_output_name.clone();
                state.ndi_output_command = NdiOutputCommand::Start;
            }
        } else {
            if ui.button("Stop NDI Output") {
                let mut state = self.shared_state.lock().unwrap();
                state.ndi_output_command = NdiOutputCommand::Stop;
            }
        }
        
        // Syphon Output section (macOS only)
        #[cfg(target_os = "macos")]
        {
            ui.separator();
            ui.text_colored([1.0, 0.5, 0.0, 1.0], "Syphon Output (macOS)");
            ui.text_disabled("Share GPU texture with Resolume, MadMapper, etc.");
            
            // Syphon server name input
            ui.input_text("Server Name", &mut self.syphon_server_name)
                .build();
            
            // Check if syphon should be active from shared state
            let syphon_requested = {
                let state = self.shared_state.lock().unwrap();
                state.syphon_output.enabled
            };
            
            if !syphon_requested {
                if ui.button("Start Syphon Output") {
                    let mut state = self.shared_state.lock().unwrap();
                    state.syphon_output.server_name = self.syphon_server_name.clone();
                    state.syphon_output.enabled = true;
                }
            } else {
                if ui.button("Stop Syphon Output") {
                    let mut state = self.shared_state.lock().unwrap();
                    state.syphon_output.enabled = false;
                }
            }
            
            ui.text(format!("Status: {}", 
                if syphon_requested { "Active" } else { "Inactive" }));
        }
        
        // Status
        ui.separator();
        ui.text("Status:");
        let state = self.shared_state.lock().unwrap();
        ui.text(format!("NDI Output: {}", 
            if state.ndi_output.is_active { "Active" } else { "Inactive" }));
        ui.text(format!("Input 1: {} ({}x{})", 
            if state.ndi_input1.is_active { "Active" } else { "Inactive" },
            state.ndi_input1.width,
            state.ndi_input1.height));
        ui.text(format!("Input 2: {} ({}x{})", 
            if state.ndi_input2.is_active { "Active" } else { "Inactive" },
            state.ndi_input2.width,
            state.ndi_input2.height));
    }
    
    /// Build the Settings tab
    fn build_settings_tab(&mut self, ui: &imgui::Ui) {
        ui.text("Application Settings");
        ui.separator();
        
        ui.text("UI Scale:");
        let mut ui_scale = {
            let state = self.shared_state.lock().unwrap();
            state.ui_scale
        };
        if ui.slider("Scale", 0.5, 2.0, &mut ui_scale) {
            let mut state = self.shared_state.lock().unwrap();
            state.ui_scale = ui_scale;
        }
        
        ui.separator();
        
        ui.text("Keyboard Shortcuts:");
        ui.bullet_text("Shift+F - Toggle Fullscreen");
        ui.bullet_text("Escape - Exit Application");
        
        ui.separator();
        
        if ui.button("Refresh All Devices") {
            self.refresh_devices();
        }
    }
    
    /// Build device selector window
    fn build_device_selector(&mut self, ui: &imgui::Ui) {
        let input_num = self.selector_for_input;
        
        ui.window(format!("Select Source for Input {}", input_num))
            .size([350.0, 400.0], imgui::Condition::FirstUseEver)
            .build(|| {
                if ui.button("Refresh") {
                    self.refresh_devices();
                }
                
                ui.separator();
                
                // Source type tabs
                if let Some(tab_bar) = ui.tab_bar("SourceTypeTabs") {
                    // Webcam tab
                    #[cfg(feature = "webcam")]
                    if let Some(tab) = ui.tab_item("Webcam") {
                        // Collect devices first to avoid borrow issues
                        let devices: Vec<(usize, String)> = self.webcam_devices.iter()
                            .enumerate()
                            .map(|(i, d)| (i, d.clone()))
                            .collect();
                        
                        if devices.is_empty() {
                            ui.text("No webcam devices found");
                        } else {
                            for (i, device) in devices {
                                let is_selected = if input_num == 1 { 
                                    self.selected_webcam1 == i as i32 
                                } else { 
                                    self.selected_webcam2 == i as i32 
                                };
                                
                                if ui.selectable_config(&device)
                                    .selected(is_selected)
                                    .build() 
                                {
                                    if input_num == 1 {
                                        self.selected_webcam1 = i as i32;
                                    } else {
                                        self.selected_webcam2 = i as i32;
                                    }
                                    self.select_webcam(input_num, i);
                                    self.show_device_selector = false;
                                }
                            }
                        }
                        tab.end();
                    }
                    
                    // NDI tab
                    if let Some(tab) = ui.tab_item("NDI") {
                        // Collect sources first to avoid borrow issues
                        let sources: Vec<(usize, String)> = self.ndi_sources.iter()
                            .enumerate()
                            .filter(|(_, s)| !s.to_lowercase().contains("obs"))
                            .map(|(i, s)| (i, s.clone()))
                            .collect();
                        
                        if sources.is_empty() {
                            ui.text("No NDI sources found");
                            ui.text("Make sure NDI sources are active on the network.");
                        } else {
                            for (i, source) in sources {
                                let is_selected = if input_num == 1 { 
                                    self.selected_ndi1 == i as i32 
                                } else { 
                                    self.selected_ndi2 == i as i32 
                                };
                                
                                if ui.selectable_config(&source)
                                    .selected(is_selected)
                                    .build() 
                                {
                                    if input_num == 1 {
                                        self.selected_ndi1 = i as i32;
                                    } else {
                                        self.selected_ndi2 = i as i32;
                                    }
                                    self.select_ndi(input_num, source);
                                    self.show_device_selector = false;
                                }
                            }
                        }
                        tab.end();
                    }
                    
                    // OBS tab
                    if let Some(tab) = ui.tab_item("OBS") {
                        // Collect OBS sources first to avoid borrow issues
                        let obs_sources: Vec<(usize, String)> = self.ndi_sources.iter()
                            .enumerate()
                            .filter(|(_, s)| s.to_lowercase().contains("obs"))
                            .map(|(i, s)| (i, s.clone()))
                            .collect();
                        
                        if obs_sources.is_empty() {
                            ui.text("No OBS NDI sources found");
                            ui.text("Enable NDI output in OBS Tools menu.");
                        } else {
                            for (i, source) in obs_sources {
                                if ui.button(format!("{}##obs{}", source, i)) {
                                    self.select_obs(input_num, source);
                                    self.show_device_selector = false;
                                }
                            }
                        }
                        tab.end();
                    }
                    
                    tab_bar.end();
                }
                
                ui.separator();
                
                if ui.button("Cancel") {
                    self.show_device_selector = false;
                }
            });
    }
    
    /// Select webcam for input
    fn select_webcam(&mut self, input_num: i32, device_index: usize) {
        let mut state = self.shared_state.lock().unwrap();
        let request = InputChangeRequest::StartWebcam {
            device_index,
            width: 1920,
            height: 1080,
            fps: 30,
        };
        
        if input_num == 1 {
            state.input1_request = request;
        } else {
            state.input2_request = request;
        }
        
        log::info!("Selected webcam {} for input {}", device_index, input_num);
    }
    
    /// Select NDI source for input
    fn select_ndi(&mut self, input_num: i32, source_name: String) {
        let mut state = self.shared_state.lock().unwrap();
        let request = InputChangeRequest::StartNdi { source_name: source_name.clone() };
        
        if input_num == 1 {
            state.input1_request = request;
        } else {
            state.input2_request = request;
        }
        
        log::info!("Selected NDI source '{}' for input {}", source_name, input_num);
    }
    
    /// Select OBS source for input
    fn select_obs(&mut self, input_num: i32, source_name: String) {
        let mut state = self.shared_state.lock().unwrap();
        let request = InputChangeRequest::StartObs { source_name: source_name.clone() };
        
        if input_num == 1 {
            state.input1_request = request;
        } else {
            state.input2_request = request;
        }
        
        log::info!("Selected OBS source '{}' for input {}", source_name, input_num);
    }
}
