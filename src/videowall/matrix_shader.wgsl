// Video Matrix Shader
// Maps input grid cells to output positions with orientation handling

// Cell mapping structure (must match Rust struct)
struct CellMapping {
    source_rect: vec4<f32>,  // x, y, width, height
    dest_rect: vec4<f32>,    // x, y, width, height
    orientation: u32,        // 0=0°, 1=90°, 2=180°, 3=270°
    aspect_ratio: f32,
    enabled: u32,
    _padding: u32,
}

// Matrix uniforms (must match Rust struct exactly - 96 bytes total)
// Avoid vec3 - use explicit u32 padding for predictable alignment
struct MatrixUniforms {
    // First 16 bytes (0-15)
    mapping_count: u32,      // 0
    input_cols: u32,         // 4
    input_rows: u32,         // 8
    output_cols: u32,        // 12
    
    // Second 16 bytes (16-31)
    output_rows: u32,        // 16
    output_width: u32,       // 20 - actual pixel width for UV calculation
    output_height: u32,      // 24 - actual pixel height for UV calculation
    _padding1: u32,          // 28
    
    // Third 16 bytes (32-47)
    _padding2_0: u32,        // 32
    _padding2_1: u32,        // 36
    _padding2_2: u32,        // 40
    _align_pad0: u32,        // 44
    
    // Fourth 16 bytes (48-63) - background_color at 16-byte boundary
    background_color: vec4<f32>,  // 48-63
    
    // Fifth 16 bytes (64-79)
    _final_pad0: u32,        // 64
    _final_pad1: u32,        // 68
    _final_pad2: u32,        // 72
    _final_pad3: u32,        // 76
    
    // Sixth 16 bytes (80-95) - ensure struct is 96 bytes
    _final_pad4: u32,        // 80
    _final_pad5: u32,        // 84
    _final_pad6: u32,        // 88
    _final_pad7: u32,        // 92
}

// Uniform bindings
@group(1) @binding(0)
var<uniform> uniforms: MatrixUniforms;

@group(1) @binding(1)
var<storage, read> mappings: array<CellMapping, 16>;

// Texture bindings
@group(0) @binding(0)
var source_tex: texture_2d<f32>;
@group(0) @binding(1)
var source_sampler: sampler;

// Vertex shader - fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Generate fullscreen triangle using vertex_index
    // vertex 0: (-1, -1), vertex 1: (3, -1), vertex 2: (-1, 3)
    // x: -1, 3, -1  ->  vertex_index * 2 - 1 gives -1, 1, 3 (wrong for middle)
    // Use select for correct values:
    let x = select(-1.0, 3.0, vertex_index == 1u);
    let y = select(-1.0, 3.0, vertex_index == 2u);
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Apply orientation to UV coordinates
fn apply_orientation(uv: vec2<f32>, orientation: u32) -> vec2<f32> {
    switch orientation {
        case 0u: { // Normal (0°)
            return uv;
        }
        case 1u: { // Rotated 90° CW
            // Top-left (0,0) -> Top-right (1,0)
            // Top-right (1,0) -> Bottom-right (1,1)
            return vec2<f32>(1.0 - uv.y, uv.x);
        }
        case 2u: { // Rotated 180°
            return vec2<f32>(1.0 - uv.x, 1.0 - uv.y);
        }
        case 3u: { // Rotated 270° CW (or 90° CCW)
            return vec2<f32>(uv.y, 1.0 - uv.x);
        }
        default: {
            return uv;
        }
    }
}

// Find mapping for current output position
fn find_mapping_for_output(output_uv: vec2<f32>) -> i32 {
    for (var i: i32 = 0; i < i32(uniforms.mapping_count); i = i32(i) + 1) {
        let mapping = mappings[i];
        
        if (mapping.enabled == 0u) {
            continue;
        }
        
        // Check if output UV is within this mapping's destination rect
        let dest_min = mapping.dest_rect.xy;
        let dest_max = dest_min + mapping.dest_rect.zw;
        
        if (output_uv.x >= dest_min.x && output_uv.x < dest_max.x &&
            output_uv.y >= dest_min.y && output_uv.y < dest_max.y) {
            return i;
        }
    }
    
    return -1; // No mapping found
}

// Fragment shader
@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Calculate output UV (0-1) based on actual output resolution
    let output_width = f32(uniforms.output_width);
    let output_height = f32(uniforms.output_height);
    let output_uv = frag_coord.xy / vec2<f32>(output_width, output_height);
    
    // Find mapping for this output position
    let mapping_idx = find_mapping_for_output(output_uv);
    
    if (mapping_idx < 0) {
        // No mapping - return background color (black)
        return uniforms.background_color;
    }
    
    let mapping = mappings[mapping_idx];
    
    // Calculate UV within the destination rectangle
    let dest_min = mapping.dest_rect.xy;
    let dest_size = mapping.dest_rect.zw;
    var local_uv = (output_uv - dest_min) / dest_size;
    
    // Clamp to prevent sampling outside bounds
    local_uv = clamp(local_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    
    // Apply orientation transform
    let oriented_uv = apply_orientation(local_uv, mapping.orientation);
    
    // Calculate source UV within the source rectangle
    let source_min = mapping.source_rect.xy;
    let source_size = mapping.source_rect.zw;
    let source_uv = source_min + oriented_uv * source_size;
    
    // Sample from source texture
    let color = textureSample(source_tex, source_sampler, source_uv);
    
    return color;
}
