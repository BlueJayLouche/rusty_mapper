// Main shader for video processing with projection mapping support
// Features: Corner pinning, UV transforms, multiple blend modes

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
};

struct MappingParams {
    // Corners: [tl_x, tl_y, tr_x, tr_y, br_x, br_y, bl_x, bl_y]
    corners: vec4<f32>,  // Packed as vec4s for alignment
    corners2: vec4<f32>,
    // Transform: scale_x, scale_y, offset_x, offset_y
    transform: vec4<f32>,
    // Rotation (radians), opacity, blend_mode, _padding
    settings: vec4<f32>,
};

@group(0) @binding(0)
var input1_tex: texture_2d<f32>;
@group(0) @binding(1)
var input1_sampler: sampler;
@group(0) @binding(2)
var input2_tex: texture_2d<f32>;
@group(0) @binding(3)
var input2_sampler: sampler;

// Uniforms for mapping
@group(1) @binding(0)
var<uniform> input1_mapping: MappingParams;
@group(1) @binding(1)
var<uniform> input2_mapping: MappingParams;
@group(1) @binding(2)
var<uniform> mix_settings: vec4<f32>; // mix_amount, _padding, _padding, _padding

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.texcoord = texcoord;
    return out;
}

/// Compute barycentric coordinates for quad interpolation
/// Used for corner pinning (perspective-correct interpolation)
fn compute_barycentric(uv: vec2<f32>, corners: array<vec2<f32>, 4>) -> vec4<f32> {
    // Using bilinear interpolation for corner pinning
    // Corners are: 0=top-left, 1=top-right, 2=bottom-right, 3=bottom-left
    
    // Convert to 0-1 range based on current UV position
    let x = uv.x;
    let y = uv.y;
    
    // Bilinear weights
    let w0 = (1.0 - x) * (1.0 - y);  // top-left
    let w1 = x * (1.0 - y);          // top-right
    let w2 = x * y;                  // bottom-right
    let w3 = (1.0 - x) * y;          // bottom-left
    
    return vec4<f32>(w0, w1, w2, w3);
}

/// Apply corner pinning transformation
fn apply_corner_pin(uv: vec2<f32>, mapping: MappingParams) -> vec2<f32> {
    // Unpack corners
    let corners = array<vec2<f32>, 4>(
        mapping.corners.xy,
        mapping.corners.zw,
        mapping.corners2.xy,
        mapping.corners2.zw
    );
    
    // Compute barycentric coordinates
    let weights = compute_barycentric(uv, corners);
    
    // Interpolate to get warped UV
    var warped_uv = 
        corners[0] * weights.x +
        corners[1] * weights.y +
        corners[2] * weights.z +
        corners[3] * weights.w;
    
    // Apply scale
    warped_uv = warped_uv * mapping.transform.xy;
    
    // Apply rotation
    let rotation = mapping.settings.x;
    if (rotation != 0.0) {
        let cos_r = cos(rotation);
        let sin_r = sin(rotation);
        let centered = warped_uv - 0.5;
        warped_uv = vec2<f32>(
            centered.x * cos_r - centered.y * sin_r,
            centered.x * sin_r + centered.y * cos_r
        ) + 0.5;
    }
    
    // Apply offset
    warped_uv = warped_uv + mapping.transform.zw;
    
    return warped_uv;
}

/// Apply blend mode
fn apply_blend_mode(base: vec4<f32>, blend: vec4<f32>, mode: i32, amount: f32) -> vec4<f32> {
    let base_rgb = base.rgb;
    let blend_rgb = blend.rgb;
    var result: vec3<f32>;
    
    switch mode {
        case 1: { // Add
            result = base_rgb + blend_rgb;
        }
        case 2: { // Multiply
            result = base_rgb * blend_rgb;
        }
        case 3: { // Screen
            result = 1.0 - (1.0 - base_rgb) * (1.0 - blend_rgb);
        }
        default: { // Normal
            result = mix(base_rgb, blend_rgb, amount);
        }
    }
    
    let alpha = mix(base.a, blend.a, amount);
    return vec4<f32>(result, alpha);
}

/// Sample input with mapping
fn sample_input(tex: texture_2d<f32>, smp: sampler, uv: vec2<f32>, mapping: MappingParams) -> vec4<f32> {
    let warped_uv = apply_corner_pin(uv, mapping);
    
    // Check if UV is in valid range (0-1)
    // If outside, return transparent/black
    if (warped_uv.x < 0.0 || warped_uv.x > 1.0 || 
        warped_uv.y < 0.0 || warped_uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    
    var color = textureSample(tex, smp, warped_uv);
    
    // Apply opacity
    color.a = color.a * mapping.settings.y;
    
    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample both inputs with their mappings
    let color1 = sample_input(input1_tex, input1_sampler, in.texcoord, input1_mapping);
    let color2 = sample_input(input2_tex, input2_sampler, in.texcoord, input2_mapping);
    
    // Get mix amount from uniform
    let mix_amount = mix_settings.x;
    
    // Determine blend mode from input2 (secondary input)
    let blend_mode = i32(input2_mapping.settings.z);
    
    // Mix the two inputs
    var result: vec4<f32>;
    
    // If input2 is not active (alpha = 0), just show input1
    if (color2.a < 0.01) {
        result = color1;
    } else if (color1.a < 0.01) {
        // If input1 is not active, show input2
        result = color2;
    } else {
        // Both active - apply blend mode
        result = apply_blend_mode(color1, color2, blend_mode, mix_amount * color2.a);
    }
    
    return result;
}
