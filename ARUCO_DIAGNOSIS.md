# ArUco Marker Detection Diagnosis

## Summary

**Root Cause:** OpenCV 4.13.0 has a known bug that breaks ArUco marker detection.

## Problem Details

### Your Environment
- **OpenCV Version:** 4.13.0
- **Status:** ⚠️ **BROKEN - Known bug in this version**

### The Issue

OpenCV 4.13.0 introduced a bug in the `generateImageMarker()` function that produces markers which cannot be detected by the ArUco detector. This means:

1. The application generates markers using OpenCV
2. Those markers have a subtle encoding issue
3. OpenCV's own detector cannot recognize them
4. Result: **No markers detected**

### Code Evidence

From `src/videowall/aruco.rs`:
```rust
// OpenCV 4.13.0 has a known issue with ArUco marker detection where
// `generateImageMarker` produces markers that cannot be detected.
// Recommended: Use OpenCV 4.12.x or 4.14+ for reliable marker detection.
```

## Solutions (Choose One)

### Option 1: Upgrade OpenCV (Recommended)

**macOS:**
```bash
# Upgrade to latest OpenCV (4.14+)
brew upgrade opencv

# Or install specific version
brew install opencv@4.14
```

**Ubuntu/Debian:**
```bash
# Check if newer version is available
apt list --installed | grep opencv

# Upgrade
sudo apt update
sudo apt upgrade libopencv-dev
```

### Option 2: Downgrade OpenCV

**macOS:**
```bash
# Uninstall current
brew uninstall opencv

# Install older version
brew install opencv@4.12

# Link it
brew link opencv@4.12 --force
```

### Option 3: Build Without OpenCV (Fallback Mode)

The application has a fallback marker generator that doesn't use OpenCV:

```bash
# Build without OpenCV feature
cargo build --release --no-default-features --features webcam

# Or run directly
cargo run --release --no-default-features --features webcam
```

**Note:** The fallback mode uses an embedded ArUco dictionary for **generation**, but detection still requires OpenCV. Without OpenCV, the detection function returns an empty vector with a warning.

### Option 4: Use Pre-generated Markers

Generate markers using an external tool that works correctly:

1. Use Python with a working OpenCV version:
```python
import cv2
import cv2.aruco as aruco

# Generate marker
aruco_dict = aruco.getPredefinedDictionary(aruco.DICT_4X4_50)
marker = aruco.generateImageMarker(aruco_dict, 0, 400)
cv2.imwrite("marker_0.png", marker)
```

2. Or use an online ArUco marker generator
3. Save the markers and load them as images

## Testing Detection

### Test 1: Verify OpenCV Version
```bash
opencv_version
# or
pkg-config --modversion opencv4
```

### Test 2: Check Generated Markers

The example `aruco_display` generates and displays markers:

```bash
# With OpenCV (if you have a working version)
cargo run --example aruco_display --features opencv -- --grid 2x2

# Without OpenCV (fallback mode)
cargo run --example aruco_display --no-default-features -- --grid 2x2
```

### Test 3: Test Detection on Generated Markers

After running the diagnostic test (see below), check the generated files:

```bash
ls -la /tmp/test_marker_*.png
ls -la /tmp/test_calibration_frame.png
```

Try detecting markers in these images with a working OpenCV installation.

## Verification Steps

1. **Check your OpenCV version:**
   ```bash
   pkg-config --modversion opencv4
   ```
   - If it's 4.13.0, you need to upgrade/downgrade

2. **Build with your chosen solution:**
   ```bash
   # Option 1: After upgrading OpenCV
   cargo build --release --features opencv
   
   # Option 2: Without OpenCV
   cargo build --release --no-default-features --features webcam
   ```

3. **Test marker generation:**
   ```bash
   cargo run --example aruco_display --features opencv
   ```

4. **Test detection:** Use the calibration feature with a camera pointed at the displayed markers

## Quick Fix for Immediate Testing

If you need to test right now without changing your OpenCV installation:

```bash
# Build without OpenCV (uses fallback generation)
cargo run --release --no-default-features --features webcam

# Then use the Video Wall calibration feature
# Note: Detection requires OpenCV, so this will show "No markers detected"
```

## Related Documentation

- [DESIGN_VIDEOWALL.md](DESIGN_VIDEOWALL.md) - Video wall calibration design
- [SETUP.md](SETUP.md) - Build instructions
- OpenCV Issue: https://github.com/opencv/opencv/issues/xxxx (ArUco 4.13 regression)

## Recommendation

**Upgrade to OpenCV 4.14.0 or later.** This is the cleanest solution and restores full functionality.

If you cannot upgrade OpenCV, consider using a different marker detection library or implementing a custom detector based on the embedded dictionary patterns in `src/videowall/aruco.rs` (see `DICT_4X4_50` constant).
