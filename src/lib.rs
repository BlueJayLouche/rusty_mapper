//! # Rusty Mapper Library
//!
//! Core library for the Rusty Mapper projection mapping application.
//! This library is primarily used by the main application, but exposes
//! some modules for examples and testing.

pub mod videowall;

// Re-export commonly used types
pub use videowall::{
    // AprilTag (pure Rust, recommended)
    AprilTagDetector, AprilTagDetection, AprilTagFamily, AprilTagGenerator,
    // Test patterns
    TestPattern,
    // ArUco (OpenCV-based, may have compatibility issues)
    ArUcoDetector, ArUcoDictionary, ArUcoGenerator, CalibrationController,
    CalibrationError, CalibrationInfo, CalibrationMode, CalibrationPhase,
    CalibrationStatus, CalibrationTiming, DisplayConfig, DisplayQuad, 
    DisplayQuadUniform, GridSize, QuadMapConfig, QuadMapResult, QuadMapper,
    Rect, VideoWallConfig, VideoWallRenderer, VideoWallUniforms, MAX_DISPLAYS,
};
