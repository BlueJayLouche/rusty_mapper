//! # Rendering Engine
//!
//! wgpu-based GPU rendering pipeline for video processing.

pub mod renderer;
pub mod texture;

pub use renderer::WgpuEngine;
pub use texture::Texture;
