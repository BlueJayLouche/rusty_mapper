//! # AprilTag Auto-Detection for Video Matrix
//!
//! Automatically detects screens in the input image using AprilTag markers.
//! Calculates screen regions, aspect ratios, and orientations from marker positions.
//!
//! ## Detection Strategy
//!
//! 1. Display AprilTags on each screen (centered or corner)
//! 2. Detect tags in the input image
//! 3. For each tag, calculate the screen region based on:
//!    - Tag center position
//!    - Known screen aspect ratio (from config or auto-detected)
//!    - Tag size relative to screen
//! 4. Determine orientation from tag rotation
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use rusty_mapper::videowall::{AprilTagAutoDetector, VideoMatrixConfig};
//!
//! // Detect screens from input image
//! let detector = AprilTagAutoDetector::new();
//! let detections = detector.detect_screens(&input_image, 2)?; // Expect 2 screens
//!
//! // Create video matrix config from detections
//! let config = detector.create_matrix_config(&detections, (1920, 1080))?;
//! ```

use super::{
    AprilTagDetection, AprilTagDetector, AprilTagFamily, AspectRatio, GridCellMapping,
    GridPosition, GridSize, InputGridConfig, Orientation, VideoMatrixConfig,
};
use glam::Vec2;
use image::GrayImage;

/// Tag placement strategy on screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagPlacement {
    /// Tag is centered on screen (default for test patterns)
    Centered,
    /// Tag is at top-left corner of screen
    TopLeft,
    /// Tag is at top-right corner of screen
    TopRight,
    /// Tag is at bottom-left corner of screen
    BottomLeft,
    /// Tag is at bottom-right corner of screen
    BottomRight,
}

impl Default for TagPlacement {
    fn default() -> Self {
        Self::Centered
    }
}

/// Configuration for AprilTag auto-detection
#[derive(Debug, Clone)]
pub struct AutoDetectConfig {
    /// Expected number of screens (for validation)
    pub expected_screens: usize,
    /// Tag family to use (default: Tag36h11)
    pub tag_family: AprilTagFamily,
    /// Physical size of tag relative to screen (e.g., 0.25 = 25% of screen width)
    pub tag_size_ratio: f32,
    /// Aspect ratio to assume when auto-detecting fails
    pub default_aspect_ratio: AspectRatio,
    /// Padding around detected region (as ratio of screen size)
    pub region_padding: f32,
    /// Minimum detection confidence
    pub min_confidence: f32,
    /// Where the tag is placed on the physical screen
    pub tag_placement: TagPlacement,
}

impl Default for AutoDetectConfig {
    fn default() -> Self {
        Self {
            expected_screens: 2,
            tag_family: AprilTagFamily::Tag36h11,
            tag_size_ratio: 0.60, // Tag is ~60% of screen width for better detection resolution
            default_aspect_ratio: AspectRatio::Ratio16_9,
            region_padding: 0.0, // No padding by default
            min_confidence: 10.0, // Minimum decision margin
            tag_placement: TagPlacement::Centered,
        }
    }
}

/// Detected screen information from AprilTag
#[derive(Debug, Clone)]
pub struct DetectedScreen {
    /// Screen ID (from AprilTag ID)
    pub screen_id: u32,
    /// Normalized corners of the screen region [TL, TR, BR, BL] in 0-1 UV space
    pub corners: [Vec2; 4],
    /// Center position in normalized coordinates
    pub center: Vec2,
    /// Detected aspect ratio
    pub aspect_ratio: AspectRatio,
    /// Detected orientation
    pub orientation: Orientation,
    /// Raw AprilTag detection data
    pub tag_detection: AprilTagDetection,
    /// Screen width in normalized coordinates (0-1)
    pub width: f32,
    /// Screen height in normalized coordinates (0-1)
    pub height: f32,
}

impl DetectedScreen {
    /// Get source rectangle in normalized UV coordinates
    pub fn source_rect(&self) -> (f32, f32, f32, f32) {
        (self.corners[0].x, self.corners[0].y, self.width, self.height)
    }

    /// Check if a point is inside this screen region
    pub fn contains(&self, uv: Vec2) -> bool {
        uv.x >= self.corners[0].x
            && uv.x <= self.corners[2].x
            && uv.y >= self.corners[0].y
            && uv.y <= self.corners[2].y
    }
}

/// AprilTag auto-detector for video matrix screens
pub struct AprilTagAutoDetector {
    config: AutoDetectConfig,
}

impl AprilTagAutoDetector {
    /// Create a new auto-detector with default config
    pub fn new() -> Self {
        Self {
            config: AutoDetectConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: AutoDetectConfig) -> Self {
        Self { config }
    }

    /// Detect screens in an image
    ///
    /// # Arguments
    /// * `image` - Input grayscale image
    /// * `image_size` - (width, height) of the image for normalization
    ///
    /// # Returns
    /// Vector of detected screens, sorted by screen_id
    pub fn detect_screens(
        &self,
        image: &GrayImage,
        image_size: (u32, u32),
    ) -> anyhow::Result<Vec<DetectedScreen>> {
        let (img_width, img_height) = image_size;
        let img_width_f = img_width as f32;
        let img_height_f = img_height as f32;

        // Detect AprilTags
        let mut detector = AprilTagDetector::new(self.config.tag_family);
        let detections = detector.detect(image);

        log::info!(
            "AprilTag detection found {} markers (expected {})",
            detections.len(),
            self.config.expected_screens
        );

        // Filter by confidence and convert to screens
        let mut screens: Vec<DetectedScreen> = detections
            .into_iter()
            .filter(|d| d.decision_margin >= self.config.min_confidence)
            .map(|detection| self.detection_to_screen(&detection, img_width_f, img_height_f))
            .collect();

        // Sort by screen_id for consistent ordering
        screens.sort_by_key(|s| s.screen_id);

        log::info!(
            "Detected {} screens with sufficient confidence",
            screens.len()
        );

        // Log detection details
        for screen in &screens {
            log::info!(
                "Screen {}: {:?} {:?} at ({:.3}, {:.3}), size {:.3}x{:.3}",
                screen.screen_id,
                screen.aspect_ratio.name(),
                screen.orientation,
                screen.center.x,
                screen.center.y,
                screen.width,
                screen.height
            );
        }

        Ok(screens)
    }

    /// Convert AprilTag detection to screen region
    fn detection_to_screen(
        &self,
        detection: &AprilTagDetection,
        img_width: f32,
        img_height: f32,
    ) -> DetectedScreen {
        // Normalize corners to 0-1 UV space
        let corners: [Vec2; 4] = [
            Vec2::new(detection.corners[0][0] / img_width, detection.corners[0][1] / img_height),
            Vec2::new(detection.corners[1][0] / img_width, detection.corners[1][1] / img_height),
            Vec2::new(detection.corners[2][0] / img_width, detection.corners[2][1] / img_height),
            Vec2::new(detection.corners[3][0] / img_width, detection.corners[3][1] / img_height),
        ];

        let center = Vec2::new(
            detection.center[0] / img_width,
            detection.center[1] / img_height,
        );

        // Detect orientation from tag rotation
        let orientation = Orientation::detect_from_corners(&detection.corners);
        
        // Debug: log corner positions (AprilTag order: TR, TL, BL, BR)
        log::info!("Tag {} corners: [0]=({:.0},{:.0}), [1]=({:.0},{:.0}), [2]=({:.0},{:.0}), [3]=({:.0},{:.0})",
            detection.id,
            detection.corners[0][0], detection.corners[0][1],
            detection.corners[1][0], detection.corners[1][1],
            detection.corners[2][0], detection.corners[2][1],
            detection.corners[3][0], detection.corners[3][1]);

        // Detect aspect ratio from tag distortion FIRST
        // AprilTags are square, so distortion tells us the screen's aspect ratio
        let tag_aspect = self.calculate_tag_aspect_ratio(&corners);
        let detected_aspect = self.detect_aspect_ratio_from_tag_aspect(tag_aspect);
        
        // Calculate image aspect ratio (width/height in pixels)
        let img_aspect = img_width / img_height;
        
        // Debug: log raw tag dimensions in pixels
        let tag_width_pixels = (detection.corners[1][0] - detection.corners[0][0]).abs();
        let tag_height_pixels = (detection.corners[3][1] - detection.corners[0][1]).abs();
        log::info!("Tag {} raw pixels: width={:.1}, height={:.1}, aspect={:.3}, orientation={:?}",
            detection.id, tag_width_pixels, tag_height_pixels, tag_width_pixels/tag_height_pixels, orientation);
        
        // Calculate screen dimensions and corners based on placement
        // Use the detected aspect ratio, not the default config
        let (screen_width, screen_height, screen_corners) = match self.config.tag_placement {
            TagPlacement::Centered => {
                self.calculate_centered_screen_with_aspect(&corners, center, detected_aspect, img_aspect)
            }
            TagPlacement::TopLeft => {
                self.calculate_corner_screen_with_aspect(&corners, center, orientation, detected_aspect, img_aspect)
            }
            _ => {
                self.calculate_centered_screen_with_aspect(&corners, center, detected_aspect, img_aspect)
            }
        };
        
        // Calculate final pixel-based aspect ratio for verification
        let pixel_width = screen_width * img_width;
        let pixel_height = screen_height * img_height;
        let calculated_ratio = screen_width / screen_height;
        let expected_ratio = detected_aspect.as_f32();
        
        let marker_to_fiducial = 0.8;
        log::info!("Screen {}: aspect={:?}, size={:.0}x{:.0}px (fiducial={:.0}px, slider={:.0}%, actual_fill={:.1}%)",
            detection.id, detected_aspect.name(), pixel_width, pixel_height, 
            tag_height_pixels, self.config.tag_size_ratio * 100.0, 
            self.config.tag_size_ratio * marker_to_fiducial * 100.0);

        DetectedScreen {
            screen_id: detection.id,
            corners: screen_corners,
            center,
            aspect_ratio: detected_aspect,
            orientation,
            tag_detection: detection.clone(),
            width: screen_width,
            height: screen_height,
        }
    }
    
    /// Detect screen aspect ratio from tag distortion
    /// 
    /// AprilTags are perfectly square (1:1). When 16:9 content is displayed:
    /// - On 4:3 screen (squished): tag appears tall/narrow, aspect ≈ 0.75
    /// - On 16:9 screen (native): tag appears square, aspect ≈ 1.0
    /// - On 21:9 screen (stretched): tag appears wide, aspect ≈ 1.33
    /// 
    /// NOTE: Camera perspective significantly distorts measurements. A 16:9 screen viewed
    /// at an angle may measure as low as 0.68 due to foreshortening.
    fn detect_aspect_ratio_from_tag_aspect(&self, tag_aspect: f32) -> AspectRatio {
        // These thresholds account for perspective distortion from angled cameras
        // - Strongly squeezed (< 0.60) = definitely 4:3 CRT
        // - Moderate squeeze (0.60-0.68) = likely 4:3
        // - Near square (> 0.68) = 16:9 with perspective distortion
        
        log::info!("Tag aspect ratio detection: {:.3}", tag_aspect);
        
        if tag_aspect < 0.60 {
            // Strongly squeezed - definitely 4:3 CRT
            log::info!("  -> Detected as 4:3 (strong squeeze, tag_aspect < 0.60)");
            AspectRatio::Ratio4_3
        } else if tag_aspect < 0.68 {
            // Moderately squeezed - likely 4:3
            log::info!("  -> Detected as 4:3 (moderate squeeze, 0.60 <= tag_aspect < 0.68)");
            AspectRatio::Ratio4_3
        } else if tag_aspect < 1.25 {
            // Near square (0.68 - 1.25) - this captures 16:9 screens
            // even with significant perspective distortion
            log::info!("  -> Detected as 16:9 (near square, 0.68 <= tag_aspect < 1.25)");
            AspectRatio::Ratio16_9
        } else if tag_aspect < 1.60 {
            // Stretched - 21:9 ultrawide
            log::info!("  -> Detected as 21:9 (stretched, 1.25 <= tag_aspect < 1.60)");
            AspectRatio::Ratio21_9
        } else {
            // Very stretched
            log::info!("  -> Detected as 21:9 (very stretched, tag_aspect >= 1.60)");
            AspectRatio::Ratio21_9
        }
    }
    
    /// Calculate centered screen with a specific aspect ratio
    /// 
    /// The detected tag corners are on the inner fiducial border (80% of marker).
    /// The marker is centered and fills tag_size_ratio of the screen.
    fn calculate_centered_screen_with_aspect(
        &self,
        tag_corners: &[Vec2; 4],
        tag_center: Vec2,
        aspect_ratio: AspectRatio,
        img_aspect: f32,
    ) -> (f32, f32, [Vec2; 4]) {
        // Calculate detected tag height (inner fiducial = ~80% of full marker)
        let left_height = (tag_corners[3] - tag_corners[0]).length();
        let right_height = (tag_corners[2] - tag_corners[1]).length();
        let fiducial_height_uv = (left_height + right_height) / 2.0;
        
        // The fiducial is 80% of the full marker, and marker fills tag_size_ratio of screen
        // So: fiducial = 0.8 * marker, and marker = slider * screen
        // Therefore: screen = fiducial / (0.8 * slider)
        let marker_to_fiducial_ratio = 0.8;
        let actual_fill = self.config.tag_size_ratio * marker_to_fiducial_ratio;
        let screen_height_uv = fiducial_height_uv / actual_fill.clamp(0.1, 1.0);
        
        // Screen width in UV coordinates:
        // screen_width_uv = screen_height_uv * (screen_aspect / image_aspect)
        let screen_aspect = aspect_ratio.as_f32();
        let screen_width_uv = screen_height_uv * (screen_aspect / img_aspect);
        
        // Calculate half dimensions
        let half_width = screen_width_uv / 2.0;
        let half_height = screen_height_uv / 2.0;

        // Calculate screen corners centered on tag_center
        let screen_corners = [
            Vec2::new(tag_center.x - half_width, tag_center.y - half_height), // TL
            Vec2::new(tag_center.x + half_width, tag_center.y - half_height), // TR
            Vec2::new(tag_center.x + half_width, tag_center.y + half_height), // BR
            Vec2::new(tag_center.x - half_width, tag_center.y + half_height), // BL
        ];

        log::debug!("Screen calc: aspect={:?}, img_aspect={:.3}, height={:.3}, width={:.3}",
            aspect_ratio.name(), img_aspect, screen_height_uv, screen_width_uv);

        (screen_width_uv, screen_height_uv, screen_corners)
    }
    
    /// Calculate corner screen with a specific aspect ratio
    fn calculate_corner_screen_with_aspect(
        &self,
        tag_corners: &[Vec2; 4],
        _tag_center: Vec2,
        orientation: Orientation,
        aspect_ratio: AspectRatio,
        img_aspect: f32,
    ) -> (f32, f32, [Vec2; 4]) {
        // Calculate detected fiducial size (inner 80% of marker)
        let fiducial_width = (tag_corners[1] - tag_corners[0]).length();
        let fiducial_height = (tag_corners[3] - tag_corners[0]).length();
        
        // Scale fiducial to screen: fiducial is 80% of marker, marker fills slider% of screen
        let marker_to_fiducial_ratio = 0.8;
        let actual_fill = self.config.tag_size_ratio * marker_to_fiducial_ratio;
        let scale_factor = 1.0 / actual_fill.clamp(0.1, 1.0);
        let screen_height_uv = fiducial_height * scale_factor;
        let screen_aspect = aspect_ratio.as_f32();
        let screen_width_uv = screen_height_uv * (screen_aspect / img_aspect);

        // Calculate screen corners based on tag placement (top-left)
        let tag_tl = tag_corners[0];
        let tag_tr = tag_corners[1];
        let tag_bl = tag_corners[3];

        // Calculate screen edges based on tag orientation
        let top_edge = tag_tr - tag_tl;
        let left_edge = tag_bl - tag_tl;

        // Normalize edge directions
        let top_dir = if top_edge.length() > 0.0 {
            top_edge.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let left_dir = if left_edge.length() > 0.0 {
            left_edge.normalize()
        } else {
            Vec2::new(0.0, 1.0)
        };

        let screen_tl = tag_tl;
        let screen_tr = tag_tl + top_dir * screen_width_uv;
        let screen_bl = tag_tl + left_dir * screen_height_uv;
        let screen_br = screen_bl + top_dir * screen_width_uv;

        let screen_corners = [screen_tl, screen_tr, screen_br, screen_bl];

        // Apply orientation swap if needed
        match orientation {
            Orientation::Rotated90 | Orientation::Rotated270 => {
                (screen_height_uv, screen_width_uv, screen_corners)
            }
            _ => (screen_width_uv, screen_height_uv, screen_corners),
        }
    }
    
    /// Calculate tag aspect ratio from detected corners
    /// Returns width/height ratio (1.0 = square, <1 = squeezed horizontally, >1 = stretched horizontally)
    fn calculate_tag_aspect_ratio(&self, corners: &[Vec2; 4]) -> f32 {
        // Calculate tag width (average of top and bottom edges)
        let top_width = (corners[1] - corners[0]).length();
        let bottom_width = (corners[2] - corners[3]).length();
        let tag_width = (top_width + bottom_width) / 2.0;
        
        // Calculate tag height (average of left and right edges)
        let left_height = (corners[3] - corners[0]).length();
        let right_height = (corners[2] - corners[1]).length();
        let tag_height = (left_height + right_height) / 2.0;
        
        if tag_height > 0.0 {
            tag_width / tag_height
        } else {
            1.0 // Assume square if can't calculate
        }
    }

    /// Calculate screen region when tag is centered on screen
    ///
    /// The tag is displayed in the center of the screen as a test pattern.
    /// We calculate screen bounds by extending from the tag based on the
    /// known tag-to-screen size ratio.
    fn calculate_centered_screen(
        &self,
        tag_corners: &[Vec2; 4],
        tag_center: Vec2,
        img_width: f32,
        img_height: f32,
    ) -> (f32, f32, [Vec2; 4]) {
        // Calculate tag width/height in normalized coordinates
        let tag_width = (tag_corners[1] - tag_corners[0]).length();
        let tag_height = (tag_corners[3] - tag_corners[0]).length();
        let tag_avg_size = (tag_width + tag_height) / 2.0;

        // Calculate screen size from tag ratio
        // tag_size_ratio = tag_width / screen_width
        let screen_width = tag_avg_size / self.config.tag_size_ratio;
        let screen_height = screen_width / self.config.default_aspect_ratio.as_f32();

        // Calculate screen corners centered on tag_center
        let half_width = screen_width / 2.0;
        let half_height = screen_height / 2.0;

        let screen_corners = [
            Vec2::new(tag_center.x - half_width, tag_center.y - half_height), // TL
            Vec2::new(tag_center.x + half_width, tag_center.y - half_height), // TR
            Vec2::new(tag_center.x + half_width, tag_center.y + half_height), // BR
            Vec2::new(tag_center.x - half_width, tag_center.y + half_height), // BL
        ];

        (screen_width, screen_height, screen_corners)
    }

    /// Calculate screen region when tag is at a corner
    fn calculate_corner_screen(
        &self,
        tag_corners: &[Vec2; 4],
        _tag_center: Vec2,
        orientation: Orientation,
        img_width: f32,
        img_height: f32,
    ) -> (f32, f32, [Vec2; 4]) {
        // Calculate actual tag width and height in pixels
        let tag_width = (tag_corners[1] - tag_corners[0]).length() * img_width;
        let tag_height = (tag_corners[3] - tag_corners[0]).length() * img_height;
        let tag_avg_size = (tag_width + tag_height) / 2.0;

        // Calculate expected screen size based on tag ratio
        let expected_screen_width = tag_avg_size / self.config.tag_size_ratio;
        let expected_screen_height =
            expected_screen_width / self.config.default_aspect_ratio.as_f32();

        // Normalize to 0-1
        let screen_width_norm = expected_screen_width / img_width;
        let screen_height_norm = expected_screen_height / img_height;

        // Calculate screen corners based on tag placement
        // For now, assume top-left placement
        let tag_tl = tag_corners[0];
        let tag_tr = tag_corners[1];
        let tag_bl = tag_corners[3];

        // Calculate screen edges based on tag orientation
        let top_edge = tag_tr - tag_tl;
        let left_edge = tag_bl - tag_tl;

        // Normalize edge directions
        let top_dir = if top_edge.length() > 0.0 {
            top_edge.normalize()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let left_dir = if left_edge.length() > 0.0 {
            left_edge.normalize()
        } else {
            Vec2::new(0.0, 1.0)
        };

        let screen_tl = tag_tl;
        let screen_tr = tag_tl + top_dir * screen_width_norm;
        let screen_bl = tag_tl + left_dir * screen_height_norm;
        let screen_br = screen_bl + top_dir * screen_width_norm;

        let screen_corners = [screen_tl, screen_tr, screen_br, screen_bl];

        // Apply orientation swap if needed
        match orientation {
            Orientation::Rotated90 | Orientation::Rotated270 => {
                // Portrait orientation - swap dimensions
                (screen_height_norm, screen_width_norm, screen_corners)
            }
            _ => (screen_width_norm, screen_height_norm, screen_corners),
        }
    }

    /// Create video matrix config from detected screens
    ///
    /// Maps detected screens to specific output cells in a FIXED 3x3 output grid.
    /// Screens are mapped consecutively starting from the specified output position.
    /// Remaining cells in the 3x3 grid are left empty (will show black).
    ///
    /// # Arguments
    /// * `screens` - Detected screens from detect_screens()
    /// * `input_resolution` - (width, height) of input texture
    /// * `output_start_pos` - Optional (col, row) to start mapping screens to (default: 0,0)
    ///
    /// # Returns
    /// Configured VideoMatrixConfig with 3x3 output grid
    pub fn create_matrix_config(
        &self,
        screens: &[DetectedScreen],
        _input_resolution: (u32, u32),
        output_start_pos: Option<(u32, u32)>,
    ) -> anyhow::Result<VideoMatrixConfig> {
        if screens.is_empty() {
            anyhow::bail!("No screens detected");
        }

        // FIXED: Always use 3x3 output grid regardless of detected screen count
        let input_grid_size = GridSize::new(screens.len() as u32, 1);
        let output_grid_size = GridSize::new(3, 3); // Always 3x3!
        
        // Create input grid config
        let mut input_grid = InputGridConfig::new(input_grid_size);

        // Get starting output position (default to 0,0)
        let (start_col, start_row) = output_start_pos.unwrap_or((0, 0));
        let start_col = start_col.min(2) as f32; // Clamp to valid 3x3 range
        let start_row = start_row.min(2) as f32;

        // Create mapping for each detected screen
        // Map Screen 0 → output (start_col, start_row), Screen 1 → next cell, etc.
        for (idx, screen) in screens.iter().enumerate() {
            let input_cell = idx;
            
            // Calculate output position with wrapping within 3x3 grid
            let offset = idx as f32;
            let output_col = (start_col + offset) % 3.0;
            let output_row = start_row + ((start_col + offset) / 3.0).floor();
            
            // Ensure we don't go out of bounds
            if output_row >= 3.0 {
                log::warn!("Screen {} cannot be mapped - exceeds 3x3 output grid bounds", idx);
                continue;
            }
            
            let output_position = GridPosition::new(output_col, output_row, 1.0, 1.0);

            // Create source rect from detected screen corners (normalized UV coordinates)
            let source_rect = super::Rect::new(
                screen.corners[0].x, // Top-left X
                screen.corners[0].y, // Top-left Y
                screen.width,        // Width
                screen.height,       // Height
            );

            let mapping = GridCellMapping::new(input_cell, output_position)
                .with_aspect_ratio(screen.aspect_ratio)
                .with_orientation(screen.orientation)
                .with_display_id(screen.screen_id)
                .with_source_rect(source_rect);

            input_grid.add_mapping(mapping);
        }

        Ok(VideoMatrixConfig {
            input_grid,
            output_grid: output_grid_size, // Fixed 3x3
            background_color: [0.0, 0.0, 0.0, 1.0],
            auto_detect: true,
            detected_screens: Vec::new(),
        })
    }

    /// Create video matrix config with specific output position (convenience method)
    ///
    /// This is a wrapper around `create_matrix_config` for backwards compatibility.
    pub fn create_matrix_config_with_position(
        &self,
        screens: &[DetectedScreen],
        input_resolution: (u32, u32),
        start_col: u32,
        start_row: u32,
    ) -> anyhow::Result<VideoMatrixConfig> {
        self.create_matrix_config(screens, input_resolution, Some((start_col, start_row)))
    }

    /// Create a simple 2-screen side-by-side configuration
    ///
    /// This is a convenience method for the common case of 2 screens
    pub fn create_two_screen_config(
        &self,
        screen0_aspect: AspectRatio,
        screen1_aspect: AspectRatio,
    ) -> VideoMatrixConfig {
        let grid_size = GridSize::new(2, 1);
        let mut input_grid = InputGridConfig::new(grid_size);

        // Screen 0 (left)
        let mapping_0 = GridCellMapping::new(0, GridPosition::new(0.0, 0.0, 1.0, 1.0))
            .with_aspect_ratio(screen0_aspect)
            .with_display_id(0);

        // Screen 1 (right)
        let mapping_1 = GridCellMapping::new(1, GridPosition::new(1.0, 0.0, 1.0, 1.0))
            .with_aspect_ratio(screen1_aspect)
            .with_display_id(1);

        input_grid.add_mapping(mapping_0);
        input_grid.add_mapping(mapping_1);

        VideoMatrixConfig {
            input_grid,
            output_grid: GridSize::new(2, 1),
            background_color: [0.0, 0.0, 0.0, 1.0],
            auto_detect: false,
            detected_screens: Vec::new(),
        }
    }

    /// Quick detect and configure in one step
    ///
    /// # Arguments
    /// * `image` - Input grayscale image
    /// * `image_size` - (width, height) of input image
    /// * `input_resolution` - Resolution for input texture reference
    pub fn auto_configure(
        &self,
        image: &GrayImage,
        image_size: (u32, u32),
        input_resolution: (u32, u32),
    ) -> anyhow::Result<VideoMatrixConfig> {
        let screens = self.detect_screens(image, image_size)?;

        if screens.len() != self.config.expected_screens {
            log::warn!(
                "Detected {} screens but expected {}",
                screens.len(),
                self.config.expected_screens
            );
        }

        self.create_matrix_config(&screens, input_resolution, None)
    }

    /// Get current config
    pub fn config(&self) -> &AutoDetectConfig {
        &self.config
    }

    /// Update config
    pub fn set_config(&mut self, config: AutoDetectConfig) {
        self.config = config;
    }
}

impl Default for AprilTagAutoDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to extract a grayscale image from a wgpu texture
/// For use when running detection on GPU textures
pub fn texture_to_gray_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> anyhow::Result<GrayImage> {
    // Create buffer to read texture data
    let buffer_size = (width * height * 4) as u64; // BGRA8
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("AprilTag Readback Buffer"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture to buffer
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("AprilTag Copy Encoder"),
    });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map buffer and convert to grayscale
    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });

    // Poll device to process buffer mapping
    device.poll(wgpu::PollType::Wait)?;

    // Wait for mapping to complete
    receiver.recv()?.map_err(|e| anyhow::anyhow!("Buffer mapping failed: {:?}", e))?;

    let data = buffer_slice.get_mapped_range();
    let mut gray_data = Vec::with_capacity((width * height) as usize);

    // Convert BGRA to grayscale (luminance)
    for chunk in data.chunks_exact(4) {
        let b = chunk[0] as f32;
        let g = chunk[1] as f32;
        let r = chunk[2] as f32;
        // ITU-R BT.601 luma coefficients
        let luma = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
        gray_data.push(luma);
    }

    drop(data);
    buffer.unmap();

    GrayImage::from_raw(width, height, gray_data)
        .ok_or_else(|| anyhow::anyhow!("Failed to create grayscale image"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_detect_config_default() {
        let config = AutoDetectConfig::default();
        assert_eq!(config.expected_screens, 2);
        assert_eq!(config.tag_size_ratio, 0.25);
        assert!(matches!(config.default_aspect_ratio, AspectRatio::Ratio16_9));
        assert!(matches!(config.tag_placement, TagPlacement::Centered));
    }

    #[test]
    fn test_tag_placement_variants() {
        assert!(matches!(TagPlacement::Centered, TagPlacement::Centered));
        assert!(matches!(TagPlacement::TopLeft, TagPlacement::TopLeft));
    }

    #[test]
    fn test_two_screen_config() {
        let detector = AprilTagAutoDetector::new();
        let config = detector.create_two_screen_config(
            AspectRatio::Ratio4_3,
            AspectRatio::Ratio16_9,
        );

        assert_eq!(config.output_grid.columns, 2);
        assert_eq!(config.output_grid.rows, 1);
        assert_eq!(config.input_grid.mappings.len(), 2);

        // Check first mapping
        let mapping0 = &config.input_grid.mappings[0];
        assert_eq!(mapping0.input_cell, 0);
        assert!(matches!(mapping0.aspect_ratio, AspectRatio::Ratio4_3));

        // Check second mapping
        let mapping1 = &config.input_grid.mappings[1];
        assert_eq!(mapping1.input_cell, 1);
        assert!(matches!(mapping1.aspect_ratio, AspectRatio::Ratio16_9));
    }
}
