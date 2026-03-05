# Rusty Mapper - Implementation Progress

## Completed Features

### 1. Project Architecture ✅
- [x] Cargo project setup with dependencies
- [x] Modular code structure (core, engine, gui, input, ndi, audio)
- [x] Configuration system with TOML
- [x] Comprehensive documentation (README, DESIGN.md, AGENTS.md)

### 2. Dual-Window System ✅
- [x] Output window (fullscreen-capable, cursor hidden)
- [x] Control window (ImGui-based interface)
- [x] Shared wgpu context between windows
- [x] Window event handling (keyboard shortcuts, resize)

### 3. Input Management ✅
- [x] **InputManager** with support for multiple input types:
  - Webcam (via nokhwa, optional feature)
  - NDI (Network Device Interface)
  - OBS (via NDI output)
- [x] Independent Input 1 and Input 2
- [x] Hot-swappable inputs without restart
- [x] Refreshable device lists

### 4. GUI Implementation ✅
- [x] **Main Tabs**: Inputs, Mapping, Output, Settings
- [x] **Inputs Tab**:
  - Device selection for Input 1 & 2
  - Source type tabs (Webcam, NDI, OBS)
  - Mix amount slider (0-100%)
  - Refresh devices button
- [x] **Mapping Tab**:
  - Corner pinning controls (4 corners, UV coordinates)
  - Global transform (scale, offset, rotation)
  - Blend settings (opacity, blend mode)
  - Reset buttons
- [x] **Output Tab**:
  - Fullscreen toggle
  - NDI output controls
  - Status display
- [x] **Settings Tab**:
  - UI scale control
  - Keyboard shortcuts info

### 5. Projection Mapping / Texture Mapping ✅
- [x] **InputMapping** struct with:
  - Corner pinning (4 UV coordinates)
  - Scale and offset
  - Rotation
  - Opacity and blend mode
- [x] **Shader implementation** (`main.wgsl`):
  - Corner pin warping (bilinear interpolation)
  - UV transformation
  - Multiple blend modes (Normal, Add, Multiply, Screen)
  - Per-input opacity
- [x] **GPU uniforms** for real-time parameter updates
- [x] Live editing via GUI with sync to GPU

### 6. NDI Integration ✅
- [x] **NDI Input** (dedicated thread):
  - BGRA to RGBA conversion
  - Bounded frame queue (latest-frame-only)
  - Async frame reception
- [x] **NDI Output** (dedicated thread):
  - RGBA to BGRA/BGRX conversion
  - Bounded frame queue (low latency)
  - Cloneable sender handle
  - Thread persists via Box::leak pattern

### 7. Rendering Engine ✅
- [x] wgpu-based GPU rendering
- [x] Dual bind group setup:
  - Bind group 0: Textures and samplers
  - Bind group 1: Mapping uniforms
- [x] Render-to-texture then blit to surface
- [x] Uniform buffer management for mapping

## Completed Fixes

### Buffer Alignment ✅
- **Issue**: `Buffer offset 64 does not respect device's requested min_uniform_buffer_offset_alignment limit 256`
- **Solution**: Switched from single buffer with offsets to separate buffers for each uniform
  - `uniform_buffer_input1` for Input 1 mapping (64 bytes)
  - `uniform_buffer_input2` for Input 2 mapping (64 bytes)  
  - `uniform_buffer_mix` for mix settings (16 bytes)
- **Result**: Build successful, no runtime errors

### GUI Rendering ✅
- **Issue**: Grey screen - ImGui draw data was created but never rendered to the control window surface
- **Solution**: Rewrote `ImGuiRenderer` with full `imgui-wgpu` integration:
  - Created dedicated wgpu surface for the control window
  - Integrated `imgui-winit-support` for proper event handling
  - Implemented full render pipeline with ImGui draw data submission
  - Added proper surface configuration and presentation
- **Result**: GUI now renders properly with all tabs (Inputs, Mapping, Output, Settings)

### HiDPI/Retina Display Fix ✅
- **Issue**: `Scissor Rect { x: 0, y: 0, w: 1600, h: 2400 } is not contained in the render target (800, 1200, 1)`
- **Root Cause**: ImGui uses physical pixels while surface was configured with logical size (2x scale on Retina)
- **Solution**: 
  - Calculate physical size using `window.scale_factor()` during initialization
  - Set `display_framebuffer_scale` in ImGui IO to match
  - Convert logical to physical size in `resize()` method
- **Result**: GUI renders correctly on HiDPI displays

### GUI Layout Cleanup ✅
- **Issue**: Debug/imgui-default window taking priority over control window, messy layout
- **Root Cause**: Using `ui.window()` wrapper around main content creates nested windows
- **Solution** (following rustjay_waaaves pattern):
  - Build UI directly to root (no wrapper window)
  - Use `ui.menu_bar()` for top menu
  - Use `ui.tab_bar()` / `ui.tab_item()` for proper tab behavior
  - Added dark theme with colored accents matching VJ aesthetic
  - Dark background clear color instead of default blue-grey
- **Result**: Clean, professional VJ-style interface

### Mouse Cursor Offset Fix ✅
- **Issue**: Mouse cursor offset down and to the right on HiDPI displays
- **Root Cause**: Double-handling scale factor with imgui-winit-support
- **Solution**: Simplified to rustjay_waaaves approach - manual event handling, logical sizes
- **Result**: Mouse works correctly on all displays

### Mouse/Keyboard Input Fix ✅
- **Issue**: Couldn't click on GUI elements after removing imgui-winit-support
- **Solution**: Added manual event handling for cursor, buttons, wheel, keyboard
- **Result**: Full ImGui interaction working

### Blend Mix Debug 🔄
- **Issue**: Mix slider behavior may be inverted (needs verification)
- **Status**: Debug logging added, needs testing with actual NDI sources

## In Progress / Known Issues

### Pending Features
- [ ] Visual preview of mapping in control window
- [ ] Preset save/load for mapping configurations
- [ ] Audio input and FFT analysis
- [ ] MIDI/OSC controller support
- [ ] Video recording output
- [ ] Syphon/Spout support (macOS/Windows GPU texture sharing)

## Architecture Highlights

### Threading Model
```
Main Thread:     Event loop → Render → Present
NDI Input 1:     Receiver → Frame Queue → GPU Upload
NDI Input 2:     Receiver → Frame Queue → GPU Upload
NDI Output:      Frame Queue → Sender → Network
Audio (future):  Capture → FFT → Shared State
```

### Data Flow
```
Input Sources (Webcam/NDI/OBS)
         ↓
  InputManager (CPU)
         ↓
  GPU Texture Upload
         ↓
  Shader (with Mapping Uniforms)
   - Corner Pin Warp
   - UV Transform
   - Blend
         ↓
  Render Target
         ↓
  + Blit to Output Window
  + NDI Output Thread
```

### Shader Pipeline
```
Vertex Shader:   Pass through position + UV

Fragment Shader:
  1. Apply corner pin (bilinear interpolation)
  2. Apply global transform (scale, rotate, offset)
  3. Sample input textures
  4. Apply blend mode and opacity
  5. Output final color
```

## Technical Decisions

1. **Separate Uniform Buffers**: Chosen over single buffer to avoid alignment issues
2. **bytemuck for GPU Data**: Zero-copy conversion between Rust structs and GPU buffers
3. **imgui-wgpu Integration**: Full wgpu-based ImGui renderer with dedicated surface for control window
4. **Optional Webcam**: Can disable with `--no-default-features` if libclang unavailable
5. **Corner Pinning**: Bilinear interpolation for quad warping (good for projection mapping)

## File Structure
```
src/
├── app.rs              # Dual-window application handler
├── config.rs           # TOML configuration
├── core/
│   ├── state.rs        # SharedState, InputMapping, commands
│   ├── vertex.rs       # GPU vertex definitions
│   └── mod.rs
├── engine/
│   ├── renderer.rs     # wgpu renderer with uniforms
│   ├── texture.rs      # Texture management
│   ├── shaders/
│   │   └── main.wgsl   # Projection mapping shader
│   └── mod.rs
├── gui/
│   ├── gui.rs          # ImGui interface (4 tabs)
│   ├── renderer.rs     # ImGui context management
│   └── mod.rs
├── input/
│   ├── mod.rs          # InputManager, InputSource
│   ├── ndi.rs          # NDI receiver
│   └── webcam.rs       # Webcam capture (optional)
├── ndi/
│   ├── output.rs       # NDI sender thread
│   └── mod.rs
├── audio/              # (placeholder)
└── main.rs
```

## Next Steps

1. **Fix current buffer alignment issue** (separate buffers approach)
2. **Test end-to-end**: Verify mapping GUI updates shader correctly
3. **Add mapping preview**: Show a small quad preview in control window
4. **Preset system**: Save/load mapping configurations
5. **Performance optimization**: Profile GPU usage

## Current Status

### Working Features ✅
- **Dual-window system** - Output window (fullscreen capable) + Control window (ImGui)
- **NDI Input** - Receive video from NDI sources (dedicated thread, BGRA→RGBA conversion)
- **NDI Output** - Send video to NDI receivers (dedicated thread, RGBA→BGRA conversion)
- **Projection Mapping** - Corner pinning with bilinear interpolation (4 corners per input)
- **Transform Controls** - Scale, offset, rotation per input
- **Blend Modes** - Normal (mix), Add, Multiply, Screen
- **Per-input Opacity** - Independent opacity control
- **GUI** - Clean dark-themed ImGui interface with 4 tabs (Inputs, Mapping, Output, Settings)
- **Device Selection** - Popup window for choosing Webcam/NDI/OBS sources
- **Hot-swappable Inputs** - Change sources without restart
- **Mouse/Keyboard Input** - Full ImGui interaction support

### Priority Feature 1: Visual GUI Layout 🆕
**Status**: Design complete, ready for implementation
**Target**: Preview-centric interface with source/output previews and right-side controls

See [DESIGN_GUI_LAYOUT.md](DESIGN_GUI_LAYOUT.md) for full specification.

**Layout:**
```
┌──────────────────────────────┬──────────────────┐
│  Source Preview (50%)        │   Controls       │
│  ┌────────────────────────┐  │   ┌───────────┐  │
│  │ Input texture with     │  │   │ Inputs    │  │
│  │ display boxes overlay  │  │   │ Mapping   │  │
│  └────────────────────────┘  │   │ Video Wall│  │
│                              │   │ Output    │  │
│  Output Preview (50%)        │   └───────────┘  │
│  ┌────────────────────────┐  │   Quick Actions │
│  │ Final output as sent   │  │                  │
│  │ to virtual device      │  │   [Start NDI]   │
│  └────────────────────────┘  │   [Calibrate]   │
└──────────────────────────────┴──────────────────┘
```

**Key Features:**
- Left panel (70%): Source preview (top) + Output preview (bottom)
- Source preview: Input texture with colored display boxes overlay
- Output preview: Final rendered output
- Right panel (30%): Tabbed controls + quick actions
- Interactive: Drag display boxes to adjust mapping
- Responsive: Maintains aspect ratios, minimum sizes

### Priority Feature 2: Video Wall Auto-Calibration 🆕
**Status**: Design complete, ready for implementation
**Target**: Auto-calibrate HDMI matrix video walls using ArUco markers

See [DESIGN_VIDEOWALL.md](DESIGN_VIDEOWALL.md) for full technical specification.

**Key Features:**
- Support any grid size (2x2, 3x3, 4x4, etc.)
- One-click calibration using webcam/camera
- ArUco marker detection via OpenCV
- Automatic UV mapping from camera view to output quads
- Single-pass GPU shader for runtime (zero overhead)
- Persistent config save/load

**Implementation Phases:**
1. ArUco pattern generator + calibration controller
2. OpenCV marker detection integration
3. Quad mapper (detected markers → UV coordinates)
4. Multi-quad runtime shader
5. GUI integration and config persistence

### Priority Feature 2: Local Video Sharing (Syphon/Spout/v4l2loopback) 🆕
**Status**: Design complete, pending prioritization
**Target**: Bidirectional zero-latency local video sharing with other VJ software

See:
- [DESIGN_LOCAL_OUTPUT.md](DESIGN_LOCAL_OUTPUT.md) - Output to other apps
- [DESIGN_LOCAL_INPUT.md](DESIGN_LOCAL_INPUT.md) - Input from other apps

**Platform Support:**
| Platform | Tech | Input | Output | Latency |
|----------|------|-------|--------|---------|
| **macOS** | Syphon | Receive | Send | ~0ms GPU |
| **Windows** | Spout | Receive | Send | ~0ms GPU |
| **Linux** | v4l2loopback | Read | Write | ~1-2ms |

**Bidirectional Use Cases:**
1. **Downstream**: Resolume → Rusty Mapper (corner pin) → Projector
2. **Mixer**: TouchDesigner + Resolume → Rusty Mapper → Output
3. **Browser Input**: Chrome (WebGL) → v4l2loopback → Rusty Mapper → NDI
4. **Feedback Loop**: Rusty Mapper → Syphon → Rusty Mapper (effects)
5. **Multi-software**: Complex chains with multiple VJ apps

**Key Features:**
- Zero-copy GPU sharing (Syphon/Spout) - same memory
- Unified `VideoInput`/`LocalOutput` traits across platforms
- Discovery: List available servers/senders/devices
- Hot-swap: Change local sources without restart
- Format negotiation (automatic where possible)

### Known Issues 🐛
- **Mix Slider Behavior** - May be inverted (needs verification with debug logging)

### Build & Run

```bash
# Build with all features (includes webcam)
cargo build --release

# Build without webcam support (macOS default)
cargo build --release --no-default-features

# Run with debug logging
RUST_LOG=debug cargo run --no-default-features

# Run normally
cargo run --no-default-features
```

## Key Shortcuts
- `Shift+F` - Toggle fullscreen
- `Escape` - Exit application

## Session Log - March 5, 2026

### Today's Work
Focused on getting the GUI fully functional:

1. **Fixed GPU buffer alignment** - Separate uniform buffers for mapping params (avoiding 256-byte alignment issues)
2. **Implemented full wgpu-based ImGui renderer** - Proper surface management with imgui-wgpu
3. **Fixed HiDPI display issues** - Correct scale factor handling following rustjay_waaaves pattern
4. **Cleaned up GUI layout** - Removed nested windows, proper tab bar (Inputs, Mapping, Output, Settings)
5. **Added dark VJ theme** - Professional look matching rustjay_waaaves style
6. **Fixed mouse interaction** - Manual event handling for cursor, buttons, wheel, keyboard

### Current State
The application now has a clean, working GUI with:
- Menu bar (File, Devices)
- Tab bar with 4 tabs
- Device selector popup for Webcam/NDI/OBS
- Full mouse/keyboard support
- Dark professional theme
- Working projection mapping with corner pinning

### Next Session
- Test with actual NDI sources
- Verify mix slider behavior
- Add mapping preview visualization
- Test blend modes

---

## Session Log - March 6, 2026

### Video Wall Auto-Calibration Design

Created comprehensive technical design for video wall support. This feature addresses a real need in the VJ community for easy video wall calibration.

**Design Highlights:**
- **ArUco Markers**: Industry-standard, robust detection, built into OpenCV
- **Flash Sequence**: Temporal separation prevents cross-talk between displays
- **One Big Virtual Display**: Matches HDMI matrix output (simplifies setup)
- **Single-Pass Runtime Shader**: Zero performance overhead during show
- **Two Calibration Modes**:
  - **Real-time**: Live webcam capture (10-15 seconds)
  - **Record & Decode**: Record with phone/DSLR, upload video file (flexible, remote-friendly)

**Architecture Decisions:**
1. OpenCV for detection (mature, well-tested ArUco implementation)
2. **Record & Decode workflow** for complex installations (record at venue, decode anywhere)
3. Automatic flash timing detection from video content (no manual sync needed)
4. Persistent config with auto-load (start-and-go for fixed installs)
5. Perspective-correct UV mapping (handles any camera angle)

**Key Innovation - Record & Decode:**
- Users can record calibration patterns with any camera (phone, DSLR)
- Transfer video file to laptop (USB, AirDrop, cloud)
- Software auto-detects flash timing and extracts frames
- Enables calibration from optimal viewing angles
- Allows remote troubleshooting (send video to support)

See [DESIGN_VIDEOWALL.md](DESIGN_VIDEOWALL.md) for full implementation spec.

### Visual GUI Layout Design

Designed a completely new preview-centric GUI layout.

**Layout Structure:**
```
┌────────────────────────────┬─────────────────┐
│ Source Preview (50%)       │ Controls (30%)  │
│ [Texture + Display Boxes]  │ ┌─────────────┐ │
├────────────────────────────┤ │ Tabs        │ │
│ Output Preview (50%)       │ │ Quick Actions│ │
│ [Final Output]             │ └─────────────┘ │
└────────────────────────────┴─────────────────┘
```

**Key Features:**
- Source preview shows input texture with colored display box overlays
- Output preview shows final rendered result
- Controls on right side in tabbed interface
- Interactive drag-to-adjust mapping
- Visual feedback instead of text-heavy controls

See [DESIGN_GUI_LAYOUT.md](DESIGN_GUI_LAYOUT.md) for full specification.

### Local Video Sharing Design (Syphon/Spout/v4l2loopback)

Designed comprehensive **bidirectional** local video sharing system for inter-process communication with other VJ software.

**Key Design Decisions:**
1. **Unified Traits**: `VideoInput` for input, `LocalOutput` for output
2. **GPU-First**: Zero-copy GPU sharing (Syphon/Spout) via wgpu interop
3. **CPU Fallback**: v4l2loopback with efficient upload/download
4. **Symmetric API**: Input and output use similar patterns per platform

**Input Implementation:**
- **Syphon Input**: Subscribe to servers, receive IOSurface → wgpu texture (zero copy)
- **Spout Input**: Connect to senders, receive D3D11 texture → wgpu (zero copy)
- **v4l2loopback Input**: V4L2 capture, mmap buffers, upload to GPU

**Use Case Examples:**
```
Resolume ──► Syphon ──► Rusty Mapper ──► Projector (downstream)
TouchDesigner ──┐
                ├──► Rusty Mapper (mixer)
Resolume ───────┘

Chrome ──► v4l2loopback ──► Rusty Mapper ──► NDI (browser input)
```

**Implementation Notes:**
- Can be developed parallel to video wall (separate subsystems)
- Linux input/output easiest (V4L2 symmetrical read/write)
- macOS input slightly harder than output (client vs server)
- Windows input/output similar complexity (DirectX interop)

See [DESIGN_LOCAL_OUTPUT.md](DESIGN_LOCAL_OUTPUT.md) and [DESIGN_LOCAL_INPUT.md](DESIGN_LOCAL_INPUT.md) for full specs.

## Summary of Technical Achievements

1. **Robust Dual-Window Architecture**: Successfully implemented output window (fullscreen) + control window (ImGui) sharing a single wgpu device/queue

2. **GPU Uniform Buffer Strategy**: Solved alignment issues by using separate buffers per uniform rather than offset-based indexing

3. **Corner Pinning Shader**: Implemented bilinear interpolation in WGSL for real-time quad warping with rotation, scale, and offset

4. **Thread-Safe NDI**: Separate threads for input (with BGRA→RGBA conversion) and output (RGBA→BGRA conversion) with bounded queues

5. **Hot-Swappable Inputs**: Runtime input source switching without application restart

6. **Modular Blend System**: Multiple blend modes (Normal, Add, Multiply, Screen) with per-input opacity control

7. **Clean Error Handling**: Extensive use of anyhow for error propagation, graceful fallbacks for missing inputs
