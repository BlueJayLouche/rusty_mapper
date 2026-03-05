# Rusty Mapper - Agent Guide

## Project Overview

This is a **projection mapping video application** built in Rust with:
- NDI input/output for video streaming
- GPU-accelerated rendering via wgpu
- Dual-window architecture (control + fullscreen output)
- Hidden cursor on output window for clean projection

## Key Architecture Patterns

### Multi-Input Support (`InputManager`)

**Unified Input Interface** (`InputSource`):
- Supports Webcam, NDI, and OBS (via NDI)
- Each input has independent type and configuration
- Hot-swappable without application restart
- Latest-frame-only semantics for all types

**InputManager**:
- Manages Input 1 and Input 2
- Refreshes device lists on demand
- Handles Webcam via nokhwa, NDI via grafton-ndi
- Updates all inputs in main loop

### NDI Integration (from rustjay_waaaves)

**Input (`NdiReceiver` in `input/ndi.rs`)**:
- Dedicated thread per source
- Receives BGRA, converts to RGBA
- Bounded channel (capacity=5), drops old frames
- Latest-frame-only semantics

**Output (`NdiOutputSender` in `ndi/output.rs`)**:
- Dedicated sender thread (prevents render loop blocking)
- Receives RGBA, converts to BGRA/BGRX
- Bounded channel (capacity=2) for low latency
- Cloneable handle (non-owning clones can submit)
- Thread persists via `Box::leak` pattern

### Dual Window Setup

```rust
// Shared wgpu resources
let instance = wgpu::Instance::new(...);
let (device, queue) = adapter.request_device(...).await?;

// Output window - owns surface
let output_surface = instance.create_surface(output_window)?;

// Control window - shares device/queue
let imgui_renderer = ImGuiRenderer::new(device, queue, control_window)?;
```

### Cursor Hiding

```rust
// Default hidden
output_window.set_cursor_visible(false);

// Show on exit, hide on enter
WindowEvent::CursorEntered => window.set_cursor_visible(false),
WindowEvent::CursorLeft => window.set_cursor_visible(true),
```

### Fullscreen Toggle

```rust
let fullscreen_mode = if fullscreen {
    Some(winit::window::Fullscreen::Borderless(None))
} else {
    None
};
output_window.set_fullscreen(fullscreen_mode);
```

## Important Dependencies

| Crate | Purpose |
|-------|---------|
| `winit` | Window management |
| `wgpu` | GPU rendering |
| `grafton-ndi` | NDI video I/O |
| `crossbeam` | Thread channels |
| `imgui` | UI framework |

## Build Notes

- Uses Rust 2021 edition
- Release profile optimized for performance (`lto = "fat"`)
- Requires NDI SDK runtime for NDI functionality

## Code Style

- Module documentation at top of each file
- Thread safety: use `Arc<Mutex<T>>` for shared state
- Error handling: use `anyhow::Result` for fallible operations
- Logging: use `log` crate macros (`info!`, `warn!`, `error!`)

## Input Commands

Input changes are handled via commands in `SharedState`:

```rust
pub enum InputChangeRequest {
    None,
    StartWebcam { device_index, width, height, fps },
    StartNdi { source_name },
    StartObs { source_name },
    StopInput,
    RefreshDevices,
}
```

GUI sets the request, App processes it in `about_to_wait()`.

## Testing NDI

Install NDI Tools for testing:
- **macOS**: NDI Video Monitor, NDI Virtual Input
- **Windows**: NDI Studio Monitor
- **Test pattern**: Use NDI Test Patterns to generate test sources

## Testing Webcam

Webcam support is optional via `webcam` feature:
```bash
# With webcam (default)
cargo run

# Without webcam
cargo run --no-default-features
```

Camera indices 0-3 are auto-detected. If a camera fails to open, it's skipped in the list.
