//! # Video Wall Auto-Calibration
//!
//! Auto-calibration system for HDMI matrix video walls using ArUco markers.
//! Supports any grid configuration (2x2, 3x3, 4x4, etc.).
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rusty_mapper::videowall::{ArUcoGenerator, VideoWallConfig};
//!
//! // Generate calibration pattern for display 0 in a 2x2 grid
//! let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
//! let pattern = generator.generate_calibration_frame(0, (2, 2), (1920, 1080));
//!
//! // Detect markers in a camera frame
//! let detector = ArUcoDetector::new(ArUcoDictionary::Dict4x4_50);
//! let markers = detector.detect_markers(&frame)?;
//! ```

pub mod aruco;
pub mod calibration;
pub mod config;
pub mod quad_mapper;
pub mod renderer;

pub use aruco::{ArUcoDetector, ArUcoDictionary, ArUcoGenerator, DetectedMarker};
pub use calibration::{
    CalibrationController, CalibrationError, CalibrationMode, CalibrationPhase,
    CalibrationStatus, CalibrationTiming, CapturedFrame, DisplayDetection,
    MarkerDisplayConfig,
};
pub use quad_mapper::{QuadMapper, QuadMapConfig, QuadMapResult};
pub use renderer::{DisplayQuadUniform, VideoWallRenderer, VideoWallUniforms, MAX_DISPLAYS};
pub use config::{CalibrationInfo, DisplayConfig, VideoWallConfig, ConfigPreset, PresetManager, PresetInfo};

use glam::{Mat3, Vec2};
use serde::{Deserialize, Serialize};

/// A detected display with its position and corners
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayQuad {
    /// Display ID (0-indexed)
    pub display_id: u32,
    /// Grid position (column, row) - (0,0) is top-left
    pub grid_position: (u32, u32),

    /// Source rectangle in main texture UV space (0-1)
    pub source_rect: Rect,

    /// Destination corners in normalized output coordinates
    /// Order: top-left, top-right, bottom-right, bottom-left
    pub dest_corners: [Vec2; 4],

    /// Perspective transformation matrix (computed from corners)
    #[serde(skip)]
    pub perspective_matrix: Option<Mat3>,
}

/// Rectangle in UV space (0-1 range)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the top-left corner
    pub fn min(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Get the bottom-right corner
    pub fn max(&self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

/// Grid size for video wall configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSize {
    pub columns: u32,
    pub rows: u32,
}

impl GridSize {
    /// Create a new grid size
    pub fn new(columns: u32, rows: u32) -> Self {
        Self { columns, rows }
    }

    /// Get total number of displays
    pub fn total_displays(&self) -> u32 {
        self.columns * self.rows
    }

    /// Get grid position from display ID
    pub fn position_from_id(&self, display_id: u32) -> (u32, u32) {
        let row = display_id / self.columns;
        let col = display_id % self.columns;
        (col, row)
    }

    /// Get display ID from grid position
    pub fn id_from_position(&self, col: u32, row: u32) -> u32 {
        row * self.columns + col
    }

    /// Common grid sizes
    pub fn two_by_two() -> Self {
        Self::new(2, 2)
    }

    pub fn three_by_three() -> Self {
        Self::new(3, 3)
    }

    pub fn four_by_four() -> Self {
        Self::new(4, 4)
    }
}

impl Default for GridSize {
    fn default() -> Self {
        Self::two_by_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_size() {
        let grid = GridSize::new(3, 2);
        assert_eq!(grid.total_displays(), 6);
        assert_eq!(grid.position_from_id(0), (0, 0));
        assert_eq!(grid.position_from_id(1), (1, 0));
        assert_eq!(grid.position_from_id(2), (2, 0));
        assert_eq!(grid.position_from_id(3), (0, 1));
        assert_eq!(grid.position_from_id(4), (1, 1));
        assert_eq!(grid.position_from_id(5), (2, 1));
    }

    #[test]
    fn test_rect() {
        let rect = Rect::new(0.25, 0.25, 0.5, 0.5);
        assert_eq!(rect.min(), Vec2::new(0.25, 0.25));
        assert_eq!(rect.max(), Vec2::new(0.75, 0.75));
    }
}
