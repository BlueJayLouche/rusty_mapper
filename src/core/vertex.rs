//! # Vertex Definitions
//!
//! GPU vertex format for quad rendering.

use bytemuck::{Pod, Zeroable};

/// Size of a vertex in bytes
pub const VERTEX_SIZE: usize = 16; // 4 floats * 4 bytes

/// Vertex for quad rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Position in normalized device coordinates (-1 to 1)
    pub position: [f32; 2],
    /// Texture coordinates (0 to 1)
    pub texcoord: [f32; 2],
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: [f32; 2], texcoord: [f32; 2]) -> Self {
        Self { position, texcoord }
    }
    
    /// Get vertices for a full-screen quad
    /// 
    /// Returns 6 vertices forming two triangles:
    /// ```
    /// (-1, 1)  (1, 1)
    ///     ┌─────┐
    ///     │    /│
    ///     │   / │
    ///     │  /  │
    ///     │ /   │
    ///     └─────┘
    /// (-1,-1)  (1,-1)
    /// ```
    pub fn quad_vertices() -> [Self; 6] {
        [
            // First triangle (top-left, top-right, bottom-left)
            Vertex::new([-1.0, 1.0], [0.0, 0.0]),   // Top-left
            Vertex::new([1.0, 1.0], [1.0, 0.0]),    // Top-right
            Vertex::new([-1.0, -1.0], [0.0, 1.0]),  // Bottom-left
            // Second triangle (top-right, bottom-right, bottom-left)
            Vertex::new([1.0, 1.0], [1.0, 0.0]),    // Top-right
            Vertex::new([1.0, -1.0], [1.0, 1.0]),   // Bottom-right
            Vertex::new([-1.0, -1.0], [0.0, 1.0]),  // Bottom-left
        ]
    }
    
    /// Get the wgpu vertex buffer layout
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
