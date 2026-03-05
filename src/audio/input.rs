//! # Audio Input
//!
//! Audio capture and FFT analysis.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::{Arc, Mutex};

/// Audio input handler with FFT analysis
pub struct AudioInput {
    fft_size: usize,
    fft_planner: FftPlanner<f32>,
    fft_buffer: Vec<Complex<f32>>,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    stream: Option<cpal::Stream>,
    amplitude: f32,
    smoothing: f32,
}

impl AudioInput {
    pub fn new(fft_size: usize) -> Self {
        let fft_planner = FftPlanner::new();
        let fft_buffer = vec![Complex::new(0.0, 0.0); fft_size];
        let sample_buffer = Arc::new(Mutex::new(Vec::with_capacity(fft_size)));
        
        Self {
            fft_size,
            fft_planner,
            fft_buffer,
            sample_buffer,
            stream: None,
            amplitude: 1.0,
            smoothing: 0.5,
        }
    }
    
    /// Initialize default audio input
    pub fn initialize(&mut self) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
        
        let config = device.default_input_config()?;
        
        log::info!("Audio input: {:?}", config);
        
        let sample_buffer = Arc::clone(&self.sample_buffer);
        let fft_size = self.fft_size;
        
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = sample_buffer.lock().unwrap();
                for &sample in data {
                    buffer.push(sample);
                    if buffer.len() >= fft_size {
                        // Process FFT here or in update()
                        buffer.clear();
                    }
                }
            },
            move |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )?;
        
        stream.play()?;
        self.stream = Some(stream);
        
        Ok(())
    }
    
    /// Get 8-band FFT values (normalized)
    pub fn get_8band_fft(&self) -> [f32; 8] {
        // Placeholder - would compute FFT and bin into 8 bands
        [0.0; 8]
    }
    
    /// Set amplitude multiplier
    pub fn set_amplitude(&mut self, amp: f32) {
        self.amplitude = amp;
    }
    
    /// Set smoothing factor
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.smoothing = smoothing;
    }
}
