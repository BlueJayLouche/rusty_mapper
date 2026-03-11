//! # Test Pattern Generator
//!
//! Generates test patterns for verifying video wall mapping and calibration.
//! Useful for alignment checks before using actual content.

use image::{RgbaImage, Rgba};

/// Test pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPattern {
    /// Color bars (SMPTEx color bars)
    ColorBars,
    /// Grid pattern with crosshair
    Grid,
    /// Numbered displays (shows display ID)
    Numbered,
    /// Checkerboard pattern
    Checkerboard,
    /// Gradient pattern
    Gradient,
}

impl TestPattern {
    /// Get pattern name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ColorBars => "Color Bars",
            Self::Grid => "Grid",
            Self::Numbered => "Numbered",
            Self::Checkerboard => "Checkerboard",
            Self::Gradient => "Gradient",
        }
    }

    /// Generate the pattern for a single display
    pub fn generate(&self, width: u32, height: u32, display_id: u32, total_displays: u32) -> RgbaImage {
        match self {
            Self::ColorBars => generate_color_bars(width, height),
            Self::Grid => generate_grid(width, height, display_id),
            Self::Numbered => generate_numbered(width, height, display_id, total_displays),
            Self::Checkerboard => generate_checkerboard(width, height, display_id),
            Self::Gradient => generate_gradient(width, height, display_id),
        }
    }

    /// Generate full frame for all displays in a grid
    pub fn generate_full_frame(
        &self,
        grid_size: (u32, u32),
        output_resolution: (u32, u32),
    ) -> RgbaImage {
        let (cols, rows) = grid_size;
        let (width, height) = output_resolution;
        let display_width = width / cols;
        let display_height = height / rows;
        let total_displays = cols * rows;

        let mut frame = RgbaImage::new(width, height);

        for id in 0..total_displays {
            let col = id % cols;
            let row = id / cols;
            let offset_x = col * display_width;
            let offset_y = row * display_height;

            let pattern = self.generate(display_width, display_height, id, total_displays);

            // Copy pattern into frame
            for y in 0..display_height {
                for x in 0..display_width {
                    frame.put_pixel(offset_x + x, offset_y + y, *pattern.get_pixel(x, y));
                }
            }
        }

        frame
    }
}

impl Default for TestPattern {
    fn default() -> Self {
        Self::Numbered
    }
}

/// Generate SMPTE color bars pattern
fn generate_color_bars(width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let bar_width = width / 7;
    
    // SMPTE color bar colors (simplified)
    let colors = [
        Rgba([191, 191, 191, 255]), // White
        Rgba([191, 191, 0, 255]),   // Yellow
        Rgba([0, 191, 191, 255]),   // Cyan
        Rgba([0, 191, 0, 255]),     // Green
        Rgba([191, 0, 191, 255]),   // Magenta
        Rgba([191, 0, 0, 255]),     // Red
        Rgba([0, 0, 191, 255]),     // Blue
    ];

    for y in 0..height {
        for x in 0..width {
            let bar_idx = (x / bar_width).min(6) as usize;
            img.put_pixel(x, y, colors[bar_idx]);
        }
    }

    img
}

/// Generate grid pattern with crosshair
fn generate_grid(width: u32, height: u32, display_id: u32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    
    // Grid spacing
    let grid_size = 50u32;
    let center_x = width / 2;
    let center_y = height / 2;

    // Draw grid lines
    for x in (0..width).step_by(grid_size as usize) {
        for y in 0..height {
            img.put_pixel(x, y, Rgba([64, 64, 64, 255]));
        }
    }
    for y in (0..height).step_by(grid_size as usize) {
        for x in 0..width {
            img.put_pixel(x, y, Rgba([64, 64, 64, 255]));
        }
    }

    // Draw crosshair at center
    for x in 0..width {
        img.put_pixel(x, center_y, Rgba([255, 255, 255, 255]));
    }
    for y in 0..height {
        img.put_pixel(center_x, y, Rgba([255, 255, 255, 255]));
    }

    // Draw display ID in corners
    let id_str = format!("{}", display_id);
    // Simple pixel-based text (just dots for now)
    draw_simple_text(&mut img, &id_str, 10, 10, Rgba([255, 255, 0, 255]));

    img
}

/// Generate numbered display pattern (shows large display ID)
fn generate_numbered(width: u32, height: u32, display_id: u32, _total_displays: u32) -> RgbaImage {
    // Background color based on display ID (helps identify)
    let hue = ((display_id * 60) % 360) as f32;
    let bg_color = hsl_to_rgba(hue, 0.5, 0.3);
    let mut img = RgbaImage::from_pixel(width, height, bg_color);

    // Draw border
    let border = 10u32;
    for x in 0..width {
        for y in 0..border {
            img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            img.put_pixel(x, height - 1 - y, Rgba([255, 255, 255, 255]));
        }
    }
    for y in border..(height - border) {
        for x in 0..border {
            img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            img.put_pixel(width - 1 - x, y, Rgba([255, 255, 255, 255]));
        }
    }

    // Draw display ID as large text in center
    let id_str = format!("{}", display_id);
    let text_color = Rgba([255, 255, 255, 255]);
    
    // Center position
    let center_x = width / 2;
    let center_y = height / 2;
    
    // Draw simple large numbers
    draw_large_number(&mut img, display_id, center_x, center_y, text_color);

    img
}

/// Generate checkerboard pattern
fn generate_checkerboard(width: u32, height: u32, display_id: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    let check_size = 40u32;
    
    // Alternate colors based on display ID
    let color1 = if display_id % 2 == 0 {
        Rgba([255, 255, 255, 255])
    } else {
        Rgba([200, 200, 200, 255])
    };
    let color2 = Rgba([0, 0, 0, 255]);

    for y in 0..height {
        for x in 0..width {
            let check_x = x / check_size;
            let check_y = y / check_size;
            let is_even = (check_x + check_y) % 2 == 0;
            img.put_pixel(x, y, if is_even { color1 } else { color2 });
        }
    }

    img
}

/// Generate gradient pattern
fn generate_gradient(width: u32, height: u32, display_id: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    
    // Different gradient direction per display
    for y in 0..height {
        for x in 0..width {
            let t = match display_id % 4 {
                0 => x as f32 / width as f32,                    // Left to right
                1 => y as f32 / height as f32,                   // Top to bottom
                2 => (x + y) as f32 / (width + height) as f32,   // Diagonal
                _ => ((x as f32 / width as f32) + (y as f32 / height as f32)) / 2.0,
            };
            
            let value = (t * 255.0) as u8;
            img.put_pixel(x, y, Rgba([value, value, value, 255]));
        }
    }

    img
}

/// Simple HSL to RGB conversion
fn hsl_to_rgba(h: f32, s: f32, l: f32) -> Rgba<u8> {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match (h / 60.0) as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    Rgba([
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
        255,
    ])
}

/// Draw simple text using block characters
fn draw_simple_text(img: &mut RgbaImage, text: &str, x: u32, y: u32, color: Rgba<u8>) {
    // Simple 3x5 font for digits
    let digits: std::collections::HashMap<char, [[u8; 3]; 5]> = [
        ('0', [[1,1,1],[1,0,1],[1,0,1],[1,0,1],[1,1,1]]),
        ('1', [[0,1,0],[1,1,0],[0,1,0],[0,1,0],[1,1,1]]),
        ('2', [[1,1,1],[0,0,1],[1,1,1],[1,0,0],[1,1,1]]),
        ('3', [[1,1,1],[0,0,1],[1,1,1],[0,0,1],[1,1,1]]),
        ('4', [[1,0,1],[1,0,1],[1,1,1],[0,0,1],[0,0,1]]),
        ('5', [[1,1,1],[1,0,0],[1,1,1],[0,0,1],[1,1,1]]),
        ('6', [[1,1,1],[1,0,0],[1,1,1],[1,0,1],[1,1,1]]),
        ('7', [[1,1,1],[0,0,1],[0,1,0],[0,1,0],[0,1,0]]),
        ('8', [[1,1,1],[1,0,1],[1,1,1],[1,0,1],[1,1,1]]),
        ('9', [[1,1,1],[1,0,1],[1,1,1],[0,0,1],[1,1,1]]),
    ].into_iter().collect();

    let scale = 2;
    let mut offset_x = x;

    for ch in text.chars() {
        if let Some(pattern) = digits.get(&ch) {
            for (row_idx, row) in pattern.iter().enumerate() {
                for (col_idx, &pixel) in row.iter().enumerate() {
                    if pixel == 1 {
                        for dy in 0..scale {
                            for dx in 0..scale {
                                let px = offset_x + col_idx as u32 * scale + dx;
                                let py = y + row_idx as u32 * scale + dy;
                                if px < img.width() && py < img.height() {
                                    img.put_pixel(px, py, color);
                                }
                            }
                        }
                    }
                }
            }
        }
        offset_x += 4 * scale;
    }
}

/// Draw a large number in the center of the image
fn draw_large_number(img: &mut RgbaImage, num: u32, center_x: u32, center_y: u32, color: Rgba<u8>) {
    let num_str = format!("{}", num);
    let scale = 8;
    let char_width = 4 * scale;
    let total_width = num_str.len() as u32 * char_width;
    let start_x = center_x.saturating_sub(total_width / 2);
    let start_y = center_y.saturating_sub(10);

    draw_simple_text(img, &num_str, start_x, start_y, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_bars_generation() {
        let img = TestPattern::ColorBars.generate(1920, 1080, 0, 1);
        assert_eq!(img.width(), 1920);
        assert_eq!(img.height(), 1080);
    }

    #[test]
    fn test_numbered_generation() {
        let img = TestPattern::Numbered.generate(1920, 1080, 5, 9);
        assert_eq!(img.width(), 1920);
        assert_eq!(img.height(), 1080);
    }

    #[test]
    fn test_full_frame_generation() {
        let img = TestPattern::Numbered.generate_full_frame((3, 3), (1920, 1080));
        assert_eq!(img.width(), 1920);
        assert_eq!(img.height(), 1080);
    }

    #[test]
    fn test_pattern_names() {
        assert_eq!(TestPattern::ColorBars.name(), "Color Bars");
        assert_eq!(TestPattern::Grid.name(), "Grid");
        assert_eq!(TestPattern::Numbered.name(), "Numbered");
    }
}
