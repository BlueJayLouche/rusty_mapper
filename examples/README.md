# Rusty Mapper Examples

This directory contains example programs demonstrating various features of Rusty Mapper.

## ArUco Pattern Display (`aruco_display.rs`)

Generates and displays ArUco calibration patterns for video wall calibration.

### Running without OpenCV

Uses fallback pattern generation (hash-based patterns, not real ArUco markers):

```bash
cargo run --example aruco_display --no-default-features
```

### Running with OpenCV (Recommended)

Generates real ArUco markers using OpenCV:

```bash
cargo run --example aruco_display --features opencv
```

### Command Line Options

```bash
# 2x2 grid (default)
cargo run --example aruco_display -- --grid 2x2

# 3x3 grid
cargo run --example aruco_display -- --grid 3x3

# 4x4 grid with 4K resolution
cargo run --example aruco_display -- --grid 4x4 --resolution 3840x2160

# Show help
cargo run --example aruco_display -- --help
```

### Controls

| Key | Action |
|-----|--------|
| `SPACE` | Next pattern |
| `N` | Next pattern |
| `P` | Previous pattern |
| `A` | Enable auto-cycle (2 second intervals) |
| `S` | Stop auto-cycle |
| `F` | Toggle fullscreen |
| `ESC` | Exit |

### OpenCV Installation

#### macOS

Using Homebrew:

```bash
# Install OpenCV
brew install opencv

# Set environment variables for pkg-config
export PKG_CONFIG_PATH="/usr/local/opt/opencv/lib/pkgconfig:$PKG_CONFIG_PATH"

# For Apple Silicon (M1/M2/M3)
export PKG_CONFIG_PATH="/opt/homebrew/opt/opencv/lib/pkgconfig:$PKG_CONFIG_PATH"
```

#### Linux (Ubuntu/Debian)

```bash
# Install OpenCV development libraries
sudo apt-get update
sudo apt-get install libopencv-dev libopencv-contrib-dev pkg-config
```

#### Linux (Fedora/RHEL)

```bash
sudo dnf install opencv-devel pkgconf
```

#### Windows

On Windows, OpenCV installation is more complex. Options include:

1. **vcpkg** (recommended):
   ```cmd
   vcpkg install opencv4[contrib]:x64-windows
   set VCPKG_ROOT=C:\path\to\vcpkg
   ```

2. **Pre-built binaries** from https://opencv.org/releases/

### Verifying OpenCV Installation

```bash
# Check if OpenCV is detected
pkg-config --modversion opencv4

# Or
pkg-config --modversion opencv
```

### Saving Patterns to Files

To save the generated patterns as image files instead of displaying them, you can modify the example or use a simple script:

```rust
use rusty_mapper::videowall::{ArUcoGenerator, ArUcoDictionary};

fn main() {
    let generator = ArUcoGenerator::new(ArUcoDictionary::Dict4x4_50);
    let patterns = generator.generate_all_calibration_frames((2, 2), (1920, 1080)).unwrap();
    
    for (i, pattern) in patterns.iter().enumerate() {
        pattern.save(format!("pattern_{}.png", i)).unwrap();
    }
}
```

## Troubleshooting

### "OpenCV not found" errors

Make sure:
1. OpenCV is installed
2. `PKG_CONFIG_PATH` is set correctly
3. You can run `pkg-config --modversion opencv4` without errors

### Build errors with OpenCV

If you encounter build errors with the `opencv` crate:

1. Make sure you have the required build tools:
   - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
   - **Linux**: `build-essential`, `clang`, `libclang-dev`

2. Try updating the crate:
   ```bash
   cargo update -p opencv
   ```

3. Check the [opencv-rust documentation](https://github.com/twistedfall/opencv-rust) for platform-specific issues

### Runtime errors

If the example compiles but fails at runtime:

1. Make sure you have a working GPU and graphics drivers
2. Try running with software rendering:
   ```bash
   WGPU_BACKEND=gl cargo run --example aruco_display --no-default-features
   ```
