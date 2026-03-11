# Rust OpenCV/ArUco Options

## Problem Summary

The `opencv` crate (v0.98) fails to build against OpenCV 4.13 (current Homebrew version).

**Error**: Binding generator panics with "internal error: entered unreachable code: Any other kind of class shouldn't be generated"

This happens during header parsing - OpenCV 4.13 introduced API changes that the binding generator doesn't handle.

---

## Option 1: Build OpenCV 4.8 from Source (Recommended)

The `opencv` crate is tested against OpenCV 4.8. Building this version from source guarantees compatibility.

### Step 1: Download OpenCV 4.8.1

```bash
# Create build directory
mkdir -p ~/opencv-build && cd ~/opencv-build

# Download OpenCV 4.8.1 and contrib modules
curl -L https://github.com/opencv/opencv/archive/4.8.1.tar.gz -o opencv-4.8.1.tar.gz
curl -L https://github.com/opencv/opencv_contrib/archive/4.8.1.tar.gz -o opencv_contrib-4.8.1.tar.gz

tar xzf opencv-4.8.1.tar.gz
tar xzf opencv_contrib-4.8.1.tar.gz
```

### Step 2: Build OpenCV

```bash
cd opencv-4.8.1
mkdir build && cd build

cmake .. \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=$HOME/opencv-4.8.1-install \
  -DOPENCV_EXTRA_MODULES_PATH=../../opencv_contrib-4.8.1/modules \
  -DBUILD_SHARED_LIBS=ON \
  -DBUILD_TESTS=OFF \
  -DBUILD_PERF_TESTS=OFF \
  -DBUILD_opencv_python2=OFF \
  -DBUILD_opencv_python3=OFF \
  -DWITH_OPENCL=ON \
  -DWITH_OPENGL=OFF \
  -DWITH_QT=OFF \
  -DWITH_GTK=OFF

make -j$(sysctl -n hw.ncpu)
make install
```

### Step 3: Configure Environment

Add to your `~/.zshrc` or `~/.bash_profile`:

```bash
# OpenCV 4.8.1
export OPENCV_LINK_PATHS="$HOME/opencv-4.8.1-install/lib"
export OPENCV_INCLUDE_PATHS="$HOME/opencv-4.8.1-install/include/opencv4"
export DYLD_LIBRARY_PATH="$HOME/opencv-4.8.1-install/lib:$DYLD_LIBRARY_PATH"
```

### Step 4: Build Your Rust Project

```bash
cd /your/project
cargo clean
cargo build
```

**Pros**: Full ArUco support, stable API  
**Cons**: Long build time (~30 min), requires ~5GB disk space

---

## Option 2: Use AprilTags (Pure Rust Alternative)

AprilTags are similar fiducial markers with native Rust support.

### Cargo.toml

```toml
[dependencies]
apriltag = "0.4"
apriltag-image = "0.1"
image = "0.25"
```

### Example Code

```rust
use apriltag::{Detector, Family, Image};
use image::GrayImage;

fn detect_markers(image_path: &str) {
    // Load image
    let img = image::open(image_path).unwrap().to_luma8();
    let (width, height) = img.dimensions();
    
    // Convert to AprilTag format
    let image = Image::from_buffer(width as usize, height as usize, &img);
    
    // Create detector
    let mut detector = Detector::new();
    detector.add_family(Family::tag_36h11());
    
    // Detect
    let detections = detector.detect(&image);
    
    for det in detections {
        println!("Detected ID: {}", det.id());
        println!("Center: {:?}", det.center());
        println!("Corners: {:?}", det.corners());
    }
}
```

### Generating AprilTag Markers

Download markers from: https://github.com/AprilRobotics/apriltag-imgs

Or generate with Python:

```python
import cv2
from cv2 import aruco

# AprilTag 36h11 family
dictionary = aruco.getPredefinedDictionary(aruco.DICT_APRILTAG_36h11)
marker_img = aruco.generateImageMarker(dictionary, 0, 200)
cv2.imwrite("apriltag_0.png", marker_img)
```

**Pros**: Pure Rust, no OpenCV dependency, faster detection  
**Cons**: Different marker format (not compatible with ArUco), fewer online resources

---

## Option 3: Wait for opencv Crate Update

Track the issue: https://github.com/twistedfall/opencv-rust/issues

OpenCV 4.13 support will likely be added in a future crate update.

**Workaround**: Pin OpenCV version temporarily:

```bash
# Don't upgrade OpenCV via Homebrew
brew pin opencv
```

---

## Option 4: Use Python Bridge (Quick & Dirty)

Call Python from Rust for marker detection:

```rust
use std::process::Command;
use std::fs;

fn detect_with_python(image_path: &str) -> Vec<Marker> {
    // Write image path to temp file, call python, read results
    let output = Command::new("python3")
        .arg("detect.py")
        .arg(image_path)
        .arg("--json")  # Modify detect.py to output JSON
        .output()
        .expect("Python failed");
    
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).unwrap()
}
```

**Pros**: Works immediately, no build issues  
**Cons**: Requires Python at runtime, slower, deployment complexity

---

## Option 5: Pre-built OpenCV via Conan/vcpkg

Use a C++ package manager to get a compatible OpenCV version:

### Using vcpkg

```bash
# Install vcpkg
git clone https://github.com/Microsoft/vcpkg.git ~/vcpkg
cd ~/vcpkg
./bootstrap-vcpkg.sh

# Install OpenCV 4.8
./vcpkg install opencv4[contrib]:x64-osx

# Set env vars
export VCPKG_ROOT="$HOME/vcpkg"
export VCPKGRS_DYNAMIC=1
```

Then build your Rust project - the `opencv` crate will auto-detect vcpkg.

---

## Comparison Table

| Option | Effort | Performance | Compatibility | Best For |
|--------|--------|-------------|---------------|----------|
| Build OpenCV 4.8 | High | Native | Full ArUco | Production apps |
| AprilTags | Low | Native | AprilTags only | New projects |
| Wait for update | None | Native | Future | Non-urgent |
| Python bridge | Low | Slow | Full | Prototyping |
| vcpkg | Medium | Native | Full | CI/CD environments |

---

## Quick Decision Guide

- **Need ArUco specifically?** → Option 1 (Build OpenCV 4.8)
- **Just need fiducial markers?** → Option 2 (AprilTags)
- **Prototyping only?** → Option 4 (Python bridge)
- **Production, can wait?** → Option 3 (Wait for update)
