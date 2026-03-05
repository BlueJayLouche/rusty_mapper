# Rusty Mapper

A high-performance projection mapping application in Rust with NDI input/output and GPU-accelerated rendering.

## Features

- **Multiple Input Types**:
  - **Webcam**: Direct camera capture via nokhwa
  - **NDI**: Network Device Interface sources
  - **OBS**: OBS Studio output via NDI plugin
- **Independent Input Mapping**: Each input slot can have different source type
- **Refreshable Device Lists**: Hot-swap inputs without restart
- **NDI Input/Output**: Full NDI support with dedicated threads for low-latency video streaming
- **Dual Window Architecture**: 
  - Fullscreen output window with hidden cursor for clean projection
  - Control window with ImGui-based interface
- **GPU Acceleration**: wgpu-based rendering for cross-platform support
- **Projection Mapping**: Corner pinning with bilinear interpolation for quad warping
- **Transform Controls**: Per-input scale, offset, and rotation
- **Blend Modes**: Normal (mix), Add, Multiply, Screen blending
- **Visual GUI** (WIP): Preview-centric interface with source/output previews and overlayed display boxes
- **Video Wall Support** (WIP): Auto-calibration for HDMI matrix walls via ArUco markers, with record/decode workflow
- **Local Sharing** (WIP): Syphon/Spout/v4l2loopback for inter-app video sharing
- **Configurable Resolution**: Internal rendering resolution independent of window size

## Architecture

See [DESIGN.md](DESIGN.md) for detailed architecture documentation.

### Design Documents

- [DESIGN_GUI_LAYOUT.md](DESIGN_GUI_LAYOUT.md) - Visual GUI layout with source/output previews
- [DESIGN_VIDEOWALL.md](DESIGN_VIDEOWALL.md) - Video wall auto-calibration using ArUco markers
- [DESIGN_LOCAL_OUTPUT.md](DESIGN_LOCAL_OUTPUT.md) - Local video output (Syphon/Spout/v4l2loopback)
- [DESIGN_LOCAL_INPUT.md](DESIGN_LOCAL_INPUT.md) - Local video input (Syphon/Spout/v4l2loopback)

```
┌─────────────────┐      ┌─────────────────────┐
│  Control Window │      │   Output Window     │
│   (ImGui UI)    │      │  (Fullscreen, No    │
│                 │      │   Cursor)           │
└────────┬────────┘      └──────────┬──────────┘
         │                          │
         └──────────┬───────────────┘
                    │
           ┌────────▼────────┐
           │   wgpu Engine   │
           │  (GPU Render)   │
           └────────┬────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
┌───────▼────────┐      ┌───────▼────────┐
│  NDI Input     │      │  NDI Output    │
│   Thread       │      │   Thread       │
└────────────────┘      └────────────────┘
```

## Building

### Requirements

- Rust 1.70+
- NDI SDK (for NDI support)
- macOS, Linux, or Windows

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

## Usage

1. **Start the application** - Two windows will appear:
   - Output window (main display)
   - Control window (settings)

2. **Select Input Sources** (Inputs tab):
   - Click "Select Source" for Input 1 or Input 2
   - Choose from Webcam, NDI, or OBS tabs
   - Click "Refresh" to detect new sources

3. **Configure Mapping** (Mapping tab):
   - Select Input 1 or Input 2 to map
   - Adjust corner positions for projection mapping
   - Set scale, offset, and rotation
   - Choose blend mode and opacity

4. **Start NDI Output** (Output tab):
   - Enter a stream name
   - Click "Start NDI Output"

5. **Toggle Fullscreen**:
   - Press `Shift+F` in the output window
   - Or use the checkbox in the Output tab

6. **Exit**:
   - Press `Escape` in the output window
   - Or close either window

### OBS Integration

To use OBS as an input source:
1. Install OBS NDI plugin: https://github.com/obs-ndi/obs-ndi
2. In OBS: Tools → NDI Output Settings → Enable
3. In Rusty Mapper: Select "OBS" tab in input selector
4. Choose your OBS NDI source

## Configuration

Edit `config.toml` to customize:

```toml
[output_window]
width = 1280
height = 720
fullscreen = false
vsync = true
fps = 60

[resolution]
internal_width = 1920
internal_height = 1080
```

## Project Structure

```
src/
├── main.rs          # Entry point
├── app.rs           # Dual-window application handler
├── config.rs        # Configuration loading
├── core/            # Core types and shared state
├── input/           # Input management (webcam, NDI, OBS)
│   ├── mod.rs       # InputManager, InputSource
│   ├── ndi.rs       # NDI receiver
│   └── webcam.rs    # Webcam capture
├── ndi/             # NDI output
├── engine/          # wgpu rendering engine
├── gui/             # ImGui control interface
└── audio/           # Audio input (optional)
```

## License

MIT
