//! # Input Module
//!
//! Handles multiple video input sources:
//! - Webcam capture (via nokhwa)
//! - NDI input (Network Device Interface)
//! - OBS (via NDI)
//!
//! Each input is independently selectable and refreshable.

use anyhow::Result;
use std::sync::mpsc;

// NDI input support
pub mod ndi;
pub use ndi::{NdiReceiver, NdiFrame, list_ndi_sources, is_ndi_available};

// Webcam support (optional)
#[cfg(feature = "webcam")]
pub mod webcam;
#[cfg(feature = "webcam")]
pub use webcam::{WebcamCapture, WebcamFrame, list_cameras};

// Placeholder types when webcam is disabled
#[cfg(not(feature = "webcam"))]
pub struct WebcamFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp: std::time::Instant,
}

#[cfg(not(feature = "webcam"))]
pub struct WebcamCapture;

#[cfg(not(feature = "webcam"))]
impl WebcamCapture {
    pub fn new(_device_index: usize, _width: u32, _height: u32, _fps: u32) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Webcam support not compiled. Enable the 'webcam' feature."))
    }
    
    pub fn start(&mut self) -> anyhow::Result<mpsc::Receiver<WebcamFrame>> {
        unreachable!()
    }
    
    pub fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(not(feature = "webcam"))]
pub fn list_cameras() -> Vec<String> {
    Vec::new()
}

/// Type of input source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    None,
    Webcam,
    Ndi,
    Obs,  // OBS via NDI
}

impl InputType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            InputType::None => "None",
            InputType::Webcam => "Webcam",
            InputType::Ndi => "NDI",
            InputType::Obs => "OBS (NDI)",
        }
    }
}

/// Information about available input devices
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    pub index: usize,
    pub name: String,
    pub device_type: InputType,
}

/// Individual input source with its own capture
pub struct InputSource {
    pub input_type: InputType,
    pub device_index: i32,
    pub source_name: String,
    pub resolution: (u32, u32),
    pub active: bool,
    pub has_new_frame: bool,
    
    // Capture instances
    #[cfg(feature = "webcam")]
    webcam: Option<WebcamCapture>,
    #[cfg(not(feature = "webcam"))]
    webcam: Option<()>,
    
    frame_receiver: Option<mpsc::Receiver<WebcamFrame>>,
    ndi_receiver: Option<NdiReceiver>,
    
    // Current frame data
    current_frame: Option<Vec<u8>>,
}

impl InputSource {
    pub fn new() -> Self {
        Self {
            input_type: InputType::None,
            device_index: -1,
            source_name: String::new(),
            resolution: (1920, 1080),
            active: false,
            has_new_frame: false,
            webcam: None,
            frame_receiver: None,
            ndi_receiver: None,
            current_frame: None,
        }
    }
    
    /// Start webcam capture
    #[cfg(feature = "webcam")]
    pub fn start_webcam(&mut self, device_index: usize, width: u32, height: u32, fps: u32) -> Result<()> {
        self.stop();
        
        let mut webcam = WebcamCapture::new(device_index, width, height, fps)?;
        let receiver = webcam.start()?;
        
        self.input_type = InputType::Webcam;
        self.device_index = device_index as i32;
        self.resolution = (width, height);
        self.active = true;
        self.webcam = Some(webcam);
        self.frame_receiver = Some(receiver);
        
        log::info!("Started webcam {} at {}x{}@{}fps", device_index, width, height, fps);
        
        Ok(())
    }
    
    /// Start webcam (placeholder when disabled)
    #[cfg(not(feature = "webcam"))]
    pub fn start_webcam(&mut self, _device_index: usize, _width: u32, _height: u32, _fps: u32) -> Result<()> {
        Err(anyhow::anyhow!("Webcam support not compiled. Enable the 'webcam' feature."))
    }
    
    /// Start NDI receiver
    pub fn start_ndi(&mut self, source_name: impl Into<String>) -> Result<()> {
        self.stop();
        
        let source_name = source_name.into();
        let mut ndi = NdiReceiver::new(source_name.clone());
        ndi.start()?;
        
        self.input_type = InputType::Ndi;
        self.source_name = source_name.clone();
        self.active = true;
        self.ndi_receiver = Some(ndi);
        
        log::info!("Started NDI input: {}", source_name);
        
        Ok(())
    }
    
    /// Start OBS (via NDI)
    pub fn start_obs(&mut self, source_name: impl Into<String>) -> Result<()> {
        // OBS via NDI uses the same mechanism as regular NDI
        self.start_ndi(source_name)?;
        self.input_type = InputType::Obs;
        Ok(())
    }
    
    /// Stop the input source
    pub fn stop(&mut self) {
        if !self.active {
            return;
        }
        
        log::info!("Stopping input source ({:?})", self.input_type);
        
        self.active = false;
        self.has_new_frame = false;
        
        // Stop webcam
        #[cfg(feature = "webcam")]
        if let Some(mut webcam) = self.webcam.take() {
            let _ = webcam.stop();
        }
        
        // Stop NDI
        if let Some(mut ndi) = self.ndi_receiver.take() {
            ndi.stop();
        }
        
        self.frame_receiver = None;
        self.current_frame = None;
        self.input_type = InputType::None;
        self.device_index = -1;
        self.source_name.clear();
    }
    
    /// Update - check for new frames
    pub fn update(&mut self) {
        if !self.active {
            return;
        }
        
        // Handle webcam frames
        if let Some(ref receiver) = self.frame_receiver {
            let mut latest_frame: Option<WebcamFrame> = None;
            
            // Drain the channel (keep only latest)
            while let Ok(frame) = receiver.try_recv() {
                latest_frame = Some(frame);
            }
            
            if let Some(frame) = latest_frame {
                self.resolution = (frame.width, frame.height);
                self.current_frame = Some(frame.data);
                self.has_new_frame = true;
            }
        }
        
        // Handle NDI frames
        if let Some(ref mut ndi) = self.ndi_receiver {
            if let Some(frame) = ndi.get_latest_frame() {
                self.resolution = (frame.width, frame.height);
                self.current_frame = Some(frame.data);
                self.has_new_frame = true;
            }
        }
    }
    
    /// Check if there's a new frame
    pub fn has_frame(&self) -> bool {
        self.has_new_frame
    }
    
    /// Take the current frame (consumes it)
    pub fn take_frame(&mut self) -> Option<Vec<u8>> {
        self.has_new_frame = false;
        self.current_frame.take()
    }
    
    /// Get current resolution
    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }
    
    /// Check if input is active
    pub fn is_active(&self) -> bool {
        self.active
    }
    
    /// Get input type
    pub fn input_type(&self) -> InputType {
        self.input_type
    }
    
    /// Get source name/display info
    pub fn source_info(&self) -> String {
        match self.input_type {
            InputType::None => "None".to_string(),
            InputType::Webcam => format!("Webcam {}", self.device_index),
            InputType::Ndi | InputType::Obs => self.source_name.clone(),
        }
    }
}

impl Drop for InputSource {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Manages multiple input sources
pub struct InputManager {
    /// Input 1 (primary)
    pub input1: InputSource,
    /// Input 2 (secondary)
    pub input2: InputSource,
    
    // Device lists
    webcam_devices: Vec<String>,
    ndi_sources: Vec<String>,
    
    // Refresh flags
    webcam_dirty: bool,
    ndi_dirty: bool,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            input1: InputSource::new(),
            input2: InputSource::new(),
            webcam_devices: Vec::new(),
            ndi_sources: Vec::new(),
            webcam_dirty: true,
            ndi_dirty: true,
        }
    }
    
    /// Refresh webcam device list
    pub fn refresh_webcam_devices(&mut self) -> Vec<String> {
        #[cfg(feature = "webcam")]
        {
            self.webcam_devices = std::panic::catch_unwind(|| {
                list_cameras()
            }).unwrap_or_else(|_| {
                log::error!("Webcam enumeration panicked");
                Vec::new()
            });
        }
        
        #[cfg(not(feature = "webcam"))]
        {
            self.webcam_devices = Vec::new();
        }
        
        self.webcam_dirty = false;
        log::info!("Found {} webcam devices", self.webcam_devices.len());
        
        self.webcam_devices.clone()
    }
    
    /// Refresh NDI sources
    pub fn refresh_ndi_sources(&mut self) -> Vec<String> {
        self.ndi_sources = list_ndi_sources(2000);
        self.ndi_dirty = false;
        log::info!("Found {} NDI sources", self.ndi_sources.len());
        
        self.ndi_sources.clone()
    }
    
    /// Get cached webcam devices (refresh if needed)
    pub fn get_webcam_devices(&mut self) -> Vec<String> {
        if self.webcam_dirty {
            self.refresh_webcam_devices()
        } else {
            self.webcam_devices.clone()
        }
    }
    
    /// Get cached NDI sources (refresh if needed)
    pub fn get_ndi_sources(&mut self) -> Vec<String> {
        if self.ndi_dirty {
            self.refresh_ndi_sources()
        } else {
            self.ndi_sources.clone()
        }
    }
    
    /// Mark devices as needing refresh
    pub fn invalidate_devices(&mut self) {
        self.webcam_dirty = true;
        self.ndi_dirty = true;
    }
    
    /// Update all inputs (poll for new frames)
    pub fn update(&mut self) {
        self.input1.update();
        self.input2.update();
    }
    
    /// Start webcam on input 1
    pub fn start_input1_webcam(&mut self, device_index: usize, width: u32, height: u32, fps: u32) -> Result<()> {
        self.input1.start_webcam(device_index, width, height, fps)
    }
    
    /// Start webcam on input 2
    pub fn start_input2_webcam(&mut self, device_index: usize, width: u32, height: u32, fps: u32) -> Result<()> {
        self.input2.start_webcam(device_index, width, height, fps)
    }
    
    /// Start NDI on input 1
    pub fn start_input1_ndi(&mut self, source_name: impl Into<String>) -> Result<()> {
        self.input1.start_ndi(source_name)
    }
    
    /// Start NDI on input 2
    pub fn start_input2_ndi(&mut self, source_name: impl Into<String>) -> Result<()> {
        self.input2.start_ndi(source_name)
    }
    
    /// Start OBS on input 1
    pub fn start_input1_obs(&mut self, source_name: impl Into<String>) -> Result<()> {
        self.input1.start_obs(source_name)
    }
    
    /// Start OBS on input 2
    pub fn start_input2_obs(&mut self, source_name: impl Into<String>) -> Result<()> {
        self.input2.start_obs(source_name)
    }
    
    /// Stop input 1
    pub fn stop_input1(&mut self) {
        self.input1.stop();
    }
    
    /// Stop input 2
    pub fn stop_input2(&mut self) {
        self.input2.stop();
    }
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}
