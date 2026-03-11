// Video Wall Multi-Quad Shader
//
// This shader renders content across multiple display quads with
// perspective-correct UV mapping and per-display color adjustments.
// Each display has its own source rectangle, destination quad,
// and post-processing color controls.
//
// The shader uses a single-pass approach where each pixel checks
// which display quad it belongs to and samples accordingly.

// Maximum number of displays supported
const MAX_DISPLAYS: u32 = 16u;  // Supports up to 4x4 grid

// Display quad uniform data
// Aligned to 16 bytes for WGSL
struct DisplayQuadData {
    // Source rectangle in UV space (x, y, width, height)
    source_rect: vec4<f32>,
    // Destination corners: TL, TR, BR, BL
    dest_tl: vec2<f32>,
    dest_tr: vec2<f32>,
    dest_br: vec2<f32>,
    dest_bl: vec2<f32>,
    // Color adjustments (applied after sampling)
    brightness: f32,    // Multiplier (0.0 - 2.0)
    contrast: f32,      // Multiplier (0.0 - 2.0)
    gamma: f32,         // Exponent (0.1 - 3.0)
    enabled: u32,
}

// Uniforms updated per frame
struct VideoWallUniforms {
    // Number of active displays
    display_count: u32,
    // Output resolution
    output_width: f32,
    output_height: f32,
    // Background color (RGB, A)
    background_color: vec4<f32>,
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(1) @binding(0)
var<uniform> uniforms: VideoWallUniforms;

@group(1) @binding(1)
var<storage, read> displays: array<DisplayQuadData>;

// Vertex shader - generates fullscreen triangle
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Generate a fullscreen triangle that covers the entire viewport
    // vertex_index: 0 -> (-1, -1), 1 -> (3, -1), 2 -> (-1, 3)
    // This creates a large triangle that covers the screen
    // Use select to avoid integer division issues
    let x = select(-1.0, 3.0, vertex_index == 1u);
    let y = select(-1.0, 3.0, vertex_index == 2u);
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Fragment shader - sample appropriate display for each pixel
@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Convert pixel coordinates to UV space (0-1)
    let output_uv = frag_coord.xy / vec2<f32>(uniforms.output_width, uniforms.output_height);
    
    // Find which display this pixel belongs to
    for (var i: u32 = 0u; i < uniforms.display_count; i = i + 1u) {
        let display = displays[i];
        
        // Skip disabled displays
        if (display.enabled == 0u) {
            continue;
        }
        
        // Build corner array for point-in-quad test
        let corners = array<vec2<f32>, 4>(
            display.dest_tl,
            display.dest_tr,
            display.dest_br,
            display.dest_bl
        );
        
        // Check if pixel is inside this display's quad
        if (point_in_quad(output_uv, corners)) {
            // Map output UV to source UV with perspective correction
            let source_uv = perspective_map(output_uv, corners, display.source_rect);
            
            // Sample from source texture
            var color = textureSample(source_texture, source_sampler, source_uv);
            
            // Apply per-display color adjustments (post-sampling)
            color = apply_color_adjustments(color, display.brightness, display.contrast, display.gamma);
            
            return color;
        }
    }
    
    // No display covers this pixel - return background color
    return uniforms.background_color;
}

// Apply brightness, contrast, and gamma adjustments
fn apply_color_adjustments(
    color: vec4<f32>,
    brightness: f32,
    contrast: f32,
    gamma: f32
) -> vec4<f32> {
    // Apply to RGB channels only, preserve alpha
    var rgb = color.rgb;
    
    // Brightness: shift all values
    rgb = rgb * brightness;
    
    // Contrast: shift around midpoint (0.5)
    rgb = (rgb - 0.5) * contrast + 0.5;
    
    // Gamma: apply power function
    // Need to handle gamma < 0 to avoid NaN
    if (gamma > 0.0) {
        rgb = pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(gamma));
    }
    
    // Clamp to valid range
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    
    return vec4<f32>(rgb, color.a);
}

// Check if point is inside quad using barycentric coordinates
// Splits quad into two triangles and tests each
fn point_in_quad(p: vec2<f32>, quad: array<vec2<f32>, 4>) -> bool {
    // Split quad into two triangles:
    // Triangle 1: corners 0, 1, 2 (TL, TR, BR)
    // Triangle 2: corners 0, 2, 3 (TL, BR, BL)
    return point_in_triangle(p, quad[0], quad[1], quad[2]) ||
           point_in_triangle(p, quad[0], quad[2], quad[3]);
}

// Check if point is inside triangle using barycentric coordinates
fn point_in_triangle(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> bool {
    // Compute vectors
    let v0 = c - a;
    let v1 = b - a;
    let v2 = p - a;
    
    // Compute dot products
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    
    // Compute barycentric coordinates
    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    
    // Check if point is inside triangle
    return (u >= 0.0) && (v >= 0.0) && (u + v <= 1.0);
}

// Perspective-correct UV mapping from output quad to source rectangle
// Uses bilinear interpolation within the source rectangle based on
// the point's position within the destination quad
fn perspective_map(
    output_uv: vec2<f32>,
    dest_quad: array<vec2<f32>, 4>,
    source_rect: vec4<f32>,
) -> vec2<f32> {
    // Compute barycentric coordinates in the destination quad
    // We'll use the quadrilateral's natural parametrization
    
    // Find the barycentric coordinates using the diagonal split method
    // First try triangle 0-1-2
    let bary1 = barycentric_coords(output_uv, dest_quad[0], dest_quad[1], dest_quad[2]);
    
    if (bary1.x >= -0.0001 && bary1.y >= -0.0001 && bary1.z >= -0.0001) {
        // Point is in triangle 0-1-2
        // Map to source rectangle using bilinear interpolation
        return bilinear_sample(source_rect, bary1.y, bary1.x + bary1.y);
    }
    
    // Try triangle 0-2-3
    let bary2 = barycentric_coords(output_uv, dest_quad[0], dest_quad[2], dest_quad[3]);
    
    if (bary2.x >= -0.0001 && bary2.y >= -0.0001 && bary2.z >= -0.0001) {
        // Point is in triangle 0-2-3
        // Map to source rectangle using bilinear interpolation
        // Note: u coordinate is adjusted for second triangle
        return bilinear_sample(source_rect, bary2.y + bary2.z, bary2.y);
    }
    
    // Point is not in either triangle (shouldn't happen if point_in_quad passed)
    // Return center of source rect as fallback
    return vec2<f32>(
        source_rect.x + source_rect.z * 0.5,
        source_rect.y + source_rect.w * 0.5,
    );
}

// Compute barycentric coordinates of point p in triangle (a, b, c)
// Returns (u, v, w) where p = u*a + v*b + w*c and u+v+w = 1
fn barycentric_coords(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> vec3<f32> {
    let v0 = c - a;
    let v1 = b - a;
    let v2 = p - a;
    
    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);
    
    let denom = dot00 * dot11 - dot01 * dot01;
    
    if (abs(denom) < 0.0001) {
        // Degenerate triangle
        return vec3<f32>(1.0, 0.0, 0.0);
    }
    
    let inv_denom = 1.0 / denom;
    let v = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let w = (dot00 * dot12 - dot01 * dot02) * inv_denom;
    let u = 1.0 - v - w;
    
    return vec3<f32>(u, v, w);
}

// Bilinear interpolation within a rectangle
// u and v should be in range [0, 1]
fn bilinear_sample(rect: vec4<f32>, u: f32, v: f32) -> vec2<f32> {
    let x = rect.x + rect.z * clamp(u, 0.0, 1.0);
    let y = rect.y + rect.w * clamp(v, 0.0, 1.0);
    return vec2<f32>(x, y);
}
