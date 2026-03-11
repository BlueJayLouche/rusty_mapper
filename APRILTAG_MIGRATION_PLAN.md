# AprilTag Migration Plan for Rusty Mapper

## Executive Summary

**Problem**: OpenCV 4.13 breaks the Rust `opencv` crate (v0.98), preventing ArUco marker detection in Rusty Mapper.

**Solution**: Migrate from ArUco to AprilTags using pure-Rust libraries.

**Why AprilTags?**
- ✅ Pure Rust implementation (no OpenCV dependency)
- ✅ More robust detection (used by NASA, robotics industry)
- ✅ Better long-distance detection
- ✅ Active maintenance
- ✅ Similar API to ArUco
- ⚠️ Different marker format (not compatible with existing ArUco markers)

---

## Technical Comparison

| Feature | ArUco | AprilTags |
|---------|-------|-----------|
| **Rust Support** | ❌ Requires OpenCV | ✅ Pure Rust (`apriltag` crate) |
| **Detection Range** | Good | Better (longer range) |
| **False Positive Rate** | Low | Lower |
| **Processing Speed** | Fast | Slightly slower |
| **Marker Count** | 50-1000 per dictionary | 587 (36h11 family) |
| **QR Code Compatible** | No | No |
| **Industry Usage** | Academic | NASA, industry standard |

---

## Migration Strategy

### Phase 1: Add AprilTag Support (Parallel)

1. Add AprilTag dependencies
2. Create `AprilTagDetector` wrapper
3. Generate AprilTag marker images
4. Test detection

### Phase 2: Replace ArUco (Switch)

1. Replace `ArUcoDetector` with `AprilTagDetector` in calibration
2. Update marker generation to use AprilTags
3. Update documentation
4. Remove OpenCV feature flag

---

## Implementation Details

### 1. Dependencies

```toml
[dependencies]
apriltag = "0.4"
apriltag-image = "0.1"
```

### 2. AprilTag Detector Wrapper

```rust
use apriltag::{Detector, Family, Image};
use image::GrayImage;

pub struct AprilTagDetector {
    detector: Detector,
}

impl AprilTagDetector {
    pub fn new() -> Self {
        let mut detector = Detector::new();
        detector.add_family(Family::tag_36h11());
        Self { detector }
    }
    
    pub fn detect(&self, image: &GrayImage) -> Vec<AprilTagDetection> {
        let (width, height) = image.dimensions();
        let apriltag_image = Image::from_buffer(
            width as usize, 
            height as usize, 
            image.as_raw()
        );
        
        self.detector.detect(&apriltag_image)
            .into_iter()
            .map(|det| AprilTagDetection {
                id: det.id() as u32,
                center: [det.center().x() as f32, det.center().y() as f32],
                corners: det.corners().iter()
                    .map(|c| [c.x() as f32, c.y() as f32])
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap(),
            })
            .collect()
    }
}
```

### 3. Marker Generation

AprilTags use pre-generated marker images. Options:

**Option A: Download from AprilRobotics**
```bash
# Download all markers from official repo
curl -L https://github.com/AprilRobotics/apriltag-imgs/archive/refs/heads/master.zip -o apriltags.zip
unzip apriltags.zip
```

**Option B: Generate with Python (one-time)**
```python
import cv2
import os

# AprilTag 36h11 family (most common)
dictionary = cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_APRILTAG_36h11)
os.makedirs("apriltags", exist_ok=True)

for marker_id in range(30):  # Generate first 30
    marker = cv2.aruco.generateImageMarker(dictionary, marker_id, 300)
    cv2.imwrite(f"apriltags/tag36_11_{marker_id:05d}.png", marker)
```

**Option C: Include pre-generated markers in repo**
Add marker images directly to `assets/apriltags/` directory.

### 4. Calibration Controller Update

Replace `ArUcoDetector` usage:

```rust
// Before (ArUco)
#[cfg(feature = "opencv")]
let detector = ArUcoDetector::new(ArUcoDictionary::Dict4x4_50);
let markers = detector.detect_markers(&mat)?;

// After (AprilTag)
let detector = AprilTagDetector::new();
let gray_image = image.to_luma8();
let markers = detector.detect(&gray_image);
```

---

## File Changes Required

### Modified Files

| File | Changes |
|------|---------|
| `Cargo.toml` | Remove `opencv` dep, add `apriltag`, `apriltag-image` |
| `src/videowall/aruco.rs` | Replace or add `AprilTagGenerator`/`AprilTagDetector` |
| `src/videowall/calibration.rs` | Use `AprilTagDetector` instead of `ArUcoDetector` |
| `src/videowall/mod.rs` | Update exports |
| `README.md` | Update documentation |

### New Files

| File | Purpose |
|------|---------|
| `assets/apriltags/*.png` | Pre-generated marker images |
| `src/videowall/apriltag.rs` | AprilTag wrapper module |

---

## Testing Plan

### Unit Tests
```rust
#[test]
fn test_apriltag_detection() {
    let detector = AprilTagDetector::new();
    let image = image::open("assets/apriltags/tag36_11_00000.png")
        .unwrap()
        .to_luma8();
    
    let detections = detector.detect(&image);
    assert_eq!(detections.len(), 1);
    assert_eq!(detections[0].id, 0);
}
```

### Integration Tests
1. Generate calibration pattern with AprilTags
2. Display on screen
3. Capture with camera
4. Verify detection

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| AprilTag crate compatibility | High | Test with current Rust version first |
| Different marker format | Medium | Generate new markers, update docs |
| Performance regression | Low | Benchmark vs ArUco, optimize if needed |
| Detection accuracy changes | Medium | Test with real projection setup |

---

## Timeline Estimate

| Phase | Duration | Tasks |
|-------|----------|-------|
| Research & Setup | 2 hours | Verify crate works, generate markers |
| Implementation | 4 hours | Write wrapper, update calibration |
| Testing | 2 hours | Unit tests, integration tests |
| Documentation | 1 hour | Update README, add migration guide |
| **Total** | **~1 day** | |

---

## Quick Start (Proof of Concept)

```bash
# 1. Add dependencies
cargo add apriltag apriltag-image

# 2. Create test script
cat > /tmp/test_apriltag.rs << 'EOF'
use apriltag::{Detector, Family, Image};

fn main() {
    let img = image::open("/Users/alpha/Developer/rust/rusty_mapper/aruco-testbed/markers/marker_0.png")
        .unwrap()
        .to_luma8();
    
    let (w, h) = img.dimensions();
    let apriltag_img = Image::from_buffer(w as usize, h as usize, img.as_raw());
    
    let mut detector = Detector::new();
    detector.add_family(Family::tag_36h11());
    
    let detections = detector.detect(&apriltag_img);
    println!("Detected {} markers", detections.len());
}
EOF

# Note: This won't detect ArUco markers - need actual AprilTag images
```

---

## References

- [AprilTag Paper](https://april.eecs.umich.edu/pdfs/olson2011tags.pdf)
- [AprilTag Images Repository](https://github.com/AprilRobotics/apriltag-imgs)
- [Rust apriltag crate](https://docs.rs/apriltag/latest/apriltag/)
- [ArUco vs AprilTag comparison](https://docs.pupil-labs.com/alpha-lab/apriltag-family-compatibility/)
