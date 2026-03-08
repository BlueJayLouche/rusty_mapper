//! # Video Wall Configuration
//!
//! Persistent configuration format for video wall calibration results.
//! Supports serialization to JSON for easy storage and editing.

use super::{DisplayQuad, GridSize, Rect};
use glam::Vec2;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Version number for config format compatibility
const CONFIG_VERSION: u32 = 1;

/// Video wall configuration - saved calibration results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoWallConfig {
    /// Config format version
    pub version: u32,
    /// Grid dimensions (e.g., 2x2, 3x3)
    pub grid_size: GridSize,
    /// Output resolution when calibrated
    pub output_resolution: (u32, u32),
    /// Per-display configurations
    pub displays: Vec<DisplayConfig>,
    /// Calibration metadata
    pub calibration_info: CalibrationInfo,
}

/// Per-display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Display ID (0-indexed, left-to-right, top-to-bottom)
    pub id: u32,
    /// Grid position (column, row)
    pub grid_position: (u32, u32),
    /// Human-readable name (e.g., "Display 1 (Top-Left)")
    pub name: String,

    /// Source UV coordinates in main texture (where to sample from)
    pub source_uv: Rect,

    /// Destination quad corners in output space (where to render to)
    /// Order: top-left, top-right, bottom-right, bottom-left
    pub dest_quad: [[f32; 2]; 4],

    /// Per-display adjustments (applied after sampling)
    #[serde(default = "default_one")]
    pub brightness: f32,
    #[serde(default = "default_one")]
    pub contrast: f32,
    #[serde(default = "default_one")]
    pub gamma: f32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_one() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

/// Calibration metadata for troubleshooting and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationInfo {
    /// When calibration was performed (ISO 8601 format)
    pub date: String,
    /// Camera source used for calibration
    pub camera_source: String,
    /// Camera resolution during calibration
    pub camera_resolution: (u32, u32),
    /// ArUco dictionary used
    pub marker_dictionary: String,
    /// Average detection confidence (0-1)
    pub avg_detection_confidence: f32,
    /// Calibration duration in seconds
    pub calibration_duration_secs: f64,
}

impl DisplayConfig {
    /// Create a new display config from a DisplayQuad
    pub fn from_quad(quad: &DisplayQuad) -> Self {
        let corners: [[f32; 2]; 4] = [
            [quad.dest_corners[0].x, quad.dest_corners[0].y],
            [quad.dest_corners[1].x, quad.dest_corners[1].y],
            [quad.dest_corners[2].x, quad.dest_corners[2].y],
            [quad.dest_corners[3].x, quad.dest_corners[3].y],
        ];

        Self {
            id: quad.display_id,
            grid_position: quad.grid_position,
            name: format!("Display {} (Col {}, Row {})", quad.display_id + 1, quad.grid_position.0, quad.grid_position.1),
            source_uv: quad.source_rect,
            dest_quad: corners,
            brightness: 1.0,
            contrast: 1.0,
            gamma: 1.0,
            enabled: true,
        }
    }

    /// Get destination corners as Vec2 array
    pub fn dest_corners_vec2(&self) -> [Vec2; 4] {
        [
            Vec2::new(self.dest_quad[0][0], self.dest_quad[0][1]),
            Vec2::new(self.dest_quad[1][0], self.dest_quad[1][1]),
            Vec2::new(self.dest_quad[2][0], self.dest_quad[2][1]),
            Vec2::new(self.dest_quad[3][0], self.dest_quad[3][1]),
        ]
    }

    /// Set destination corners from Vec2 array
    pub fn set_dest_corners(&mut self, corners: &[Vec2; 4]) {
        self.dest_quad = [
            [corners[0].x, corners[0].y],
            [corners[1].x, corners[1].y],
            [corners[2].x, corners[2].y],
            [corners[3].x, corners[3].y],
        ];
    }
}

impl VideoWallConfig {
    /// Create a new video wall configuration from display quads
    pub fn from_quads(
        quads: Vec<DisplayQuad>,
        grid_size: GridSize,
        output_resolution: (u32, u32),
        calibration_info: CalibrationInfo,
    ) -> Self {
        let displays: Vec<DisplayConfig> = quads.iter().map(DisplayConfig::from_quad).collect();

        Self {
            version: CONFIG_VERSION,
            grid_size,
            output_resolution,
            displays,
            calibration_info,
        }
    }

    /// Load configuration from a JSON file
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a JSON file
    pub fn save_to_file(&self, path: &Path) -> anyhow::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config file path
    pub fn default_config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rusty_mapper");
        config_dir.join("videowall_config.json")
    }

    /// Save to default location
    pub fn save_default(&self) -> anyhow::Result<()> {
        let path = Self::default_config_path();
        self.save_to_file(&path)
    }

    /// Load from default location if it exists
    pub fn load_default() -> Option<Self> {
        let path = Self::default_config_path();
        if path.exists() {
            match Self::load_from_file(&path) {
                Ok(config) => Some(config),
                Err(e) => {
                    log::warn!("Failed to load default videowall config: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Validate the configuration
    fn validate(&self) -> anyhow::Result<()> {
        if self.version != CONFIG_VERSION {
            anyhow::bail!(
                "Unsupported config version: {} (expected {})",
                self.version,
                CONFIG_VERSION
            );
        }

        let expected_displays = self.grid_size.total_displays() as usize;
        if self.displays.len() != expected_displays {
            anyhow::bail!(
                "Display count mismatch: expected {} displays for {:?} grid, found {}",
                expected_displays,
                self.grid_size,
                self.displays.len()
            );
        }

        // Check all display IDs are unique and in range
        let mut ids: Vec<u32> = self.displays.iter().map(|d| d.id).collect();
        ids.sort();
        for (i, id) in ids.iter().enumerate() {
            if *id != i as u32 {
                anyhow::bail!("Display ID {} out of sequence (expected {})", id, i);
            }
        }

        Ok(())
    }

    /// Get display config by ID
    pub fn get_display(&self, id: u32) -> Option<&DisplayConfig> {
        self.displays.iter().find(|d| d.id == id)
    }

    /// Get display config by grid position
    pub fn get_display_at(&self, col: u32, row: u32) -> Option<&DisplayConfig> {
        let id = self.grid_size.id_from_position(col, row);
        self.get_display(id)
    }

    /// Check if all displays are enabled
    pub fn all_enabled(&self) -> bool {
        self.displays.iter().all(|d| d.enabled)
    }

    /// Get enabled display count
    pub fn enabled_count(&self) -> usize {
        self.displays.iter().filter(|d| d.enabled).count()
    }

    /// Update a display's color adjustments
    pub fn update_display_adjustments(
        &mut self,
        display_id: u32,
        brightness: Option<f32>,
        contrast: Option<f32>,
        gamma: Option<f32>,
    ) -> bool {
        if let Some(display) = self.displays.iter_mut().find(|d| d.id == display_id) {
            if let Some(b) = brightness {
                display.brightness = b.clamp(0.0, 2.0);
            }
            if let Some(c) = contrast {
                display.contrast = c.clamp(0.0, 2.0);
            }
            if let Some(g) = gamma {
                display.gamma = g.clamp(0.1, 3.0);
            }
            true
        } else {
            false
        }
    }

    /// Toggle display enabled state
    pub fn toggle_display(&mut self, display_id: u32) -> bool {
        if let Some(display) = self.displays.iter_mut().find(|d| d.id == display_id) {
            display.enabled = !display.enabled;
            true
        } else {
            false
        }
    }

    /// Set display enabled state
    pub fn set_display_enabled(&mut self, display_id: u32, enabled: bool) -> bool {
        if let Some(display) = self.displays.iter_mut().find(|d| d.id == display_id) {
            display.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Reset all adjustments to defaults
    pub fn reset_adjustments(&mut self) {
        for display in &mut self.displays {
            display.brightness = 1.0;
            display.contrast = 1.0;
            display.gamma = 1.0;
        }
    }
}

/// Configuration preset with name and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPreset {
    /// Preset name (e.g., "Stage Setup A")
    pub name: String,
    /// Optional description
    pub description: String,
    /// When created/modified
    pub modified_date: String,
    /// The actual configuration
    pub config: VideoWallConfig,
}

impl ConfigPreset {
    /// Create a new preset from a config
    pub fn new(name: impl Into<String>, config: VideoWallConfig) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            modified_date: chrono::Utc::now().to_rfc3339(),
            config,
        }
    }

    /// Update the config and modification date
    pub fn update_config(&mut self, config: VideoWallConfig) {
        self.config = config;
        self.modified_date = chrono::Utc::now().to_rfc3339();
    }
}

/// Preset manager for loading/saving multiple configurations
pub struct PresetManager {
    presets_dir: PathBuf,
}

impl PresetManager {
    /// Create a new preset manager
    pub fn new() -> Self {
        let presets_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rusty_mapper")
            .join("presets");
        
        Self { presets_dir }
    }

    /// Create with custom directory
    pub fn with_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            presets_dir: path.into(),
        }
    }

    /// Get the presets directory
    pub fn presets_dir(&self) -> &Path {
        &self.presets_dir
    }

    /// Ensure presets directory exists
    pub fn ensure_dir(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.presets_dir)?;
        Ok(())
    }

    /// Save a preset
    pub fn save_preset(&self, preset: &ConfigPreset) -> anyhow::Result<PathBuf> {
        self.ensure_dir()?;
        
        let filename = format!("{}.json", sanitize_filename(&preset.name));
        let path = self.presets_dir.join(filename);
        
        let content = serde_json::to_string_pretty(preset)?;
        std::fs::write(&path, content)?;
        
        Ok(path)
    }

    /// Load a preset by name
    pub fn load_preset(&self, name: &str) -> anyhow::Result<ConfigPreset> {
        let filename = format!("{}.json", sanitize_filename(name));
        let path = self.presets_dir.join(filename);
        
        let content = std::fs::read_to_string(&path)?;
        let preset: ConfigPreset = serde_json::from_str(&content)?;
        
        Ok(preset)
    }

    /// Load a preset from a specific path
    pub fn load_preset_from_path(path: &Path) -> anyhow::Result<ConfigPreset> {
        let content = std::fs::read_to_string(path)?;
        let preset: ConfigPreset = serde_json::from_str(&content)?;
        Ok(preset)
    }

    /// List all available presets
    pub fn list_presets(&self) -> anyhow::Result<Vec<PresetInfo>> {
        self.ensure_dir()?;
        
        let mut presets = Vec::new();
        
        for entry in std::fs::read_dir(&self.presets_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension() == Some(std::ffi::OsStr::new("json")) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(preset) = serde_json::from_str::<ConfigPreset>(&content) {
                        presets.push(PresetInfo {
                            name: preset.name,
                            description: preset.description,
                            modified_date: preset.modified_date,
                            path,
                            grid_size: preset.config.grid_size,
                            enabled_displays: preset.config.enabled_count(),
                        });
                    }
                }
            }
        }
        
        // Sort by name
        presets.sort_by(|a, b| a.name.cmp(&b.name));
        
        Ok(presets)
    }

    /// Delete a preset
    pub fn delete_preset(&self, name: &str) -> anyhow::Result<()> {
        let filename = format!("{}.json", sanitize_filename(name));
        let path = self.presets_dir.join(filename);
        
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        
        Ok(())
    }

    /// Quick save with timestamped name
    pub fn quick_save(&self, config: &VideoWallConfig) -> anyhow::Result<PathBuf> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let name = format!("Calibration_{}", timestamp);
        let preset = ConfigPreset::new(name, config.clone());
        self.save_preset(&preset)
    }
}

impl Default for PresetManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a preset (without loading full config)
#[derive(Debug, Clone)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub modified_date: String,
    pub path: PathBuf,
    pub grid_size: GridSize,
    pub enabled_displays: usize,
}

/// Sanitize a filename to be safe for filesystem
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' => '_',
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c => c,
        })
        .collect()
}

impl Default for CalibrationInfo {
    fn default() -> Self {
        Self {
            date: chrono::Utc::now().to_rfc3339(),
            camera_source: String::new(),
            camera_resolution: (0, 0),
            marker_dictionary: String::from("DICT_4X4_50"),
            avg_detection_confidence: 0.0,
            calibration_duration_secs: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_quad(id: u32, grid_pos: (u32, u32)) -> DisplayQuad {
        DisplayQuad {
            display_id: id,
            grid_position: grid_pos,
            source_rect: Rect::new(0.0, 0.0, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 1.0),
            ],
            perspective_matrix: None,
        }
    }

    #[test]
    fn test_config_creation() {
        let quads = vec![
            create_test_quad(0, (0, 0)),
            create_test_quad(1, (1, 0)),
            create_test_quad(2, (0, 1)),
            create_test_quad(3, (1, 1)),
        ];

        let info = CalibrationInfo {
            date: "2024-01-01T00:00:00Z".to_string(),
            camera_source: "Test Camera".to_string(),
            camera_resolution: (1920, 1080),
            marker_dictionary: "DICT_4X4_50".to_string(),
            avg_detection_confidence: 0.95,
            calibration_duration_secs: 10.5,
        };

        let config = VideoWallConfig::from_quads(quads, GridSize::two_by_two(), (1920, 1080), info);

        assert_eq!(config.grid_size.total_displays(), 4);
        assert_eq!(config.displays.len(), 4);
        assert_eq!(config.version, CONFIG_VERSION);
    }

    #[test]
    fn test_config_serialization() {
        let quads = vec![
            create_test_quad(0, (0, 0)),
            create_test_quad(1, (1, 0)),
        ];

        let info = CalibrationInfo {
            date: "2024-01-01T00:00:00Z".to_string(),
            camera_source: "Test Camera".to_string(),
            camera_resolution: (1920, 1080),
            marker_dictionary: "DICT_4X4_50".to_string(),
            avg_detection_confidence: 0.95,
            calibration_duration_secs: 10.5,
        };

        let config = VideoWallConfig::from_quads(quads, GridSize::new(2, 1), (1920, 1080), info);

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("Serialized config:\n{}", json);

        // Deserialize back
        let deserialized: VideoWallConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.grid_size.total_displays(), 2);
        assert_eq!(deserialized.displays.len(), 2);
    }

    #[test]
    fn test_display_config_from_quad() {
        let quad = DisplayQuad {
            display_id: 0,
            grid_position: (0, 0),
            source_rect: Rect::new(0.0, 0.0, 0.5, 0.5),
            dest_corners: [
                Vec2::new(0.1, 0.1),
                Vec2::new(0.9, 0.1),
                Vec2::new(0.9, 0.9),
                Vec2::new(0.1, 0.9),
            ],
            perspective_matrix: None,
        };

        let config = DisplayConfig::from_quad(&quad);

        assert_eq!(config.id, 0);
        assert_eq!(config.grid_position, (0, 0));
        assert!(config.name.contains("Display 1"));
        assert_eq!(config.source_uv.x, 0.0);
        assert_eq!(config.brightness, 1.0);
        assert_eq!(config.contrast, 1.0);
        assert!(config.enabled);

        // Test round-trip of corners
        let corners = config.dest_corners_vec2();
        assert!((corners[0].x - 0.1).abs() < 0.001);
        assert!((corners[3].y - 0.9).abs() < 0.001);
    }
}
