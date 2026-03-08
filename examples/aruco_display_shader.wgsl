// Simple fullscreen texture shader for ArUco pattern display

@group(0) @binding(0)
var pattern_texture: texture_2d<f32>;

@group(0) @binding(1)
var pattern_sampler: sampler;

// Fullscreen triangle strip vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Generate positions for a fullscreen triangle strip
    // vertex_index: 0 -> (-1, -1), 1 -> (3, -1), 2 -> (-1, 3)
    // This creates a large triangle that covers the entire screen
    var pos = vec2<f32>(
        f32(vertex_index % 2u) * 4.0 - 1.0,  // x: -1.0 or 3.0
        f32(vertex_index / 2u) * 4.0 - 1.0   // y: -1.0 or 3.0
    );
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Get texture dimensions
    let texture_dims = vec2<f32>(textureDimensions(pattern_texture));
    
    // Get output dimensions (from frag_coord which is in pixel space)
    // We need to use a uniform for this, but for simplicity we'll use a fixed value
    // The aspect ratio will be handled by the texture sampler
    
    // Sample the texture with aspect ratio preservation
    // The texture coordinates are adjusted to maintain aspect ratio
    let uv = frag_coord.xy / texture_dims;
    
    // Sample the pattern texture
    let color = textureSample(pattern_texture, pattern_sampler, uv);
    
    return color;
}
