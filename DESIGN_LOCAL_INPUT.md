# Local Video Input Design (Syphon/Spout/v4l2loopback)

## Overview

Bidirectional local video sharing - receive video from other VJ software via Syphon (macOS), Spout (Windows), and v4l2loopback (Linux). This enables Rusty Mapper to act as a downstream processor in complex VJ chains.

## Architecture: Symmetric Input/Output

```
┌─────────────────────────────────────────────────────────────────┐
│                   RUSTY MAPPER INPUT SOURCES                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │    NDI       │  │   Webcam     │  │   Local GPU  │          │
│  │   Network    │  │   (nokhwa)   │  │   Sharing    │          │
│  │              │  │              │  │              │          │
│  │ • Remote     │  │ • USB Cam    │  │ • Syphon     │◄── macOS │
│  │ • OBS NDI    │  │ • Capture    │  │ • Spout      │◄── Win   │
│  │ • Other PC   │  │   Cards      │  │ • v4l2loop   │◄── Linux │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                  │                  │                  │
│         └──────────────────┴──────────────────┘                  │
│                            │                                     │
│                   ┌────────▼────────┐                           │
│                   │  Input Manager  │                           │
│                   │  (Unified API)  │                           │
│                   └────────┬────────┘                           │
│                            │                                     │
│                   ┌────────▼────────┐                           │
│                   │  wgpu Texture   │                           │
│                   │  Upload/Share   │                           │
│                   └─────────────────┘                           │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Use Cases

### 1. VJ Chain: Resolume → Rusty Mapper → Output
```
Resolume (generative visuals)
    ↓
Syphon/Spout (GPU texture)
    ↓
Rusty Mapper (projection mapping, corner pinning)
    ↓
Projector/LED Wall
```

### 2. Complex Routing: Multiple Sources
```
TouchDesigner (3D render) ──┐
                            ├──► Rusty Mapper (mixer) ──► Output
Resolume (video clips) ─────┘
        ↑
   Syphon/Spout
```

### 3. Browser/WebGL Input (v4l2loopback)
```
Chrome (WebGL visuals)
    ↓
v4l2loopback (virtual camera)
    ↓
Rusty Mapper (process)
    ↓
NDI Output to rest of venue
```

### 4. Final Stage Processing
```
Main VJ Software (Resolume/MadMapper)
    ↓
Syphon/Spout
    ↓
Rusty Mapper (final color correction, LUTs, output)
    ↓
Projector
```

## Platform-Specific Implementation

### macOS: Syphon Input

**Mechanism:**
- Subscribe to named Syphon servers
- Receive `IOSurface` handles
- Import as Metal texture
- Use with wgpu via Metal interop

**Implementation:**

```rust
pub struct SyphonInput {
    client: syphon::Client,
    server_name: String,
    current_frame: Option<metal::Texture>,
}

impl SyphonInput {
    pub fn new(server_name: &str) -> Result<Self> {
        // Create Syphon client bound to specific server
        // Or list available servers and let user choose
    }
    
    pub fn list_servers() -> Vec<SyphonServerInfo> {
        // Query system for available Syphon servers
        // Returns name, dimensions, format
    }
    
    pub fn try_receive_frame(&mut self) -> Option<&metal::Texture> {
        // Non-blocking check for new frame
        // Syphon delivers frames asynchronously
    }
    
    /// Convert Metal texture to wgpu texture
    pub fn get_wgpu_texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        // Create wgpu texture from existing Metal texture
        // Zero copy - same GPU memory
    }
}
```

**Frame Lifecycle:**
```
1. Syphon Server (Resolume/etc) publishes IOSurface
2. Syphon Client (us) receives surface handle
3. Import as Metal texture
4. Create wgpu texture view (zero copy)
5. Use in shader as input
6. Syphon handles reference counting
```

---

### Windows: Spout Input

**Mechanism:**
- Connect to named Spout sender
- Receive DirectX shared texture handle
- Map to wgpu via DirectX interop

**Implementation:**

```rust
pub struct SpoutInput {
    receiver: spout::Receiver,
    sender_name: String,
    shared_texture: Option<ID3D11Texture2D>,
}

impl SpoutInput {
    pub fn new(sender_name: &str) -> Result<Self> {
        // Create Spout receiver
    }
    
    pub fn list_senders() -> Vec<SpoutSenderInfo> {
        // Query available Spout senders
    }
    
    pub fn try_receive_frame(&mut self) -> Option<&ID3D11Texture2D> {
        // Check for new frame
        // Spout updates shared texture reference
    }
    
    /// Convert D3D11 texture to wgpu texture
    pub fn get_wgpu_texture(&self, device: &wgpu::Device) -> wgpu::Texture {
        // Use wgpu's DirectX interop
        // May require copy if formats don't match
    }
}
```

---

### Linux: v4l2loopback Input

**Mechanism:**
- Open v4l2loopback device as V4L2 capture device
- Read frames via mmap or read()
- Upload to GPU as wgpu texture

**Implementation:**

```rust
pub struct V4l2Input {
    device_path: PathBuf,
    fd: i32,
    buffers: Vec<V4l2Buffer>,
    current_frame: Option<Vec<u8>>,
    width: u32,
    height: u32,
    format: V4l2PixelFormat,
}

impl V4l2Input {
    pub fn new(device: &str) -> Result<Self> {
        // Open V4L2 device
        // Query capabilities
        // Set format (negotiate with source)
        // Allocate buffers (mmap or userptr)
        // Start streaming (VIDIOC_STREAMON)
    }
    
    pub fn list_devices() -> Vec<V4l2DeviceInfo> {
        // Scan /dev/video*
        // Filter for v4l2loopback devices
        // Return name, caps, current format
    }
    
    /// Non-blocking frame capture
    pub fn try_receive_frame(&mut self) -> Option<&[u8]> {
        // VIDIOC_DQBUF (dequeue buffer)
        // If buffer available, return slice
        // If no buffer, return None (non-blocking)
    }
    
    /// Release frame back to driver
    pub fn release_frame(&mut self, buffer_idx: usize) {
        // VIDIOC_QBUF (queue buffer back)
    }
    
    /// Upload to wgpu texture (CPU→GPU)
    pub fn upload_to_wgpu(
        &self,
        frame_data: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> wgpu::Texture {
        // Create texture
        // Write texture data (handles format conversion)
    }
}
```

**V4L2 Buffer Management:**
```rust
pub enum IoMethod {
    Mmap,      // Driver allocates, we map to user space
    UserPtr,   // We allocate, driver fills
    ReadWrite, // Simple read() / write()
}

impl V4l2Input {
    fn init_mmap(&mut self) -> Result<()> {
        // VIDIOC_REQBUFS - request N buffers
        // For each buffer:
        //   VIDIOC_QUERYBUF - get buffer info
        //   mmap() - map to user space
        //   VIDIOC_QBUF - queue buffer for capture
    }
}
```

---

## Unified Input Trait

```rust
/// Trait for all video input sources
pub trait VideoInput: Send {
    /// Start/initialize the input
    fn start(&mut self) -> Result<()>;
    
    /// Stop/cleanup
    fn stop(&mut self);
    
    /// Non-blocking frame check
    fn try_receive_frame(&mut self) -> Option<FrameRef>;
    
    /// Get current resolution
    fn resolution(&self) -> (u32, u32);
    
    /// Get pixel format
    fn pixel_format(&self) -> PixelFormat;
    
    /// Is input connected/active?
    fn is_connected(&self) -> bool;
    
    /// Get display name for UI
    fn name(&self) -> &str;
}

/// Frame reference (avoids copying)
pub enum FrameRef<'a> {
    /// GPU texture (zero copy from Syphon/Spout)
    GpuTexture(&'a wgpu::Texture),
    
    /// CPU buffer (needs upload for v4l2loopback)
    CpuBuffer(&'a [u8]),
}

/// Platform-specific implementations
pub enum LocalInput {
    #[cfg(target_os = "macos")]
    Syphon(SyphonInput),
    #[cfg(windows)]
    Spout(SpoutInput),
    #[cfg(target_os = "linux")]
    V4l2(V4l2Input),
}

impl VideoInput for LocalInput {
    fn try_receive_frame(&mut self) -> Option<FrameRef> {
        match self {
            #[cfg(target_os = "macos")]
            LocalInput::Syphon(s) => s.try_receive_frame().map(FrameRef::GpuTexture),
            #[cfg(windows)]
            LocalInput::Spout(s) => s.try_receive_frame().map(FrameRef::GpuTexture),
            #[cfg(target_os = "linux")]
            LocalInput::V4l2(v) => v.try_receive_frame().map(FrameRef::CpuBuffer),
        }
    }
    // ... other methods delegated similarly
}
```

---

## Integration with Input Manager

Extend existing `InputSource` enum:

```rust
pub enum InputSource {
    None,
    Webcam(WebcamCapture),
    Ndi(NdiReceiver),
    // NEW: Local GPU sharing inputs
    #[cfg(target_os = "macos")]
    Syphon(SyphonInput),
    #[cfg(windows)]
    Spout(SpoutInput),
    #[cfg(target_os = "linux")]
    V4l2(V4l2Input),
}

impl InputSource {
    pub fn start_local(&mut self, source_type: LocalInputType, name: &str) -> Result<()> {
        match source_type {
            #[cfg(target_os = "macos")]
            LocalInputType::Syphon => {
                *self = InputSource::Syphon(SyphonInput::new(name)?);
            }
            #[cfg(windows)]
            LocalInputType::Spout => {
                *self = InputSource::Spout(SpoutInput::new(name)?);
            }
            #[cfg(target_os = "linux")]
            LocalInputType::V4l2 => {
                *self = InputSource::V4l2(V4l2Input::new(name)?);
            }
        }
        self.start()
    }
}
```

---

## Frame Pipeline

### GPU Path (Syphon/Spout)
```
External App
    ↓
IOSurface/DX11 Texture (GPU memory)
    ↓
Rusty Mapper imports as wgpu::Texture (zero copy)
    ↓
Directly usable in shader
```

### CPU Path (v4l2loopback)
```
External App
    ↓
v4l2loopback device
    ↓
Rusty Mapper reads via V4L2
    ↓
CPU buffer (RGBA or YUV)
    ↓
Upload to wgpu::Texture
    ↓
Usable in shader
```

### Frame Upload Optimization

```rust
/// Efficient CPU→GPU upload for v4l2loopback
pub fn upload_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frame_data: &[u8],
    width: u32,
    height: u32,
    format: PixelFormat,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Input Frame"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    
    // Convert format if needed (YUYV → RGBA)
    let rgba_data = match format {
        PixelFormat::Rgba => frame_data.to_vec(),
        PixelFormat::Yuyv => yuyv_to_rgba(frame_data, width, height),
        PixelFormat::Rgb => rgb_to_rgba(frame_data),
    };
    
    // Upload
    queue.write_texture(
        texture.as_image_copy(),
        &rgba_data,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        texture.size(),
    );
    
    texture
}
```

---

## GUI Integration

### Input Source Selection Dialog

Add "Local" tab alongside Webcam/NDI/OBS:

```rust
fn build_local_input_selector(&mut self, ui: &imgui::Ui, input_num: i32) {
    ui.text("Local GPU/Video Sharing");
    ui.separator();
    
    // List available sources
    let sources = self.list_local_sources();
    
    if sources.is_empty() {
        ui.text_colored([1.0, 0.5, 0.0, 1.0], "No local sources found");
        ui.text("Make sure another app is sharing video.");
    } else {
        for source in sources {
            let label = format!("{} ({}x{})", source.name, source.width, source.height);
            if ui.button(&label) {
                self.select_local_input(input_num, source);
                self.show_device_selector = false;
            }
            ui.same_line();
            ui.text(format!("- {}", source.source_type));
        }
    }
    
    ui.separator();
    ui.text_disabled("Tip: Start sharing in Resolume, OBS, or TouchDesigner first.");
}

/// Platform-specific source listing
fn list_local_sources(&self) -> Vec<LocalSourceInfo> {
    let mut sources = Vec::new();
    
    #[cfg(target_os = "macos")]
    {
        sources.extend(SyphonInput::list_servers());
    }
    
    #[cfg(windows)]
    {
        sources.extend(SpoutInput::list_senders());
    }
    
    #[cfg(target_os = "linux")]
    {
        sources.extend(V4l2Input::list_devices());
    }
    
    sources
}
```

---

## Bidirectional Use Case: Loopback

User can route output back to input for feedback effects:

```
Rusty Mapper Input 1 ───┐
   (Syphon/Spout)      │
                        ├──► Processing ───► Output
Rusty Mapper Output ────┘   (Corner pin,
   (feedback loop)          Effects, etc.)
```

**Configuration:**
```rust
// Enable local output
output.enable_syphon("RustyMapper-Feedback");

// ... later, as input
input.start_local(LocalInputType::Syphon, "RustyMapper-Feedback");
```

---

## Dependencies

### All Platforms
```toml
[dependencies]
# Already have wgpu for GPU interop
```

### macOS
```toml
[target.'cfg(target_os = "macos")'.dependencies]
metal = "0.29"
cocoa = "0.25"
objc = "0.2"
# Syphon framework bindings (may need custom)
```

### Windows
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = ["Win32_Graphics_Direct3D11"] }
d3d11 = "0.3"
# Spout SDK bindings (may need custom)
```

### Linux
```toml
[target.'cfg(target_os = "linux")'.dependencies]
nix = { version = "0.27", features = ["ioctl"] }
v4l2 = "0.1"  # Or implement manually with ioctl
```

---

## Testing Strategy

### macOS
- Resolume Arena → Syphon → Rusty Mapper
- MadMapper → Syphon → Rusty Mapper
- TouchDesigner → Syphon → Rusty Mapper
- OBS (macOS) → Syphon → Rusty Mapper

### Windows
- Resolume Arena → Spout → Rusty Mapper
- TouchDesigner → Spout → Rusty Mapper
- OBS → Spout → Rusty Mapper
- Unreal Engine → Spout → Rusty Mapper

### Linux
- FFmpeg → v4l2loopback → Rusty Mapper
- OBS → v4l2loopback → Rusty Mapper
- GStreamer → v4l2loopback → Rusty Mapper
- Browser (via webcam access) → v4l2loopback → Rusty Mapper

---

## Implementation Order

### Phase 1: Discovery & Listing
- List available Syphon servers (macOS)
- List available Spout senders (Windows)
- List available V4L2 devices (Linux)
- GUI: Show in device selector

### Phase 2: Basic Input
- Syphon: Receive IOSurface, display
- Spout: Receive D3D11 texture, display
- V4l2: Read frames, upload to GPU, display

### Phase 3: Integration
- Integrate with InputManager
- Support as Input 1 and Input 2
- Hot-swapping between local and other sources

### Phase 4: Optimization
- Zero-copy GPU path (Syphon/Spout)
- Efficient CPU upload (v4l2)
- Format conversion shaders (YUV→RGBA)

### Phase 5: Advanced
- Bidirectional (output → input feedback)
- Format negotiation
- Performance monitoring

---

## Relationship to Output Design

| Feature | Output | Input |
|---------|--------|-------|
| **Syphon** | Create server, publish texture | Create client, subscribe to server |
| **Spout** | Create sender, share handle | Create receiver, get handle |
| **v4l2loopback** | Write to device | Read from device |
| **Latency** | Zero (GPU) | Zero (GPU) or ~1ms (CPU) |
| **Complexity** | Simple (producer) | Medium (consumer) |

**Code Sharing:**
- Platform detection logic
- Format conversion utilities
- wgpu interop code
- GUI listing components

---

## Questions for You

1. **Priority**: Input before output, or output before input?
   - Input = Rusty Mapper receives from other apps
   - Output = Rusty Mapper sends to other apps

2. **Testing environment**: What apps will you test with?
   - macOS: Resolume, TouchDesigner, MadMapper, OBS?
   - Windows: Same, or others?
   - Linux: OBS, FFmpeg, browsers?

3. **Format support**: 
   - Should v4l2loopback support YUV formats (more efficient) or just RGBA (simpler)?

4. **Bidirectional loops**: 
   - Should we support feedback (output back to input) for effects?

5. **Implementation approach**:
   - Implement input/output together (symmetric)?
   - Or separate phases (one then the other)?
