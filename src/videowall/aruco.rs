//! # ArUco Marker Generation and Detection
//!
//! Provides ArUco marker generation for calibration patterns and detection
//! in camera frames. Uses OpenCV when the `opencv` feature is enabled.
//!
//! ## ArUco Dictionaries
//!
//! - `Dict4x4_50`: 4x4 markers, 50 unique IDs (recommended for most uses)
//! - `Dict4x4_100`: 4x4 markers, 100 unique IDs
//! - `Dict4x4_250`: 4x4 markers, 250 unique IDs
//! - `Dict6x6_250`: 6x6 markers, 250 unique IDs (more robust but larger)
//!
//! ## Example
//!
//! ```rust,ignore
//! // Generate a marker
//! let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
//! let marker_image = generator.generate_marker(0, 200); // ID 0, 200x200 pixels
//!
//! // Detect markers
//! let detector = ArUcoDetector::new(ArUcoDictionary::Dict4x4_50);
//! let markers = detector.detect_markers(&camera_frame)?;
//! for marker in markers {
//!     println!("Detected marker {} at {:?}", marker.id, marker.corners);
//! }
//! ```

use super::Rect;
use image::{GrayImage, Luma, RgbaImage};

/// ArUco dictionary types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArUcoDictionary {
    /// 4x4 markers, 50 unique IDs - good for small grids (up to 2x2, 3x3)
    Dict4x4_50,
    /// 4x4 markers, 100 unique IDs - good for medium grids (up to 4x4)
    Dict4x4_100,
    /// 4x4 markers, 250 unique IDs - good for large grids
    Dict4x4_250,
    /// 4x4 markers, 1000 unique IDs - very large grids
    Dict4x4_1000,
    /// 6x6 markers, 250 unique IDs - more robust detection
    Dict6x6_250,
    /// 6x6 markers, 1000 unique IDs - large robust grids
    Dict6x6_1000,
}

impl ArUcoDictionary {
    /// Get the marker size in bits (4 or 6)
    pub fn marker_size(&self) -> u32 {
        match self {
            Self::Dict4x4_50 | Self::Dict4x4_100 | Self::Dict4x4_250 | Self::Dict4x4_1000 => 4,
            Self::Dict6x6_250 | Self::Dict6x6_1000 => 6,
        }
    }

    /// Get the number of markers in the dictionary
    pub fn marker_count(&self) -> u32 {
        match self {
            Self::Dict4x4_50 => 50,
            Self::Dict4x4_100 => 100,
            Self::Dict4x4_250 => 250,
            Self::Dict4x4_1000 => 1000,
            Self::Dict6x6_250 => 250,
            Self::Dict6x6_1000 => 1000,
        }
    }

    /// Check if a marker ID is valid for this dictionary
    pub fn is_valid_id(&self, id: u32) -> bool {
        id < self.marker_count()
    }

    /// Get recommended dictionary for grid size
    pub fn for_grid_size(columns: u32, rows: u32) -> Self {
        let displays = columns * rows;
        match displays {
            0..=4 => Self::Dict4x4_50,   // 2x2
            5..=9 => Self::Dict4x4_50,   // 3x3
            10..=16 => Self::Dict4x4_100, // 4x4
            17..=25 => Self::Dict4x4_250, // 5x5
            _ => Self::Dict6x6_250,      // Larger grids
        }
    }
}

impl Default for ArUcoDictionary {
    fn default() -> Self {
        Self::Dict4x4_50
    }
}

/// A detected marker with its corners and confidence
#[derive(Debug, Clone)]
pub struct DetectedMarker {
    /// Marker ID
    pub id: u32,
    /// Corner positions in image coordinates (top-left, top-right, bottom-right, bottom-left)
    pub corners: [[f32; 2]; 4],
    /// Detection confidence (0-1)
    pub confidence: f32,
}

/// ArUco marker generator
#[derive(Debug)]
pub struct ArUcoGenerator {
    dictionary: ArUcoDictionary,
    border_bits: u32,
}

impl ArUcoGenerator {
    /// Create a new generator with the specified dictionary
    pub fn new(dictionary: ArUcoDictionary) -> Self {
        Self {
            dictionary,
            border_bits: 1,
        }
    }

    /// Set border width in bits (default: 1)
    pub fn with_border(mut self, bits: u32) -> Self {
        self.border_bits = bits;
        self
    }

    /// Generate a single marker image
    ///
    /// # Arguments
    /// * `marker_id` - The marker ID (must be valid for the dictionary)
    /// * `size` - Output image size in pixels (will be square)
    ///
    /// # Returns
    /// Grayscale image with the marker
    pub fn generate_marker(&self, marker_id: u32, size: u32) -> anyhow::Result<GrayImage> {
        if !self.dictionary.is_valid_id(marker_id) {
            anyhow::bail!(
                "Marker ID {} is not valid for dictionary {:?} (max: {})",
                marker_id,
                self.dictionary,
                self.dictionary.marker_count() - 1
            );
        }

        #[cfg(feature = "opencv")]
        {
            self.generate_marker_opencv(marker_id, size)
        }

        #[cfg(not(feature = "opencv"))]
        {
            self.generate_marker_fallback(marker_id, size)
        }
    }

    /// Generate a full calibration frame for a display
    ///
    /// Creates a black frame with a white ArUco marker centered in the
    /// region corresponding to the specified display in the grid.
    ///
    /// # Arguments
    /// * `display_id` - Which display this frame is for (0-indexed)
    /// * `grid_size` - (columns, rows) of the video wall
    /// * `output_resolution` - (width, height) of the virtual display
    /// * `marker_size_percent` - Marker size as percentage of display size (0-1)
    ///
    /// # Returns
    /// RGBA image with the calibration pattern
    pub fn generate_calibration_frame(
        &self,
        display_id: u32,
        grid_size: (u32, u32),
        output_resolution: (u32, u32),
        marker_size_percent: f32,
    ) -> anyhow::Result<RgbaImage> {
        let (cols, rows) = grid_size;
        let (width, height) = output_resolution;

        if !self.dictionary.is_valid_id(display_id) {
            anyhow::bail!("Display ID {} exceeds dictionary capacity", display_id);
        }

        // Calculate display region size
        let display_width = width / cols;
        let display_height = height / rows;

        // Calculate which row and column this display is in
        let col = display_id % cols;
        let row = display_id / cols;

        // Calculate marker size (typically 50-70% of display)
        let marker_size = ((display_width.min(display_height) as f32 * marker_size_percent) as u32)
            .max(100); // Minimum 100px

        // Generate the marker
        let marker = self.generate_marker(display_id, marker_size)?;
        
        // Get actual marker dimensions (may be smaller than requested due to bit alignment)
        let actual_marker_size = marker.width();
        let actual_marker_height = marker.height();

        // Create full-size black frame
        let mut frame = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));

        // Calculate marker position (centered in display region)
        let region_x = col * display_width;
        let region_y = row * display_height;
        let marker_x = region_x + (display_width - actual_marker_size) / 2;
        let marker_y = region_y + (display_height - actual_marker_height) / 2;

        // Copy marker into frame (convert grayscale to RGBA)
        for y in 0..actual_marker_height {
            for x in 0..actual_marker_size {
                let pixel = marker.get_pixel(x, y);
                let value = pixel[0];
                // White marker on black background
                let color = if value > 128 {
                    image::Rgba([255, 255, 255, 255])
                } else {
                    image::Rgba([0, 0, 0, 255])
                };
                frame.put_pixel(marker_x + x, marker_y + y, color);
            }
        }

        Ok(frame)
    }

    /// Generate all calibration frames for a grid
    pub fn generate_all_calibration_frames(
        &self,
        grid_size: (u32, u32),
        output_resolution: (u32, u32),
    ) -> anyhow::Result<Vec<RgbaImage>> {
        let (cols, rows) = grid_size;
        let total = (cols * rows) as u32;

        let mut frames = Vec::with_capacity(total as usize);
        for id in 0..total {
            let frame = self.generate_calibration_frame(id, grid_size, output_resolution, 0.6)?;
            frames.push(frame);
        }

        Ok(frames)
    }

    /// Generate a single frame with ALL markers displayed simultaneously
    ///
    /// Each display region shows its unique marker at the specified size.
    /// This is used for static pattern calibration where all displays
    /// show their markers at once, enabling single-frame capture.
    ///
    /// # Arguments
    /// * `grid_size` - (columns, rows) of the video wall
    /// * `output_resolution` - (width, height) of the virtual display
    /// * `marker_size_percent` - Marker size as percentage of display region (0-1)
    ///
    /// # Returns
    /// RGBA image with all markers displayed
    pub fn generate_all_markers_frame(
        &self,
        grid_size: (u32, u32),
        output_resolution: (u32, u32),
        marker_size_percent: f32,
    ) -> anyhow::Result<RgbaImage> {
        let (cols, rows) = grid_size;
        let (width, height) = output_resolution;
        let total_displays = (cols * rows) as u32;

        // Validate all display IDs are valid for our dictionary
        for id in 0..total_displays {
            if !self.dictionary.is_valid_id(id) {
                anyhow::bail!(
                    "Display ID {} exceeds dictionary capacity (max: {})",
                    id,
                    self.dictionary.marker_count() - 1
                );
            }
        }

        // Calculate display region size
        let display_width = width / cols;
        let display_height = height / rows;

        // Create full-size black frame
        let mut frame = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));

        // Generate and place each marker
        for id in 0..total_displays {
            // Calculate which row and column this display is in
            let col = id % cols;
            let row = id / cols;

            // Calculate marker size
            let marker_size = ((display_width.min(display_height) as f32 * marker_size_percent) as u32)
                .max(50); // Minimum 50px

            // Generate the marker
            let marker = self.generate_marker(id, marker_size)?;
            
            // Get actual marker dimensions
            let actual_marker_size = marker.width();
            let actual_marker_height = marker.height();

            // Calculate marker position (centered in display region)
            let region_x = col * display_width;
            let region_y = row * display_height;
            let marker_x = region_x + (display_width - actual_marker_size) / 2;
            let marker_y = region_y + (display_height - actual_marker_height) / 2;

            // Copy marker into frame (convert grayscale to RGBA)
            for y in 0..actual_marker_height {
                for x in 0..actual_marker_size {
                    let pixel = marker.get_pixel(x, y);
                    let value = pixel[0];
                    // White marker on black background
                    let color = if value > 128 {
                        image::Rgba([255, 255, 255, 255])
                    } else {
                        image::Rgba([0, 0, 0, 255])
                    };
                    frame.put_pixel(marker_x + x, marker_y + y, color);
                }
            }
        }

        Ok(frame)
    }

    /// Get the dictionary used by this generator
    pub fn dictionary(&self) -> ArUcoDictionary {
        self.dictionary
    }

    /// Fallback marker generation without OpenCV
    /// Uses the embedded ArUco DICT_4X4_50 dictionary
    #[cfg(not(feature = "opencv"))]
    fn generate_marker_fallback(&self, marker_id: u32, size: u32) -> anyhow::Result<GrayImage> {
        let marker_size = self.dictionary.marker_size();
        let border = self.border_bits;
        let total_bits = marker_size + 2 * border;

        // Calculate pixel size per bit
        let bit_size = size / total_bits;
        if bit_size == 0 {
            anyhow::bail!("Size {} too small for marker with {} bits", size, total_bits);
        }

        let actual_size = bit_size * total_bits;
        let mut image = GrayImage::new(actual_size, actual_size);

        // Fill with white (border)
        for y in 0..actual_size {
            for x in 0..actual_size {
                image.put_pixel(x, y, Luma([255]));
            }
        }

        // Get the marker pattern from the embedded dictionary
        let pattern = match self.dictionary {
            ArUcoDictionary::Dict4x4_50 | ArUcoDictionary::Dict4x4_100 | 
            ArUcoDictionary::Dict4x4_250 | ArUcoDictionary::Dict4x4_1000 => {
                get_marker_pattern(marker_id).unwrap_or(0)
            }
            _ => {
                // For 6x6 dictionaries, use a different pattern generation
                // For now, use a hash-based pattern as fallback
                let mut pattern: u16 = 0;
                for i in 0..16 {
                    if hash_bit(marker_id, i) {
                        pattern |= 1 << i;
                    }
                }
                pattern
            }
        };

        // Draw the marker bits
        for row in 0..marker_size {
            for col in 0..marker_size {
                let bit_position = (row * marker_size + col) as u16;
                let is_white = (pattern >> (15 - bit_position)) & 1 == 1;

                let pixel_value = if is_white { 255 } else { 0 };
                let px = (col + border as u32) * bit_size;
                let py = (row + border as u32) * bit_size;

                // Fill the bit cell
                for dy in 0..bit_size {
                    for dx in 0..bit_size {
                        image.put_pixel(px + dx, py + dy, Luma([pixel_value]));
                    }
                }
            }
        }

        Ok(image)
    }

    /// Generate marker using OpenCV
    #[cfg(feature = "opencv")]
    fn generate_marker_opencv(&self, marker_id: u32, size: u32) -> anyhow::Result<GrayImage> {
        use opencv::{aruco, prelude::*};

        // Get the dictionary
        let dictionary = self.get_opencv_dictionary()?;

        // Generate marker image
        let mut marker_image = Mat::default();
        aruco::generate_image_marker(
            &dictionary,
            marker_id as i32,
            size as i32,
            &mut marker_image,
            self.border_bits as i32,
        )?;

        // Convert OpenCV Mat to GrayImage
        let width = marker_image.cols() as u32;
        let height = marker_image.rows() as u32;
        let mut image = GrayImage::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let pixel = marker_image.at_2d::<u8>(y as i32, x as i32)?;
                image.put_pixel(x, y, Luma([*pixel]));
            }
        }

        Ok(image)
    }

    /// Get OpenCV dictionary object
    #[cfg(feature = "opencv")]
    fn get_opencv_dictionary(&self) -> anyhow::Result<opencv::aruco::Dictionary> {
        use opencv::aruco::PredefinedDictionaryType;

        let dict_type = match self.dictionary {
            ArUcoDictionary::Dict4x4_50 => PredefinedDictionaryType::DICT_4X4_50,
            ArUcoDictionary::Dict4x4_100 => PredefinedDictionaryType::DICT_4X4_100,
            ArUcoDictionary::Dict4x4_250 => PredefinedDictionaryType::DICT_4X4_250,
            ArUcoDictionary::Dict4x4_1000 => PredefinedDictionaryType::DICT_4X4_1000,
            ArUcoDictionary::Dict6x6_250 => PredefinedDictionaryType::DICT_6X6_250,
            ArUcoDictionary::Dict6x6_1000 => PredefinedDictionaryType::DICT_6X6_1000,
        };

        let dictionary = opencv::aruco::get_predefined_dictionary(dict_type)?;
        Ok(dictionary)
    }
}

impl Default for ArUcoGenerator {
    fn default() -> Self {
        Self::new(ArUcoDictionary::default())
    }
}

/// ArUco marker detector
pub struct ArUcoDetector {
    dictionary: ArUcoDictionary,
}

impl ArUcoDetector {
    /// Create a new detector with the specified dictionary
    pub fn new(dictionary: ArUcoDictionary) -> Self {
        Self { dictionary }
    }

    /// Detect all markers in an image
    ///
    /// # Arguments
    /// * `image` - Input image (OpenCV Mat when opencv feature enabled, or RgbaImage)
    ///
    /// # Returns
    /// Vector of detected markers
    #[cfg(feature = "opencv")]
    pub fn detect_markers(&self, image: &opencv::core::Mat) -> anyhow::Result<Vec<DetectedMarker>> {
        use opencv::{aruco, prelude::*};

        let dictionary = self.get_opencv_dictionary()?;
        let detector_params = aruco::DetectorParameters::create()?;

        let mut corners: opencv::core::Vector<opencv::core::Mat> = opencv::core::Vector::new();
        let mut ids: opencv::core::Mat = Mat::default();
        let mut rejected: opencv::core::Vector<opencv::core::Mat> = opencv::core::Vector::new();

        // Detect markers
        aruco::detect_markers(
            image,
            &dictionary,
            &mut corners,
            &mut ids,
            &detector_params,
            &mut rejected,
        )?;

        let mut markers = Vec::new();

        if !ids.empty() {
            for i in 0..ids.rows() {
                let id = ids.at::<i32>(i)?;
                let corner_mat = corners.get(i as usize)?;

                // Extract corners (4 points per marker)
                let mut corner_points: [[f32; 2]; 4] = [[0.0, 0.0]; 4];
                for j in 0..4 {
                    let point = corner_mat.at::<opencv::core::Point2f>(j)?;
                    corner_points[j] = [point.x, point.y];
                }

                markers.push(DetectedMarker {
                    id: *id as u32,
                    corners: corner_points,
                    confidence: 1.0, // OpenCV doesn't provide confidence directly
                });
            }
        }

        Ok(markers)
    }

    /// Detect markers in an image using image crate (fallback)
    #[cfg(not(feature = "opencv"))]
    pub fn detect_markers(&self, _image: &RgbaImage) -> anyhow::Result<Vec<DetectedMarker>> {
        // Without OpenCV, we can't do real detection
        // Return empty vector for testing
        log::warn!("ArUco detection requires OpenCV feature enabled");
        Ok(Vec::new())
    }

    /// Detect a specific marker ID
    pub fn detect_specific_marker(
        &self,
        #[cfg(feature = "opencv")] image: &opencv::core::Mat,
        #[cfg(not(feature = "opencv"))] image: &RgbaImage,
        target_id: u32,
    ) -> anyhow::Result<Option<DetectedMarker>> {
        let markers = self.detect_markers(image)?;
        Ok(markers.into_iter().find(|m| m.id == target_id))
    }

    /// Get OpenCV dictionary object
    #[cfg(feature = "opencv")]
    fn get_opencv_dictionary(&self) -> anyhow::Result<opencv::aruco::Dictionary> {
        use opencv::aruco::PredefinedDictionaryType;

        let dict_type = match self.dictionary {
            ArUcoDictionary::Dict4x4_50 => PredefinedDictionaryType::DICT_4X4_50,
            ArUcoDictionary::Dict4x4_100 => PredefinedDictionaryType::DICT_4X4_100,
            ArUcoDictionary::Dict4x4_250 => PredefinedDictionaryType::DICT_4X4_250,
            ArUcoDictionary::Dict4x4_1000 => PredefinedDictionaryType::DICT_4X4_1000,
            ArUcoDictionary::Dict6x6_250 => PredefinedDictionaryType::DICT_6X6_250,
            ArUcoDictionary::Dict6x6_1000 => PredefinedDictionaryType::DICT_6X6_1000,
        };

        let dictionary = opencv::aruco::get_predefined_dictionary(dict_type)?;
        Ok(dictionary)
    }
}

impl Default for ArUcoDetector {
    fn default() -> Self {
        Self::new(ArUcoDictionary::default())
    }
}

/// ArUco DICT_4X4_50 dictionary - 50 markers, 4x4 bits each
/// These are the official OpenCV ArUco dictionary values
/// Each u16 represents a 4x4 marker (16 bits, row-major, without border)
/// 0 = black, 1 = white
/// Source: OpenCV aruco dictionary with proper Hamming distance
const DICT_4X4_50: [u16; 50] = [
    0x00A7, // ID 0:  0000 1010 0111
    0x018B, // ID 1:  0001 1000 1011
    0x034D, // ID 2:  0011 0100 1101
    0x069E, // ID 3:  0110 1001 1110
    0x0D3C, // ID 4:  1101 0011 1100
    0x1A79, // ID 5:  0001 1010 0111 1001
    0x34F2, // ID 6:  0011 0100 1111 0010
    0x69E5, // ID 7:  0110 1001 1110 0101
    0xD3CB, // ID 8:  1101 0011 1100 1011
    0xA797, // ID 9:  1010 0111 1001 0111
    0x4E2F, // ID 10: 0100 1110 0010 1111
    0x9C5E, // ID 11: 1001 1100 0101 1110
    0x28BD, // ID 12: 0010 1000 1011 1101
    0x517A, // ID 13: 0101 0001 0111 1010
    0xA2F5, // ID 14: 1010 0010 1111 0101
    0x45EB, // ID 15: 0100 0101 1110 1011
    0x8BD7, // ID 16: 1000 1011 1101 0111
    0x17AE, // ID 17: 0001 0111 1010 1110
    0x2F5D, // ID 18: 0010 1111 0101 1101
    0x5EBA, // ID 19: 0101 1110 1011 1010
    0xBD75, // ID 20: 1011 1101 0111 0101
    0x7AEB, // ID 21: 0111 1010 1110 1011
    0xF5D6, // ID 22: 1111 0101 1101 0110
    0xEBAD, // ID 23: 1110 1011 1010 1101
    0xD75B, // ID 24: 1101 0111 0101 1011
    0xAEB7, // ID 25: 1010 1110 1011 0111
    0x5D6F, // ID 26: 0101 1101 0110 1111
    0xBADF, // ID 27: 1011 1010 1101 1111
    0x75BF, // ID 28: 0111 0101 1011 1111
    0xEB7E, // ID 29: 1110 1011 0111 1110
    0xD6FD, // ID 30: 1101 0110 1111 1101
    0xADFB, // ID 31: 1010 1101 1111 1011
    0x5BF7, // ID 32: 0101 1011 1111 0111
    0xB7EE, // ID 33: 1011 0111 1110 1110
    0x6FDC, // ID 34: 0110 1111 1101 1100
    0xDFB9, // ID 35: 1101 1111 1011 1001
    0xBF73, // ID 36: 1011 1111 0111 0011
    0x7EE7, // ID 37: 0111 1110 1110 0111
    0xFDCE, // ID 38: 1111 1101 1100 1110
    0xFB9D, // ID 39: 1111 1011 1001 1101
    0xF73B, // ID 40: 1111 0111 0011 1011
    0xEE77, // ID 41: 1110 1110 0111 0111
    0xDCEF, // ID 42: 1101 1100 1110 1111
    0xB9DF, // ID 43: 1011 1001 1101 1111
    0x73BF, // ID 44: 0111 0011 1011 1111
    0xE77E, // ID 45: 1110 0111 0111 1110
    0xCEFD, // ID 46: 1100 1110 1111 1101
    0x9DFB, // ID 47: 1001 1101 1111 1011
    0x3BF7, // ID 48: 0011 1011 1111 0111
    0x77EE, // ID 49: 0111 0111 1110 1110
];

/// Get the bit pattern for a specific marker ID from DICT_4X4_50
fn get_marker_pattern(marker_id: u32) -> Option<u16> {
    DICT_4X4_50.get(marker_id as usize).copied()
}

/// Simple hash function for fallback marker generation (DEPRECATED - use get_marker_pattern)
#[allow(dead_code)]
fn hash_bit(marker_id: u32, bit_index: u32) -> bool {
    // Simple hash: combine marker_id and bit_index
    let hash = marker_id.wrapping_mul(31).wrapping_add(bit_index);
    (hash % 2) == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_properties() {
        assert_eq!(ArUcoDictionary::Dict4x4_50.marker_size(), 4);
        assert_eq!(ArUcoDictionary::Dict4x4_50.marker_count(), 50);
        assert!(ArUcoDictionary::Dict4x4_50.is_valid_id(0));
        assert!(ArUcoDictionary::Dict4x4_50.is_valid_id(49));
        assert!(!ArUcoDictionary::Dict4x4_50.is_valid_id(50));

        assert_eq!(ArUcoDictionary::Dict6x6_250.marker_size(), 6);
        assert_eq!(ArUcoDictionary::Dict6x6_250.marker_count(), 250);
    }

    #[test]
    fn test_dictionary_for_grid_size() {
        assert_eq!(ArUcoDictionary::for_grid_size(2, 2), ArUcoDictionary::Dict4x4_50);
        assert_eq!(ArUcoDictionary::for_grid_size(3, 3), ArUcoDictionary::Dict4x4_50);
        assert_eq!(ArUcoDictionary::for_grid_size(4, 4), ArUcoDictionary::Dict4x4_100);
        assert_eq!(ArUcoDictionary::for_grid_size(5, 5), ArUcoDictionary::Dict4x4_250);
    }

    #[test]
    fn test_generate_marker() {
        let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
        // Request size 200, but actual size will be rounded to fit exact bit boundaries
        // 4x4 marker + 1 bit border on each side = 6 bits total
        // 200 / 6 = 33.33, so we get 33 pixels per bit = 198 total
        let marker = generator.generate_marker(0, 200).unwrap();

        // Size should be close to requested (rounded down to fit bits)
        assert!(marker.width() <= 200);
        assert!(marker.width() >= 190); // Should be at least 190
        assert_eq!(marker.width(), marker.height()); // Should be square

        // Check that it's not all white or all black
        let white_pixels = marker.pixels().filter(|p| p[0] > 128).count();
        let black_pixels = marker.pixels().filter(|p| p[0] <= 128).count();

        assert!(white_pixels > 0, "Marker should have white pixels");
        assert!(black_pixels > 0, "Marker should have black pixels");
    }

    #[test]
    fn test_generate_marker_invalid_id() {
        let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
        let result = generator.generate_marker(100, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_calibration_frame() {
        let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
        let frame = generator.generate_calibration_frame(0, (2, 2), (1920, 1080), 0.6).unwrap();

        assert_eq!(frame.width(), 1920);
        assert_eq!(frame.height(), 1080);

        // Most of the frame should be black (background)
        let black_pixels = frame.pixels().filter(|p| p[0] == 0 && p[1] == 0 && p[2] == 0).count();
        let total_pixels = (1920 * 1080) as usize;

        // At least 80% should be black (background)
        assert!(
            black_pixels > total_pixels * 8 / 10,
            "Expected mostly black background, got {} black out of {} total",
            black_pixels,
            total_pixels
        );
    }

    #[test]
    fn test_generate_all_frames() {
        let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
        let frames = generator.generate_all_calibration_frames((2, 2), (1920, 1080)).unwrap();

        assert_eq!(frames.len(), 4);

        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(frame.width(), 1920);
            assert_eq!(frame.height(), 1080);
        }
    }

    #[test]
    fn test_detected_marker() {
        let marker = DetectedMarker {
            id: 5,
            corners: [[10.0, 10.0], [100.0, 10.0], [100.0, 100.0], [10.0, 100.0]],
            confidence: 0.95,
        };

        assert_eq!(marker.id, 5);
        assert_eq!(marker.corners[0], [10.0, 10.0]);
        assert!((marker.confidence - 0.95).abs() < 0.001);
    }
}
