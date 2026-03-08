# Rusty Mapper Setup Guide

High-performance projection mapping application with NDI and Syphon support.

## Prerequisites

### 1. Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Syphon Framework (macOS)

The framework is shared across the workspace:

```
../crates/syphon/syphon-lib/Syphon.framework
```

### 3. NDI Runtime (Optional)

For NDI input/output support, download from [NDI.tv](https://ndi.tv)

### 4. OpenCV (Optional)

For video wall calibration features:

```bash
cargo build --features opencv
```

## Building

```bash
# Standard build
cargo build --release

# With all features
cargo build --release --features webcam,opencv

# Run
cargo run --release
```

## Syphon Support

### Input

Receive video from Syphon servers (Resolume, TouchDesigner, etc.):

```rust
use rusty_mapper::input::{
    InputManager, 
    InputType,
    syphon_input::SyphonDiscovery
};

// Discover servers
let discovery = SyphonDiscovery::new();
let servers = discovery.discover_servers();

// Start Syphon input
let mut input_manager = InputManager::new();
input_manager.start_input1_syphon("Resolume Arena")?;

// Get frames
input_manager.update();
if let Some(data) = input_manager.input1.take_frame() {
    // Use frame data (BGRA)
}
```

### Output

Send mapped output to other apps:

```rust
use rusty_mapper::output::SyphonOutput;

// Create output
let mut output = SyphonOutput::new(
    "Rusty Mapper Output",
    device.clone(),
    queue.clone()
)?;

// Each frame
output.submit_frame(&texture, &device, &queue)?;
```

## Configuration

### Input Sources

The app supports multiple input types:

- **Webcam** - Local camera capture
- **NDI** - Network Device Interface
- **Syphon** - macOS GPU sharing
- **OBS** - Via NDI plugin

### Video Wall

For projection mapping:

1. Calibrate using ArUco markers
2. Configure display positions
3. Map input to outputs
4. Export/Import calibration

## Troubleshooting

### Syphon not available

Check the framework is linked:

```bash
otool -L target/debug/rusty_mapper | grep Syphon
```

Should show:
```
@rpath/Syphon.framework/Versions/A/Syphon
```

### Crash on background thread

Ensure you're using the latest syphon-core with autoreleasepool fixes.

## Features

### Input
- ✅ Webcam (nokhwa)
- ✅ NDI (grafton-ndi)
- ✅ Syphon (syphon-core)

### Output
- ✅ NDI
- ✅ Syphon
- 🚧 Spout (Windows - planned)

### Processing
- ✅ Quad warping
- ✅ Edge blending
- ✅ Calibration
- 🚧 Shader effects (planned)

## Links

- [Syphon Documentation](../crates/syphon/README.md)
- [Syphon Troubleshooting](../crates/syphon/TROUBLESHOOTING.md)
