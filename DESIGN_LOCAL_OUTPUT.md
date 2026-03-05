# Local Video Output Design (Syphon/Spout/v4l2loopback)

## Overview

Add support for local inter-process video sharing, allowing Rusty Mapper to output to other VJ software, streaming tools, and video applications without network overhead.

## Platform Matrix

| Platform | Technology | Mechanism | Latency | Best For |
|----------|-----------|-----------|---------|----------|
| **macOS** | Syphon | IOSurface GPU texture sharing | ~0ms | Resolume, MadMapper, Millumin |
| **Windows** | Spout | DirectX/GL texture sharing | ~0ms | Resolume, TouchDesigner, OBS |
| **Linux** | v4l2loopback | Virtual video device (V4L2) | ~1-2ms | OBS, FFmpeg, VLC, JACK |

## Architecture Decision: Unified Output Interface

```rust
/// Trait for all local output mechanisms
pub trait LocalOutput: Send {
    /// Initialize the output with dimensions and format
    fn initialize(&mut self, width: u32, height: u32, format: PixelFormat) -> Result<()>;
    
    /// Submit a frame ( GPU texture or CPU buffer)
    fn submit_frame(&mut self, frame: FrameData) -> Result<()>;
    
    /// Check if output is still connected/active
    fn is_connected(&self) -> bool;
    
    /// Get output name for UI
    fn name(&self) -> &str;
    
    /// Shutdown/cleanup
    fn shutdown(&mut self);
}

/// Platform-specific implementation selected at runtime
pub enum LocalOutputAdapter {
    #[cfg(target_os = "macos")]
    Syphon(SyphonOutput),
    #[cfg(target_windows)]
    Spout(SpoutOutput),
    #[cfg(target_os = "linux")]
    V4l2(V4l2Output),
}
```

## Platform-Specific Details

### macOS: Syphon

**Mechanism:**
- Uses `IOSurface` for zero-copy GPU texture sharing between processes
- Metal/OpenGL texture published to system-wide name
- Consumers subscribe to the name and get direct GPU texture access

**Implementation Options:**

```rust
// Option A: Native Metal (wgpu already uses Metal on macOS)
pub struct SyphonOutput {
    server: syphon::Server,
    texture: metal::Texture,
    device: metal::Device,
}

impl SyphonOutput {
    pub fn new(name: &str, width: u32, height: u32) -> Result<Self> {
        // Create Syphon server with Metal device
        // wgpu gives us access to the underlying Metal device
    }
    
    pub fn submit_wgpu_texture(&mut self, texture: &wgpu::Texture) {
        // Get underlying Metal texture from wgpu
        // Publish to Syphon
    }
}

// Option B: Use syphon-rs crate (if available)
// https://github.com/Syphon/Syphon-Framework
```

**Pros:**
- Zero latency (GPU→GPU, no copy)
- Works with all major macOS VJ software
- Very stable, widely used

**Cons:**
- Requires Metal interop (wgpu supports this)
- macOS only

---

### Windows: Spout

**Mechanism:**
- DirectX shared texture handles (or OpenGL FBO sharing)
- Named shared memory for texture metadata
- Texture format negotiation between sender/receiver

**Implementation:**

```rust
pub struct SpoutOutput {
    sender: spout::Sender,
    width: u32,
    height: u32,
}

impl SpoutOutput {
    pub fn new(name: &str, width: u32, height: u32) -> Result<Self> {
        // Create Spout sender
        // Negotiate texture format (DX11 shared texture)
    }
    
    pub fn submit_frame(&mut self, device: &ID3D11Device, texture: &ID3D11Texture2D) {
        // Spout handles the DirectX shared texture publishing
    }
}
```

**Crates to investigate:**
- `spout-rs` (if exists)
- Direct bindings to Spout SDK
- Use `windows-rs` for DirectX interop

**Pros:**
- Zero latency GPU sharing
- Supported by Resolume, TouchDesigner, OBS, etc.
- Can fallback to CPU sharing if GPU fails

**Cons:**
- Windows only
- DirectX interop complexity with wgpu

---

### Linux: v4l2loopback

**Mechanism:**
- Kernel module creates virtual video devices (`/dev/videoX`)
- Write raw video frames to device file
- Any V4L2-compatible application can read it like a webcam

**Implementation:**

```rust
pub struct V4l2Output {
    device_path: PathBuf,
    file: Option<File>,
    width: u32,
    height: u32,
    format: V4l2Format,
}

impl V4l2Output {
    pub fn new(device: &str, width: u32, height: u32) -> Result<Self> {
        // Open /dev/videoX (created by v4l2loopback module)
        // Set format via V4L2 ioctl (VIDIOC_S_FMT)
    }
    
    pub fn submit_frame(&mut self, rgba_data: &[u8]) -> Result<()> {
        // Convert RGBA to required format (YUYV, MJPEG, etc.)
        // Write to device file
        // V4L2 buffers handle the rest
    }
}
```

**Required V4L2 ioctls:**
- `VIDIOC_QUERYCAP` - Check device capabilities
- `VIDIOC_S_FMT` - Set pixel format (YUYV, RGB24, etc.)
- `VIDIOC_STREAMON` - Start streaming
- Write frames via `write()` or `VIDIOC_QBUF/VIDIOC_DQBUF`

**Format Conversion:**
```rust
/// Convert RGBA to YUYV (common V4L2 format)
fn rgba_to_yuyv(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    // RGBA -> YUV -> YUYV (4:2:2 subsampling)
    // Can be GPU-accelerated with wgpu compute shader
}
```

**Pros:**
- Works with ANY V4L2 application (OBS, FFmpeg, browsers)
- No special SDK needed (just file I/O + ioctls)
- Can use CPU or GPU path

**Cons:**
- Requires kernel module installation (`modprobe v4l2loopback`)
- ~1-2ms latency (copy through kernel)
- Must handle format conversion (YUV)

---

## Integration Design

### Output Priority System

Users can enable multiple outputs simultaneously:

```rust
pub struct OutputManager {
    // Primary outputs (local GPU sharing)
    syphon: Option<SyphonOutput>,      // macOS
    spout: Option<SpoutOutput>,        // Windows
    v4l2: Option<V4l2Output>,          // Linux
    
    // Network output
    ndi: Option<NdiOutputSender>,
    
    // Screen output (always active)
    screen: ScreenOutput,
}

impl OutputManager {
    pub fn render_frame(&mut self, texture: &wgpu::Texture) {
        // 1. Copy to screen (always)
        self.screen.present(texture);
        
        // 2. GPU-sharing outputs (zero copy where possible)
        #[cfg(target_os = "macos")]
        if let Some(syphon) = &mut self.syphon {
            syphon.submit_wgpu_texture(texture);
        }
        
        #[cfg(windows)]
        if let Some(spout) = &mut self.spout {
            spout.submit_wgpu_texture(texture);
        }
        
        // 3. CPU-based outputs (need readback)
        if self.v4l2.is_some() || self.ndi.is_some() {
            let rgba_data = self.readback_texture(texture);
            
            if let Some(v4l2) = &mut self.v4l2 {
                v4l2.submit_frame(&rgba_data);
            }
            if let Some(ndi) = &mut self.ndi {
                ndi.submit_frame(&rgba_data);
            }
        }
    }
}
```

### GPU Readback Strategy

For CPU-based outputs (v4l2, NDI), we need to read GPU texture to CPU:

```rust
impl OutputManager {
    /// Efficient GPU->CPU readback using wgpu
    fn readback_texture(&self, texture: &wgpu::Texture) -> Vec<u8> {
        // Create staging buffer
        let buffer_size = (texture.width() * texture.height() * 4) as u64;
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        
        // Copy texture to buffer
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &staging_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(texture.width() * 4),
                    rows_per_image: Some(texture.height()),
                },
            },
            texture.size(),
        );
        queue.submit(std::iter::once(encoder.finish()));
        
        // Map and read
        let slice = staging_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {});
        device.poll(wgpu::Maintain::Wait);
        
        let data = slice.get_mapped_range();
        let rgba = data.to_vec();
        drop(data);
        staging_buffer.unmap();
        
        rgba
    }
}
```

**Optimization:**
- Triple-buffer readback (like we do for NDI)
- Async readback (don't block render thread)
- Dedicated output thread for CPU-based formats

---

## User Interface

### New "Outputs" Tab (or extend existing Output tab)

```rust
fn build_outputs_tab(&mut self, ui: &imgui::Ui) {
    ui.text("Local Outputs");
    ui.separator();
    
    // Platform-specific outputs
    #[cfg(target_os = "macos")]
    {
        ui.text("Syphon (macOS GPU Sharing)");
        let mut enabled = self.syphon_enabled;
        if ui.checkbox("Enable Syphon Output", &mut enabled) {
            self.toggle_syphon(enabled);
        }
        if enabled {
            ui.input_text("Server Name", &mut self.syphon_name).build();
        }
    }
    
    #[cfg(windows)]
    {
        ui.text("Spout (Windows GPU Sharing)");
        let mut enabled = self.spout_enabled;
        if ui.checkbox("Enable Spout Output", &mut enabled) {
            self.toggle_spout(enabled);
        }
        if enabled {
            ui.input_text("Sender Name", &mut self.spout_name).build();
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        ui.text("v4l2loopback (Linux Virtual Camera)");
        let mut enabled = self.v4l2_enabled;
        if ui.checkbox("Enable V4L2 Output", &mut enabled) {
            self.toggle_v4l2(enabled);
        }
        if enabled {
            ui.input_text("Device Path", &mut self.v4l2_device).build();
            ui.text_disabled("Example: /dev/video10");
            
            // Help text for setup
            if ui.button("Show Setup Instructions") {
                self.show_v4l2_help = true;
            }
        }
    }
    
    // NDI (already exists, keep it)
    ui.separator();
    ui.text("Network Output (NDI)");
    // ... existing NDI controls ...
}
```

### v4l2loopback Setup Helper

```rust
fn show_v4l2_setup_dialog(&self) {
    r#"
    v4l2loopback Setup Instructions:
    
    1. Install v4l2loopback:
       $ sudo apt install v4l2loopback-dkms  # Debian/Ubuntu
       $ sudo modprobe v4l2loopback devices=1
    
    2. Check device created:
       $ v4l2-ctl --list-devices
    
    3. In Rusty Mapper, enter the device path:
       /dev/video10 (or whatever number shows up)
    
    4. In OBS/VLC/etc, select "Dummy video device"
       as the video source.
    "#;
}
```

---

## Dependencies by Platform

### macOS
```toml
[target.'cfg(target_os = "macos")'.dependencies]
# Syphon bindings
metal = "0.29"  # For Metal interop
cocoa = "0.25" # For Objective-C interop
objc = "0.2"
```

### Windows
```toml
[target.'cfg(windows)'.dependencies]
# Spout/DirectX
windows = { version = "0.52", features = ["Win32_Graphics_Direct3D11"] }
# Or spout crate if available
```

### Linux
```toml
[target.'cfg(target_os = "linux")'.dependencies]
# V4L2
v4l2 = "0.1"  # Or nix for ioctl bindings
nix = { version = "0.27", features = ["ioctl"] }
```

---

## Questions for You

### 1. Priority
Should this be implemented **before**, **after**, or **parallel** to the video wall feature?

- **Before**: Establishes output infrastructure that video wall can reuse
- **After**: Video wall is higher priority for your immediate use
- **Parallel**: Both features can share the output manager architecture

### 2. Implementation Strategy
**Option A: Gradual** - Add one platform at a time (macOS first since you dev on Mac?)
**Option B: Parallel** - Design all three, implement together
**Option C: Focus** - Pick one platform that matters most to you

### 3. GPU vs CPU Path
**For v4l2loopback specifically:**
- **GPU Path**: Keep texture on GPU, use wgpu compute shader for RGBA→YUV conversion
- **CPU Path**: Readback RGBA, do conversion on CPU (simpler, slightly slower)

Recommendation: CPU path first (simpler), GPU compute optimization later.

### 4. Format Support
**v4l2loopback supports multiple formats:**
- YUYV (YUV 4:2:2) - most compatible
- RGB24 - simpler, but higher bandwidth
- MJPEG - compressed, lower bandwidth but quality loss

Default to YUYV for compatibility?

### 5. Multi-Output Priority
Should we allow ALL outputs simultaneously?
- Screen + Syphon + NDI (useful for recording + streaming + local VJ)
- Or limit to prevent performance issues?

### 6. Testing Strategy
- **macOS**: Test with Resolume, MadMapper, OBS
- **Windows**: Test with Resolume, TouchDesigner, OBS
- **Linux**: Test with OBS, FFmpeg, browsers

Do you have access to all three platforms for testing?

---

## Recommended Implementation Order

Based on typical VJ workflows, I suggest:

1. **macOS Syphon** (if that's your dev platform)
   - Fastest to test
   - Zero-copy GPU path
   - Huge value for macOS VJs

2. **Linux v4l2loopback**
   - Easiest implementation (file I/O + ioctls)
   - Works with EVERYTHING
   - Festival/Touring VJs often use Linux for stability

3. **Windows Spout**
   - Most complex (DirectX interop)
   - But largest user base

4. **Optimize** - GPU compute for format conversion, async readback

---

## Integration with Existing Code

Minimal changes needed:

1. **New module**: `src/output/local/` with platform subdirectories
2. **Extend OutputManager**: Add local outputs alongside NDI
3. **GUI tab**: Add "Outputs" tab or extend existing Output tab
4. **Config**: Add local output settings to AppConfig

This can coexist with video wall work - they touch different parts of the codebase.

What do you think? Which platform should we start with?
