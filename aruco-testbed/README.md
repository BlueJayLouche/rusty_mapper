# ArUco Marker Testbed

A basic ArUco marker generation and detection test suite.

## Quick Start (Python - Working)

The Python implementation uses OpenCV's built-in ArUco module and works immediately.

```bash
# Generate 5 markers (IDs 0-4)
python3 generate.py

# Detect markers in an image
python3 detect.py markers/marker_2.png

# Test with multiple markers
python3 test_multi.py
```

### Python Files

| File | Purpose |
|------|---------|
| `generate.py` | Generates ArUco markers using DICT_6X6_250 dictionary |
| `detect.py` | Detects markers and draws green bounding boxes with red IDs |
| `test_multi.py` | Creates a test scene with 3 markers for multi-detection testing |

### How Marker Detection Works

1. **Dictionary**: Uses `DICT_6X6_250` (6x6 bit markers, 250 unique IDs)
2. **Padding**: Generated markers include 50px white padding for reliable detection
3. **Detection**: The `ArucoDetector` finds markers by detecting the black border pattern
4. **Output**: Detection draws green bounding boxes and labels markers with red text

### Output Files

- `markers/marker_*.png` - Generated markers (300x300px with padding)
- `detection_result.png` - Last detection output
- `test_multi_result.png` - Multi-marker detection result

---

## Rust OpenCV Status

**Problem**: The `opencv` crate (Rust bindings) has compatibility issues with OpenCV 4.13.

**Error**: Binding generator fails with "internal error: entered unreachable code" when processing OpenCV 4.13 headers.

### Root Cause

The `opencv` crate v0.98 officially supports up to OpenCV 4.8. OpenCV 4.13 (brew current) introduces API changes that break the binding generator.

---

## Rust Options

### Option 1: Downgrade OpenCV (Quick Fix)

Install an older OpenCV version that's compatible with the Rust crate:

```bash
# Uninstall current OpenCV
brew uninstall opencv

# Install OpenCV 4.8 (last compatible version)
brew install opencv@4.8
# Or build from source with specific version
```

Then set environment variables before building:
```bash
export DYLD_LIBRARY_PATH="/usr/local/Cellar/llvm/22.1.0/lib:$DYLD_LIBRARY_PATH"
export PKG_CONFIG_PATH="/usr/local/opt/opencv@4.8/lib/pkgconfig:$PKG_CONFIG_PATH"
cargo build
```

### Option 2: Use AprilTags Instead

AprilTags are similar to ArUco but have better Rust support:

```toml
[dependencies]
apriltag = "0.4"
apriltag-image = "0.4"
```

AprilTags are more robust and have pure-Rust implementations.

### Option 3: Use Pure Rust Computer Vision

Switch to a pure Rust vision library that doesn't require OpenCV:

```toml
[dependencies]
image = "0.25"
imageproc = "0.25"
```

Then implement marker detection manually or use QR code detection (which is built into `imageproc`).

### Option 4: Python FFI Bridge

Keep detection logic in Python and call it from Rust:

```rust
use std::process::Command;

fn detect_markers(image_path: &str) -> Vec<Marker> {
    let output = Command::new("python3")
        .arg("detect.py")
        .arg(image_path)
        .output()
        .expect("Failed to execute Python");
    // Parse output...
}
```

### Option 5: ONNX Runtime / Deep Learning

Use a pre-trained neural network for marker detection:

```toml
[dependencies]
onnxruntime = "0.0.14"
```

This avoids OpenCV entirely but requires training or finding a suitable model.

---

## Recommended Approach

**For immediate development**: Use **Option 1** (downgrade OpenCV to 4.8) or **Option 2** (switch to AprilTags).

**For production**: Consider **Option 2** (AprilTags) as they have better cross-platform Rust support and are actively maintained.
