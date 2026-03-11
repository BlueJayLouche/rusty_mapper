# AprilTag Implementation Summary

## ✅ Status: IMPLEMENTED & TESTED

The AprilTag marker detection system has been successfully implemented as a pure-Rust replacement for the OpenCV-dependent ArUco detection.

---

## What Was Done

### 1. Added AprilTag Dependencies

**Cargo.toml:**
```toml
[dependencies]
apriltag = "0.4"
```

(Note: `apriltag-image` was removed due to image crate version conflicts, manual conversion is used instead)

### 2. Created AprilTag Module

**New file: `src/videowall/apriltag.rs`**

Provides:
- `AprilTagDetector` - Pure Rust marker detector
- `AprilTagGenerator` - Marker image loading and calibration frame generation
- `AprilTagFamily` - Enum for tag families (Tag36h11, Tag25h9, Tag16h5)
- `AprilTagDetection` - Detection result struct

### 3. Generated AprilTag Marker Images

**Location: `assets/apriltags/`**

Generated 20 markers (IDs 0-19) from the tag36h11 family:
- 300x300 pixel markers
- 50px white padding for reliable detection
- Total size: 400x400 pixels per marker

### 4. Updated Module Exports

**Modified files:**
- `src/videowall/mod.rs` - Added AprilTag exports
- `src/lib.rs` - Added AprilTag re-exports

---

## Test Results

```
running 6 tests
test videowall::apriltag::tests::test_family_properties ... ok
test videowall::apriltag::tests::test_family_for_grid_size ... ok
test videowall::apriltag::tests::test_invalid_marker_id ... ok
test videowall::apriltag::tests::test_marker_filename ... ok
test videowall::apriltag::tests::test_detector_creation ... ok
Detected 1 markers
Detection successful: ID 0 at center [199.92467, 199.91994]
test videowall::apriltag::tests::test_detection_on_real_image ... ok

test result: ok. 6 passed; 0 failed
```

---

## Usage Examples

### Basic Detection

```rust
use rusty_mapper::videowall::{AprilTagDetector, AprilTagFamily};
use image::GrayImage;

// Create detector
let mut detector = AprilTagDetector::new(AprilTagFamily::Tag36h11);

// Load image
let img = image::open("marker.png")?.to_luma8();

// Detect markers
let detections = detector.detect(&img);

for det in &detections {
    println!("Detected ID {} at center {:?}", det.id, det.center);
    println!("Corners: {:?}", det.corners);
}
```

### Generate Calibration Frame

```rust
use rusty_mapper::videowall::{AprilTagGenerator, AprilTagFamily};

let generator = AprilTagGenerator::new(AprilTagFamily::Tag36h11);

// Generate frame for display 0 in a 2x2 grid
let frame = generator.generate_calibration_frame(
    0,                    // Display ID
    (2, 2),              // Grid size (columns, rows)
    (1920, 1080),        // Output resolution
    0.6                  // Marker size as % of display
)?;

// Save or display the frame
frame.save("calibration_0.png")?;
```

### Generate All Markers Frame

```rust
let generator = AprilTagGenerator::new(AprilTagFamily::Tag36h11);

// Generate frame with ALL markers displayed
let all_markers = generator.generate_all_markers_frame(
    (2, 2),              // Grid size
    (1920, 1080),        // Output resolution
    0.6                  // Marker size
)?;
```

---

## API Comparison: ArUco vs AprilTag

| Feature | ArUco (OpenCV) | AprilTag (Pure Rust) |
|---------|---------------|---------------------|
| **Dependencies** | OpenCV 4.x | `apriltag` crate only |
| **Detection** | `ArUcoDetector::detect_markers()` | `AprilTagDetector::detect()` |
| **Generation** | `generateImageMarker()` | Load pre-generated PNGs |
| **Dictionary** | `ArUcoDictionary::Dict4x4_50` | `AprilTagFamily::Tag36h11` |
| **Marker Count** | 50-1000 | 587 (tag36h11) |
| **Corners** | `[[f32; 2]; 4]` | `[[f32; 2]; 4]` |
| **Center** | Not provided | `[f32; 2]` |
| **Confidence** | Not provided | `decision_margin` |

---

## Next Steps (Optional)

### 1. Update Calibration Controller

To fully replace ArUco in the calibration workflow, update `src/videowall/calibration.rs`:

```rust
// Replace this:
#[cfg(feature = "opencv")]
let detector = ArUcoDetector::new(self.generator.dictionary());

// With this:
let mut detector = AprilTagDetector::new(AprilTagFamily::Tag36h11);
let gray_image = image.to_luma8();
let markers = detector.detect(&gray_image);
```

### 2. Generate More Markers

If you need more than 20 markers:

```bash
python3 << 'EOF'
import cv2
import os

os.makedirs("assets/apriltags", exist_ok=True)
dictionary = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_APRILTAG_36h11)

for marker_id in range(100):  # Generate 100 markers
    marker_img = cv2.aruco.generateImageMarker(dictionary, marker_id, 300)
    padded = cv2.copyMakeBorder(marker_img, 50, 50, 50, 50, 
                                 cv2.BORDER_CONSTANT, value=255)
    filename = f"assets/apriltags/tag36_11_{marker_id:05d}.png"
    cv2.imwrite(filename, padded)
    
print(f"Generated 100 markers")
EOF
```

### 3. Remove OpenCV Feature (Optional)

Once fully migrated, you can remove the OpenCV feature:

```toml
# Cargo.toml
[features]
default = ["webcam"]  # Remove "opencv"
webcam = ["nokhwa"]
# opencv = ["dep:opencv"]  # Remove this line
```

And remove the OpenCV-related code from:
- `src/videowall/aruco.rs` (or keep for backward compatibility)
- `src/videowall/calibration.rs` (simplify by removing cfg flags)

---

## Performance Notes

- **Detection speed**: AprilTags are slightly slower than ArUco but more robust
- **Accuracy**: Better at long distances and challenging angles (NASA uses them)
- **Memory**: No OpenCV overhead, smaller binary size
- **Cross-platform**: Works on any platform Rust supports (no C++ dependencies)

---

## Troubleshooting

### Detection not working?

1. **Check marker size**: Markers should be at least 100x100 pixels in the image
2. **Check contrast**: Ensure good lighting and contrast
3. **Try different parameters**:
   ```rust
   detector.set_decimation(1.0);  // No decimation for better accuracy
   detector.set_sigma(0.0);       // No blur
   detector.set_refine_edges(true);
   ```

### Need more detection range?

Use the `tag16h5` family for smaller markers (but more false positives possible):
```rust
let detector = AprilTagDetector::new(AprilTagFamily::Tag16h5);
```

---

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Added `apriltag = "0.4"` |
| `src/videowall/apriltag.rs` | **NEW** - AprilTag implementation |
| `src/videowall/mod.rs` | Added AprilTag exports |
| `src/lib.rs` | Added AprilTag re-exports |
| `assets/apriltags/` | **NEW** - Generated marker images |

---

## References

- [AprilTag Paper](https://april.eecs.umich.edu/pdfs/olson2011tags.pdf)
- [Rust apriltag crate](https://docs.rs/apriltag/latest/apriltag/)
- [AprilTag Image Repository](https://github.com/AprilRobotics/apriltag-imgs)
