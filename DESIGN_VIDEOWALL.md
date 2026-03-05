# Video Wall Auto-Calibration Design

## Overview

Auto-calibration system for HDMI matrix video walls using ArUco markers. Supports any grid configuration (2x2, 3x3, 4x4, etc.) with **two calibration modes**:

1. **Real-time**: Live camera capture during pattern flashing
2. **Record & Decode**: Record patterns with any camera/phone, upload video file for processing

The "record & decode" mode is ideal for complex installations where running a cable to the FOH position is impractical, or when you want to use a better camera (DSLR, phone) than a webcam.

## Architecture

### Two Calibration Workflows

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      REAL-TIME CALIBRATION                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐     ┌──────────────┐     ┌──────────────┐             │
│  │ ArUco Generator │────▶│ HDMI Matrix  │────▶│ Physical Wall│             │
│  │ (GPU)           │     │ (Virtual     │     │ (NxM grid)   │             │
│  │                 │     │  Display)    │     │              │             │
│  └─────────────────┘     └──────────────┘     └──────────────┘             │
│                                                        │                    │
│                              ┌─────────────────────────┘                    │
│                              ▼                                              │
│                       ┌──────────────┐                                     │
│                       │ Live Camera  │                                     │
│                       │ (Webcam/USB) │                                     │
│                       └──────┬───────┘                                     │
│                              ▼                                              │
│                       ┌──────────────┐     ┌──────────────┐               │
│                       │OpenCV ArUco  │────▶│   Config     │               │
│                       │Detection     │     │   (Save)     │               │
│                       └──────────────┘     └──────────────┘               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                    RECORD & DECODE CALIBRATION                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  RECORD PHASE (at venue)              DECODE PHASE (anytime/anywhere)       │
│  ───────────────────────              ─────────────────────────────────      │
│                                                                             │
│  ┌─────────────────┐                  ┌─────────────────┐                  │
│  │ ArUco Generator │                  │ Upload Video    │                  │
│  │ (Flash patterns)│                  │ (MP4/MOV/etc)   │                  │
│  └────────┬────────┘                  └────────┬────────┘                  │
│           │                                    │                           │
│           ▼                                    ▼                           │
│  ┌─────────────────┐                  ┌─────────────────┐                  │
│  │ HDMI Matrix     │                  │ Video Decoder   │                  │
│  │ (Virtual Disp)  │                  │ (ffmpeg/opencv) │                  │
│  └────────┬────────┘                  └────────┬────────┘                  │
│           │                                    │                           │
│           ▼                                    ▼                           │
│  ┌─────────────────┐                  ┌─────────────────┐                  │
│  │ Physical Wall   │                  │ Frame Extract   │                  │
│  │ (NxM grid)      │                  │ (sync to flash) │                  │
│  └────────┬────────┘                  └────────┬────────┘                  │
│           │                                    │                           │
│           ▼                                    ▼                           │
│  ┌─────────────────┐                  ┌─────────────────┐                  │
│  │ Phone/DSLR      │                  │OpenCV ArUco     │                  │
│  │ (Record video)  │                  │Detection        │                  │
│  └────────┬────────┘                  └────────┬────────┘                  │
│           │                                    │                           │
│           ▼                                    ▼                           │
│  ┌─────────────────┐                  ┌─────────────────┐                  │
│  │ Save MP4/MOV    │─────────────────▶│   Config        │                  │
│  │                 │   (transfer)     │   (Save/Apply)  │                  │
│  └─────────────────┘                  └─────────────────┘                  │
│                                                                             │
│  BENEFITS:                         BENEFITS:                                │
│  • Use any camera (phone/DSLR)     • Calibrate from FOH                   │
│  • Better image quality            • Process multiple times                 │
│  • No cable runs needed            • Can retry with different settings      │
│  • Record multiple angles          • Share video for remote help            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      RUNTIME MODE                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐     ┌─────────────────────┐               │
│  │ Main Content    │────▶│ Multi-Quad Shader   │────▶ Output   │
│  │ Texture         │     │ (Single Pass)       │      Display  │
│  │ (1920x1080)     │     │                     │               │
│  └─────────────────┘     └─────────────────────┘               │
│                                │                                │
│                                ▼                                │
│                       ┌──────────────────┐                     │
│                       │ Uniform Buffer   │                     │
│                       │ (Display Quads)  │                     │
│                       │ Loaded from Config│                    │
│                       └──────────────────┘                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. ArUco Pattern Generator

Generates unique markers for each display in the grid.

```rust
pub struct ArUcoGenerator {
    dictionary: ArucoDictionary,  // DICT_4X4_50 or DICT_6X6_250
    marker_size: u32,             // Pixels per marker
    border_bits: u32,             // Black border around marker
}

impl ArUcoGenerator {
    /// Generate marker image for display ID
    pub fn generate_marker(&self, display_id: u32) -> Vec<u8> {
        // Use OpenCV to generate ArUco marker
        // Returns grayscale image bytes
    }
    
    /// Generate full calibration frame for a specific display
    /// Shows marker centered in display region
    pub fn generate_calibration_frame(
        &self,
        display_id: u32,
        grid_size: (u32, u32),
        output_resolution: (u32, u32),
    ) -> Vec<u8> {
        // Black background with white ArUco marker centered
        // Marker sized appropriately for expected camera distance
    }
}
```

### 2. Video Decoder (Record & Decode Mode)

Decodes uploaded calibration videos and extracts frames for pattern detection.

```rust
pub struct VideoDecoder {
    video_path: PathBuf,
    capture: VideoCapture,
    fps: f64,
    total_frames: i64,
    resolution: (u32, u32),
}

pub struct DecodedFrame {
    pub frame_number: i64,
    pub timestamp_ms: f64,
    pub image: Mat,  // OpenCV Mat
    pub display_id: Option<u32>,  // If we know which display this should be
}

impl VideoDecoder {
    pub fn open(video_path: &Path) -> Result<Self> {
        // Open video with OpenCV VideoCapture
        // Support MP4, MOV, AVI, MKV (whatever ffmpeg supports)
        // Extract metadata: fps, resolution, duration
    }
    
    /// Extract frame at specific timestamp (in milliseconds)
    pub fn extract_frame_at(&mut self, timestamp_ms: f64) -> Result<DecodedFrame> {
        // CAP_PROP_POS_MSEC to seek
        // read() to get frame
        // Convert to RGB if needed
    }
    
    /// Auto-detect flash pattern timing
    /// 
    /// Analyzes video to find when each pattern appears
    /// Returns mapping of display_id -> timestamp
    pub fn detect_flash_timing(&mut self, grid_size: (u32, u32)) -> Result<Vec<FlashEvent>> {
        // Sample frames throughout video
        // Look for frames with ArUco markers
        // Correlate with expected pattern sequence
        // Handle: variable frame rate, compression artifacts, lighting changes
    }
    
    /// Extract all calibration frames
    pub fn extract_calibration_frames(
        &mut self,
        flash_events: &[FlashEvent],
    ) -> Result<Vec<DecodedFrame>> {
        // For each flash event:
        //   - Extract frame at peak brightness (middle of flash)
        //   - Extract frames before/after for validation
        //   - Return best quality frame
    }
}

pub struct FlashEvent {
    pub display_id: u32,
    pub timestamp_ms: f64,
    pub confidence: f32,
}
```

**Video Format Support:**
- Codecs: H.264, H.265, ProRes, Motion JPEG
- Containers: MP4, MOV, AVI, MKV
- Sources: Phone cameras, DSLR, action cams, screen recordings

**Robustness Features:**
- Handle variable frame rates
- Compensate for compression artifacts
- Support different recording speeds (30fps, 60fps, 120fps)
- Auto-rotation detection (phone orientation)

### 3. Calibration Controller

Manages both real-time and record/decode calibration workflows.

```rust
pub enum CalibrationMode {
    /// Real-time with live camera
    RealTime { camera: Box<dyn CameraSource> },
    /// Decode from recorded video
    VideoDecode { decoder: VideoDecoder },
}

pub enum CalibrationPhase {
    Idle,
    // Real-time phases
    Countdown { seconds_remaining: u32 },
    Flashing {
        current_display: usize,
        frame_count: u32,
        frames_per_display: u32,
    },
    // Video decode phases
    VideoUpload,
    AnalyzingVideo { progress: f32 },
    ExtractingFrames { current: usize, total: usize },
    // Common phases
    Processing,
    Detecting,
    BuildingMap,
    Complete(VideoWallConfig),
    Error(String),
}

pub struct CalibrationController {
    mode: CalibrationMode,
    phase: CalibrationPhase,
    grid_size: (u32, u32),
    output_window: Arc<Window>,
    captured_data: Vec<DisplayDetection>,
}

impl CalibrationController {
    /// Start real-time calibration
    pub fn start_realtime(&mut self, grid_size: (u32, u32), camera: Box<dyn CameraSource>) {
        self.mode = CalibrationMode::RealTime { camera };
        self.grid_size = grid_size;
        self.phase = CalibrationPhase::Countdown { seconds_remaining: 3 };
    }
    
    /// Start video decode calibration
    pub fn start_video_decode(&mut self, grid_size: (u32, u32), video_path: &Path) -> Result<()> {
        let decoder = VideoDecoder::open(video_path)?;
        self.mode = CalibrationMode::VideoDecode { decoder };
        self.grid_size = grid_size;
        self.phase = CalibrationPhase::AnalyzingVideo { progress: 0.0 };
        Ok(())
    }
    
    /// Process calibration (called every frame)
    pub fn update(&mut self) -> Option<VideoWallConfig> {
        match &mut self.mode {
            CalibrationMode::RealTime { camera } => {
                self.update_realtime(camera)
            }
            CalibrationMode::VideoDecode { decoder } => {
                self.update_video_decode(decoder)
            }
        }
    }
    
    fn update_video_decode(&mut self, decoder: &mut VideoDecoder) -> Option<VideoWallConfig> {
        match &mut self.phase {
            CalibrationPhase::AnalyzingVideo { progress } => {
                // Analyze video to detect flash timing
                match decoder.detect_flash_timing(self.grid_size) {
                    Ok(events) => {
                        self.flash_events = events;
                        self.phase = CalibrationPhase::ExtractingFrames { 
                            current: 0, 
                            total: events.len() 
                        };
                    }
                    Err(e) => {
                        self.phase = CalibrationPhase::Error(e.to_string());
                    }
                }
                None
            }
            CalibrationPhase::ExtractingFrames { current, total } => {
                // Extract frames for each flash event
                if let Some(event) = self.flash_events.get(*current) {
                    if let Ok(frame) = decoder.extract_frame_at(event.timestamp_ms) {
                        self.captured_data.push(DisplayDetection {
                            display_id: event.display_id,
                            frame: frame.image,
                        });
                    }
                    *current += 1;
                    if *current >= *total {
                        self.phase = CalibrationPhase::Processing;
                    }
                }
                None
            }
            CalibrationPhase::Processing => {
                self.process_captured_frames()
            }
            CalibrationPhase::Complete(config) => {
                return Some(config.clone());
            }
            _ => None
        }
    }
}
```

### 3. ArUco Detector (OpenCV)

Detects markers in camera frames and returns corner positions.

```rust
pub struct ArUcoDetector {
    dictionary: ArucoDictionary,
    detector_params: DetectorParameters,
}

pub struct DetectedMarker {
    pub id: u32,
    pub corners: [Point2f; 4],  // In camera image coordinates
    pub confidence: f32,
}

impl ArUcoDetector {
    pub fn detect_markers(&self, frame: &Mat) -> Vec<DetectedMarker> {
        // OpenCV ArUco detection
        // Returns all markers found in frame
    }
    
    /// Detect specific marker ID with sub-pixel refinement
    pub fn detect_specific_marker(
        &self,
        frame: &Mat,
        target_id: u32,
    ) -> Option<DetectedMarker> {
        let markers = self.detect_markers(frame);
        markers.into_iter().find(|m| m.id == target_id)
    }
}
```

### 4. Quad Mapper

Builds the UV mapping from detected markers to output quads.

```rust
pub struct DisplayQuad {
    pub display_id: u32,
    pub grid_position: (u32, u32),  // (col, row) in grid
    
    // Source: Where to sample from main texture (0-1 UV space)
    pub source_rect: Rect,
    
    // Destination: Where on output (in normalized output coordinates)
    // These form a quad that may be perspective-distorted
    pub dest_corners: [Vec2; 4],  // TL, TR, BR, BL
    
    // Transform matrix for perspective correction
    pub perspective_matrix: Mat3,
}

pub struct QuadMapper;

impl QuadMapper {
    /// Build display quads from detected markers
    /// 
    /// Each marker is assumed to represent the CENTER of a display.
    /// We extrapolate the display corners based on:
    /// - Marker corners (gives us orientation and rough size)
    /// - Expected grid layout
    /// - Adjacent marker positions (for scale reference)
    pub fn build_quads(
        detections: &[DisplayDetection],
        grid_size: (u32, u32),
        camera_resolution: (u32, u32),
    ) -> Vec<DisplayQuad> {
        // Algorithm:
        // 1. Find all markers and their centers
        // 2. Compute average marker size in pixels
        // 3. For each detected marker:
        //    - Center = marker center
        //    - Size = distance to nearest neighbor * 0.9 (bezel compensation)
        //    - Corners = center ± size in marker's orientation
        // 4. Map to output UV space (0-1)
        // 5. Compute perspective transform for each display
    }
    
    /// Compute source rectangle in main texture based on grid position
    fn compute_source_rect(
        grid_pos: (u32, u32),
        grid_size: (u32, u32),
    ) -> Rect {
        // If 2x2 grid: each display gets 1/4 of main texture
        // Display 0 (TL): source = (0, 0) to (0.5, 0.5)
        // Display 1 (TR): source = (0.5, 0) to (1.0, 0.5)
        // etc.
    }
}
```

### 5. Video Wall Configuration

Persistent configuration format.

```rust
#[derive(Serialize, Deserialize)]
pub struct VideoWallConfig {
    pub version: u32,
    pub grid_size: (u32, u32),
    pub output_resolution: (u32, u32),
    pub displays: Vec<DisplayConfig>,
    pub calibration_info: CalibrationInfo,
}

#[derive(Serialize, Deserialize)]
pub struct DisplayConfig {
    pub id: u32,
    pub grid_position: (u32, u32),
    pub name: String,  // "Display 1 (Top-Left)"
    
    // Normalized coordinates (0-1)
    pub source_uv: Rect,           // Where in main texture
    pub dest_quad: [Vec2; 4],      // Where on output (perspective quad)
    
    // Optional: per-display adjustments
    pub brightness: f32,
    pub contrast: f32,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CalibrationInfo {
    pub date: DateTime<Utc>,
    pub camera_source: String,
    pub camera_resolution: (u32, u32),
    pub marker_dictionary: String,
    pub avg_detection_confidence: f32,
}
```

### 6. Runtime Multi-Quad Shader

Single-pass GPU shader for runtime rendering.

```wgsl
// Uniforms
struct DisplayQuad {
    source_rect: vec4<f32>,      // x, y, width, height (in UV space)
    dest_corners: array<vec2<f32>, 4>,  // TL, TR, BR, BL (in output UV)
    enabled: u32,
    _padding: vec3<u32>,
}

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(1) @binding(0)
var<uniform> display_count: u32;

@group(1) @binding(1)
var<uniform> displays: array<DisplayQuad, 16>;  // Max 4x4 grid

// Vertex shader: full-screen quad
@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    // Generate full-screen triangle strip
    let x = f32(idx % 2u);  // 0, 1, 0, 1
    let y = f32(idx / 2u);  // 0, 0, 1, 1
    return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}

// Fragment shader: sample appropriate display
@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let output_uv = pos.xy / vec2<f32>(textureDimensions(source_texture));
    
    // Find which display this pixel belongs to
    for (var i: u32 = 0u; i < display_count; i = i + 1u) {
        let display = displays[i];
        
        if (display.enabled == 0u) {
            continue;
        }
        
        // Check if pixel is inside this display's quad
        if (point_in_quad(output_uv, display.dest_corners)) {
            // Map output UV to source UV with perspective correction
            let source_uv = perspective_map(
                output_uv,
                display.dest_corners,
                display.source_rect
            );
            return textureSample(source_texture, source_sampler, source_uv);
        }
    }
    
    // Background color if no display covers this pixel
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

// Barycentric coordinate check for point-in-quad
fn point_in_quad(p: vec2<f32>, quad: array<vec2<f32>, 4>) -> bool {
    // Split quad into two triangles
    // Triangle 1: 0, 1, 2
    // Triangle 2: 0, 2, 3
    return point_in_triangle(p, quad[0], quad[1], quad[2]) ||
           point_in_triangle(p, quad[0], quad[2], quad[3]);
}

// Perspective-correct UV mapping
fn perspective_map(
    output_uv: vec2<f32>,
    dest_quad: array<vec2<f32>, 4>,
    source_rect: vec4<f32>,
) -> vec2<f32> {
    // Compute barycentric coordinates in destination quad
    let bary = compute_barycentric(output_uv, dest_quad);
    
    // Map to source rectangle (0-1 space)
    // Source rect: x, y, width, height
    let source_tl = vec2<f32>(source_rect.x, source_rect.y);
    let source_br = source_tl + vec2<f32>(source_rect.z, source_rect.w);
    
    // Bilinear interpolation within source rect
    let source_uv = mix(
        mix(source_tl, vec2<f32>(source_br.x, source_tl.y), bary.x),
        mix(vec2<f32>(source_tl.x, source_br.y), source_br, bary.x),
        bary.y
    );
    
    return source_uv;
}
```

## Calibration Sequence

### User Flow

```
1. User clicks "Calibrate Video Wall" in GUI
   └─▶ Dialog: Select grid size (2x2, 3x3, 4x4)
   
2. Countdown (3 seconds)
   └─▶ Shows: "Point camera at wall. Starting in 3... 2... 1..."
   
3. Flash Sequence (N displays × 0.5s each)
   ├─▶ Display 1: Shows ArUco marker #1
   │   └─▶ Camera captures frame
   ├─▶ Display 2: Shows ArUco marker #2
   │   └─▶ Camera captures frame
   ├─▶ ...
   └─▶ Display N: Shows ArUco marker #N
       └─▶ Camera captures frame
   
4. Processing (1-2 seconds)
   └─▶ Progress bar: "Detecting markers..."
   
5. Results
   ├─▶ If success: Shows preview with detected quads overlaid
   │   └─▶ User clicks "Save" or "Retry"
   └─▶ If failure: Shows which displays weren't detected
       └─▶ User can retry or manually adjust
```

### Technical Sequence

```rust
fn calibrate_wall(
    grid_size: (u32, u32),
    camera: &mut dyn CameraSource,
    output: &mut OutputSurface,
) -> Result<VideoWallConfig, CalibrationError> {
    // 1. Generate ArUco patterns
    let generator = ArUcoGenerator::new(DICT_4X4_50);
    let patterns: Vec<_> = (0..grid_size.0 * grid_size.1)
        .map(|id| generator.generate_calibration_frame(id, grid_size, output.resolution()))
        .collect();
    
    // 2. Flash sequence
    let mut detections = Vec::new();
    for (id, pattern) in patterns.iter().enumerate() {
        // Show pattern
        output.show_pattern(pattern);
        
        // Wait for frame sync
        std::thread::sleep(Duration::from_millis(100));
        
        // Capture camera frame
        if let Some(frame) = camera.capture() {
            detections.push(DisplayDetection { display_id: id as u32, frame });
        }
        
        // Brief pause between displays
        std::thread::sleep(Duration::from_millis(200));
    }
    
    // 3. Detect markers in each frame
    let detector = ArUcoDetector::new(DICT_4X4_50);
    let mut marker_detections = Vec::new();
    
    for detection in &detections {
        if let Some(marker) = detector.detect_specific_marker(&detection.frame, detection.display_id) {
            marker_detections.push(marker);
        }
    }
    
    // 4. Build quad map
    let quads = QuadMapper::build_quads(&marker_detections, grid_size, camera.resolution());
    
    // 5. Validate
    if quads.len() != (grid_size.0 * grid_size.1) as usize {
        return Err(CalibrationError::MissingDisplays {
            expected: grid_size.0 * grid_size.1,
            found: quads.len() as u32,
        });
    }
    
    // 6. Build config
    let config = VideoWallConfig::from_quads(quads, grid_size, camera.resolution());
    
    Ok(config)
}
```

---

## Record & Decode Workflow

### Overview

This workflow allows users to record the calibration patterns with any camera/phone, then upload the video file for processing. This is ideal for:
- Complex venues where running cables to FOH is impractical
- Using better cameras (DSLR, phone) than webcams
- Calibrating from a more optimal viewing angle
- Re-processing with different settings
- Remote troubleshooting (share video with support)

### User Flow: Record Phase

```
1. Setup at Venue
   ├─▶ Install HDMI matrix and displays
   ├─▶ Connect laptop with Rusty Mapper
   └─▶ Position phone/camera viewing entire wall
   
2. Start Recording Mode
   ├─▶ User clicks "Record Calibration Video"
   ├─▶ Dialog: Select grid size (2x2, 3x3, 4x4)
   └─▶ Dialog: Select recording mode
       ├─▶ "Timed flash" (auto-advances)
       └─▶ "Manual advance" (press spacebar to next display)
   
3. Countdown
   └─▶ "Start recording NOW! Beginning in 3... 2... 1... GO!"
   
4. Flash Sequence
   ├─▶ Each display shows ArUco marker for 1-2 seconds
   ├─▶ Optional audio cue (beep) on each change
   └─▶ User ensures each pattern is clearly visible in recording
   
5. Completion
   ├─▶ "All patterns shown. Stop recording."
   ├─▶ User stops camera recording
   └─▶ Transfer video file to laptop (USB, AirDrop, etc.)
```

### User Flow: Decode Phase

```
1. Upload Video
   ├─▶ User clicks "Decode from Video"
   ├─▶ File picker: Select MP4/MOV/AVI
   └─▶ Optional: Enter expected grid size (or auto-detect)
   
2. Video Analysis (5-10 seconds)
   ├─▶ Progress: "Analyzing video... 30%"
   ├─▶ System detects flash timing automatically
   ├─▶ Identifies which frames contain which markers
   └─▶ Shows: "Found 9 flash events in 12-second video"
   
3. Frame Extraction (2-3 seconds)
   ├─▶ Extracts best quality frame for each pattern
   ├─▶ Progress: "Processing display 3 of 9..."
   └─▶ Detects ArUco markers in each frame
   
4. Review Results
   ├─▶ Shows detected displays overlaid on video frames
   ├─▶ Confidence scores for each detection
   ├─▶ Option to manually adjust if needed
   └─▶ User clicks "Apply" or "Retry with different settings"
   
5. Save Configuration
   └─▶ Saves to config file with timestamp
```

### Technical Implementation

#### Timing Synchronization

The key challenge is correlating video frames with displayed patterns. Two approaches:

**Approach A: Visual Sync (Preferred)**
```rust
/// Auto-detect flash timing by analyzing video frames
fn detect_flash_timing_by_content(video: &mut VideoDecoder) -> Vec<FlashEvent> {
    // Sample frames at regular intervals
    // Look for frames containing ArUco markers
    // Build timeline of when each marker appears
    
    // Algorithm:
    // 1. Sample every N frames (e.g., every 10 frames at 30fps = every 333ms)
    // 2. Run quick ArUco detection on each sample
    // 3. Record: (timestamp, marker_id, confidence)
    // 4. Cluster detections: group by marker_id
    // 5. For each marker_id, find the time window where it's visible
    // 6. Return middle of each window as "peak" time
    
    let sample_interval_ms = 100;  // Sample every 100ms
    let mut detections: Vec<TimestampedDetection> = Vec::new();
    
    for t in (0..video.duration_ms()).step_by(sample_interval_ms) {
        if let Ok(frame) = video.extract_frame_at(t as f64) {
            if let Some(markers) = detector.detect_markers(&frame) {
                for marker in markers {
                    detections.push(TimestampedDetection {
                        timestamp_ms: t as f64,
                        marker_id: marker.id,
                        confidence: marker.confidence,
                    });
                }
            }
        }
    }
    
    // Cluster and find peaks
    cluster_detections_to_flash_events(detections)
}
```

**Approach B: Audio Sync (Alternative)**
```rust
/// Use audio beeps to synchronize
/// Laptop plays audible beep when pattern changes
/// Video records audio + video
/// Analyze audio waveform to find beep timestamps
fn detect_flash_timing_by_audio(video: &mut VideoDecoder) -> Vec<FlashEvent> {
    // Extract audio track
    // Look for impulse sounds (beeps)
    // Map beep timestamps to pattern changes
}
```

#### Robustness Features

**Handling Different Recording Conditions:**

```rust
pub struct VideoDecodeOptions {
    /// Expected number of displays (for validation)
    pub expected_display_count: Option<usize>,
    
    /// Flash duration estimate (helps find patterns)
    pub expected_flash_duration_ms: f64,
    
    /// Minimum confidence threshold
    pub min_confidence: f32,
    
    /// Whether to auto-rotate video based on phone orientation
    pub auto_rotate: bool,
    
    /// Frame extraction quality (use nearest keyframe vs decode)
    pub extraction_quality: ExtractionQuality,
}

pub enum ExtractionQuality {
    /// Fast: Extract nearest keyframe (may be off by few frames)
    Fast,
    /// Accurate: Decode exact frame (slower but precise)
    Accurate,
    /// Best: Average multiple frames around flash peak
    MultiFrameAverage { frame_count: usize },
}
```

**Error Recovery:**

```rust
pub enum DecodeError {
    /// Video doesn't contain enough patterns
    MissingPatterns { expected: usize, found: usize },
    
    /// Some markers detected multiple times (timing ambiguous)
    AmbiguousTiming { marker_id: u32, detections: Vec<f64> },
    
    /// Video quality too poor for reliable detection
    PoorQuality { reason: String },
    
    /// Video orientation wrong (e.g., upside down)
    WrongOrientation,
}
```

### GUI Integration

```rust
fn build_videowall_calibration_dialog(&mut self, ui: &imgui::Ui) {
    ui.text("Video Wall Calibration");
    ui.separator();
    
    // Calibration mode selection
    ui.text("Calibration Mode:");
    ui.radio_button("Real-time (webcam)", &mut self.cal_mode, CalMode::RealTime);
    ui.radio_button("Record & Decode (video file)", &mut self.cal_mode, CalMode::VideoDecode);
    
    match self.cal_mode {
        CalMode::RealTime => {
            // ... existing real-time UI ...
        }
        CalMode::VideoDecode => {
            ui.separator();
            
            // Phase 1: Record instructions
            ui.text_colored([1.0, 1.0, 0.0, 1.0], "Step 1: Record at Venue");
            ui.text("1. Set up your HDMI matrix and displays");
            ui.text("2. Position camera/phone to see entire wall");
            ui.text("3. Click 'Start Recording Mode' below");
            ui.text("4. Start recording on your camera when prompted");
            ui.text("5. Wait for all patterns to flash");
            ui.text("6. Stop recording and transfer video to this computer");
            
            if ui.button("Start Recording Mode") {
                self.start_recording_mode();
            }
            
            ui.separator();
            
            // Phase 2: Decode
            ui.text_colored([1.0, 1.0, 0.0, 1.0], "Step 2: Decode Video");
            
            ui.input_text("Video file path", &mut self.video_path_input).build();
            ui.same_line();
            if ui.button("Browse...") {
                self.browse_for_video();
            }
            
            if !self.video_path_input.is_empty() {
                if ui.button("Analyze Video") {
                    self.start_video_decode();
                }
            }
            
            // Show progress if decoding
            if let Some(progress) = self.decode_progress {
                ui.progress_bar(progress).overlay_text("Analyzing...").build();
            }
            
            // Show results if available
            if let Some(preview) = &self.decode_preview {
                ui.separator();
                ui.text("Detected Displays:");
                // ... show preview image with overlays ...
                
                if ui.button("Apply Configuration") {
                    self.apply_video_wall_config();
                }
            }
        }
    }
}
```

### File Format Support

**Video Codecs:**
- H.264/AVC (most phones, good compatibility)
- H.265/HEVC (newer phones, smaller files)
- ProRes (DSLRs, professional cameras)
- Motion JPEG (screen recordings)

**Containers:**
- MP4 (universal)
- MOV (iPhone, Mac)
- AVI (older cameras)
- MKV (less common but supported)

**Dependencies:**
```toml
[dependencies]
# Video decoding
opencv = { version = "0.88", features = ["videoio"] }
# Or alternatively:
# ffmpeg-next = "6.1"  # More formats but heavier dependency
```

### Example Usage Scenarios

**Scenario 1: Festival Setup**
```
FOH Position (100m away)
  ├─▶ Laptop with Rusty Mapper
  └─▶ HDMI cable to matrix (too long!)

Solution:
  1. Run short HDMI from matrix to projector
  2. Use phone at FOH to record calibration video
  3. AirDrop video to laptop
  4. Decode and apply config
  5. Ready to go!
```

**Scenario 2: Permanent Install**
```
Venue: Ceiling-mounted displays
Problem: Can't see all displays from operator position

Solution:
  1. Use wide-angle camera at optimal viewing position
  2. Record calibration video
  3. Decode with high quality settings
  4. Fine-tune corners if needed
  5. Save config for daily use
```

**Scenario 3: Remote Support**
```
Client: Can't get calibration working
Support: "Send me your calibration video"
  1. Client records video at venue
  2. Uploads to cloud
  3. Support downloads and analyzes
  4. Sends back config file or advice
```

## Integration with Existing Code

### New Modules

```
src/
├── videowall/
│   ├── mod.rs              # Public API
│   ├── calibration.rs      # Calibration controller/state machine
│   ├── aruco.rs            # ArUco generation and detection (OpenCV)
│   ├── quad_mapper.rs      # UV mapping logic
│   ├── config.rs           # Config serialization
│   ├── renderer.rs         # Runtime multi-quad renderer
│   └── shader.wgsl         # Multi-quad shader
```

### GUI Integration

Add to existing control window:

```rust
// In Settings or new "Video Wall" tab
fn build_videowall_tab(&mut self, ui: &imgui::Ui) {
    ui.text("Video Wall Configuration");
    ui.separator();
    
    // Grid size selection
    ui.text("Grid Size:");
    let grid_sizes = [("2x2", (2, 2)), ("3x3", (3, 3)), ("4x4", (4, 4))];
    // ... radio buttons ...
    
    // Camera selection
    ui.text("Calibration Camera:");
    // ... dropdown of available cameras ...
    
    if ui.button("Start Calibration") {
        self.start_calibration();
    }
    
    if self.has_saved_config() {
        if ui.button("Load Saved Config") {
            self.load_videowall_config();
        }
        ui.same_line();
        if ui.button("Clear Config") {
            self.clear_videowall_config();
        }
    }
    
    // Preview if calibrated
    if let Some(config) = &self.videowall_config {
        ui.separator();
        ui.text("Configuration Preview");
        // ... show diagram of detected displays ...
    }
}
```

### Runtime Integration

Modify existing renderer:

```rust
// In WgpuEngine::render()
pub fn render(&mut self) {
    // ... existing setup ...
    
    // Check if video wall mode is active
    if let Some(videowall_config) = &self.videowall_config {
        // Use multi-quad shader
        self.render_videowall(videowall_config, &bind_group);
    } else {
        // Use normal shader
        self.render_normal(&bind_group, &uniform_bind_group);
    }
}
```

## Performance Considerations

### Calibration Phase
- **Pattern Generation**: ~1ms per display (GPU-accelerated if possible)
- **Flash Sequence**: 0.5s per display = 8s for 4x4 grid
- **Marker Detection**: ~50ms per frame with OpenCV
- **Total Calibration Time**: ~10-15 seconds for 4x4 grid

### Runtime Phase
- **Shader Complexity**: O(1) per pixel (simple quad lookup)
- **Memory**: One uniform buffer (~1KB for 16 displays)
- **GPU Overhead**: <0.1ms per frame (single pass)

### Optimization: Caching
- Save calibration results to disk
- Skip calibration on startup if valid config exists
- Config includes hash of display EDID (detect hardware changes)

## Error Handling

### Common Failure Modes

1. **Marker not detected**
   - Cause: Camera angle, lighting, motion blur
   - Recovery: Retry that specific display, or manual placement

2. **Wrong marker detected**
   - Cause: Reflection showing other display
   - Recovery: Flash sequence ensures temporal separation

3. **Partial detection**
   - Cause: Some displays obscured or off
   - Recovery: Allow calibration with subset, mark missing as "disabled"

4. **Perspective too extreme**
   - Cause: Camera at extreme angle
   - Recovery: Warn user, suggest better camera position

### Validation Checks
- All expected markers found
- Marker positions form expected grid pattern
- No duplicate IDs detected
- Marker sizes within reasonable bounds
- Perspective distortion not excessive

## Future Enhancements

1. **Multi-camera support** for very large walls
2. **LED wall sub-pixel mapping** (for fine pitch LED)
3. **Per-display color correction** using colorimeter
4. **Edge blending** for overlapping projectors
5. **3D mapping** for curved surfaces (spherical, cylindrical)
6. **Automatic re-calibration** on detecting hardware changes


## Implementation Plan

### Phase 1: Foundation (Week 1)
**Goal**: Basic ArUco generation and detection working

**Tasks:**
1. Add OpenCV dependency to Cargo.toml
2. Create `videowall/aruco.rs` module:
   - ArUco marker generation using OpenCV
   - Marker detection wrapper
3. Create `videowall/config.rs`:
   - VideoWallConfig struct
   - Serialize/deserialize (JSON or TOML)
4. Unit tests for marker generation/detection

**Deliverable**: Can generate ArUco patterns and detect them in test images

---

### Phase 2: Calibration Controller (Week 2)
**Goal**: Working calibration state machine

**Tasks:**
1. Create `videowall/calibration.rs`:
   - CalibrationPhase state machine
   - Flash sequence timing
   - Camera capture integration
2. Integrate with existing camera input system
3. Create calibration pattern display (renders to output window)
4. Add progress UI (countdown, flashing progress, processing status)

**Deliverable**: Can run through full calibration sequence and capture frames

---

### Phase 3: Quad Mapping (Week 3)
**Goal**: Convert detected markers to UV mapping

**Tasks:**
1. Create `videowall/quad_mapper.rs`:
   - Build quads from detected markers
   - Perspective matrix calculation
   - Grid position inference
2. Validation logic (check for missing displays, wrong IDs)
3. Preview visualization (overlay detected quads on camera image)
4. Manual adjustment UI (fine-tune corner positions)

**Deliverable**: Calibration produces valid VideoWallConfig with correct UV mapping

---

### Phase 4: Runtime Renderer (Week 4)
**Goal**: Multi-quad shader for video wall output

**Tasks:**
1. Create `videowall/shader.wgsl`:
   - Multi-quad fragment shader
   - Perspective-correct UV mapping
2. Create `videowall/renderer.rs`:
   - VideoWallRenderer struct
   - Uniform buffer management
   - Pipeline setup
3. Integrate with WgpuEngine (switch between normal/wall mode)
4. Performance optimization (ensure <0.1ms overhead)

**Deliverable**: Can render content across video wall with correct mapping

---

### Phase 5: GUI Integration (Week 5)
**Goal**: User-friendly calibration workflow

**Tasks:**
1. Add "Video Wall" tab to control GUI
2. Grid size selector (2x2, 3x3, 4x4, custom)
3. Camera source selector
4. Calibration start/stop controls
5. Results preview and validation
6. Config save/load UI
7. Error handling and user feedback

**Deliverable**: Complete user-facing video wall feature

---

### Phase 6: Polish & Edge Cases (Week 6)
**Goal**: Production-ready feature

**Tasks:**
1. Handle edge cases:
   - Partial detection (some displays off/missing)
   - Extreme camera angles
   - Poor lighting conditions
2. Add calibration validation warnings
3. Implement retry/resume logic
4. Performance profiling and optimization
5. Documentation and examples
6. Testing with real HDMI matrix hardware

**Deliverable**: Video wall feature ready for VJ use

---

## Dependencies to Add

```toml
[dependencies]
# OpenCV for ArUco marker detection
opencv = { version = "0.88", default-features = false, features = ["aruco", "imgproc"] }

# Serialization for config
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# For matrix math (perspective transforms)
# (glam already in project, use that)
```

## Testing Strategy

### Unit Tests
- ArUco generation produces valid markers
- Marker detection finds markers in test images
- Quad mapping produces correct UV coordinates
- Config serialization roundtrip

### Integration Tests
- Full calibration sequence with test pattern
- Runtime renderer produces correct output
- Config save/load works correctly

### Manual Tests
- 2x2 grid with physical displays
- 3x3 grid at different camera angles
- Various lighting conditions
- Different HDMI matrix configurations

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| OpenCV build complexity | High | Use pre-built wheels, document build process |
| ArUco detection fails in poor lighting | Medium | Implement fallback patterns (color-coded), add manual adjustment |
| Performance issues with large grids | Low | Single-pass shader design, early performance testing |
| HDMI matrix compatibility | Medium | Test with common matrices (Blackbird, J-Tech), document known-good |
| Camera latency causes timing issues | Low | Configurable flash duration, frame sync logic |

## Success Criteria

- [ ] Can calibrate 2x2 grid in <10 seconds
- [ ] Can calibrate 4x4 grid in <20 seconds
- [ ] Runtime performance overhead <0.1ms per frame
- [ ] Works with at least 3 popular HDMI matrix brands
- [ ] No manual adjustment needed for typical setups (<5° camera angle)
- [ ] Config persists across restarts
- [ ] GUI is intuitive for non-technical users

## Resources

- [ArUco Marker Detection Tutorial](https://docs.opencv.org/4.x/d5/dae/tutorial_aruco_detection.html)
- [OpenCV Rust Bindings](https://github.com/twistedfall/opencv-rust)
- [Perspective Transformation Math](https://docs.opencv.org/4.x/da/d54/group__imgproc__transform.html)
- [VJ Community Video Wall Thread](https://www.resolume.com/forum/viewtopic.php?t=12345) (example)
