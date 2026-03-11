# Video Wall System - Ready for Testing

## ✅ Completed Features

### 1. AprilTag Marker System
- **Detection**: Pure Rust AprilTag detection (no OpenCV)
- **Generation**: Calibration patterns with all markers displayed
- **Marker images**: 20 pre-generated AprilTags in `assets/apriltags/`

### 2. Calibration System
- **Photo calibration**: Load a screenshot of your displays
- **Real-time calibration**: Live camera feed
- **Auto-mapping**: Detects markers and builds display quads automatically
- **Missing display handling**: Works with partial setups (e.g., 2 displays in 3x3 grid)

### 3. Test Pattern Generator (NEW)
```rust
use rusty_mapper::videowall::TestPattern;

// Generate test pattern for verification
let pattern = TestPattern::Numbered.generate_full_frame(
    (3, 3),           // Grid size
    (1920, 1080)      // Output resolution
);
pattern.save("test_pattern.png").unwrap();
```

**Available patterns:**
- `ColorBars` - SMPTE color bars
- `Grid` - Grid with crosshair and display ID
- `Numbered` - Large display numbers (recommended for alignment)
- `Checkerboard` - Black/white checkerboard
- `Gradient` - Gradient patterns

### 4. Manual Corner Adjustment (Already Implemented)
- **GUI Controls**: Sliders for each corner (TL, TR, BR, BL)
- **Per-display adjustments**: Brightness, contrast, gamma
- **Performance mode**: Can disable per-display adjustments

### 5. Output Texture Resizing (Already Implemented)
The `InputTextureManager` automatically resizes textures when input resolution changes.

---

## 🎯 How to Test Your Setup

### Step 1: Generate Test Pattern
```bash
# Use the example to generate a test pattern
cargo run --example aruco_display --no-default-features
```
Or manually:
```python
import cv2
# Generate 9 numbered displays for 3x3 grid
os.makedirs("test_output", exist_ok=True)
for i in range(9):
    # Create 640x360 image with large number
    img = np.zeros((360, 640, 3), dtype=np.uint8)
    cv2.putText(img, str(i), (200, 250), cv2.FONT_HERSHEY_SIMPLEX, 8, (255,255,255), 10)
    cv2.imwrite(f"test_output/display_{i}.png", img)
```

### Step 2: Display Test Pattern
- Use your HDMI matrix to show the test pattern on all displays
- Or use the app's calibration mode which shows numbered AprilTags

### Step 3: Take Photo
- Take a photo of all displays showing the test pattern
- Ensure all displays are visible in the photo

### Step 4: Calibrate
1. Open the app
2. Go to Video Wall → Photo Calibration
3. Select your photo
4. The app will detect AprilTags and build the mapping

### Step 5: Verify
- Use `TestPattern::Numbered` to verify mapping is correct
- Each display should show its correct ID

### Step 6: Fine-tune (Optional)
- Use the corner adjustment sliders to fine-tune each display
- Adjust brightness/contrast per display if needed

---

## 📋 Answers to Your Requirements

| Requirement | Status | Notes |
|------------|--------|-------|
| Up to 4x4 grid | ✅ | Supports 4x4 (16 displays) |
| Partial setup (2 displays) | ✅ | Missing displays auto-detected |
| Front-facing camera | ✅ | Works with any camera angle |
| Display markers from app | ✅ | AprilTag 36h11 family |
| 75% marker size | ✅ | Configurable in `MarkerDisplayConfig` |
| Input 1 or 2 selectable | ✅ | Per-display source selection |
| Per-display adjustment | ✅ | Brightness, contrast, gamma |
| Performance bypass | ✅ | Disable adjustments in shader |
| Photo calibration | ✅ | Implemented |
| Test pattern generator | ✅ | 5 patterns available |
| Manual corner adjustment | ✅ | Already implemented |

---

## 🚀 Quick Start Commands

```bash
# Build without OpenCV
cargo build --no-default-features --features webcam

# Run the app
cargo run --no-default-features --features webcam

# Run tests
cargo test --no-default-features --features webcam
```

---

## 🎨 Test Pattern Usage

```rust
use rusty_mapper::videowall::TestPattern;
use image::save_buffer;

// Generate numbered pattern for 3x3 grid
let pattern = TestPattern::Numbered.generate_full_frame((3, 3), (1920, 1080));
pattern.save("numbered_test.png").unwrap();

// Generate grid pattern for alignment
let grid = TestPattern::Grid.generate_full_frame((2, 2), (1920, 1080));
grid.save("grid_test.png").unwrap();
```

---

## 🔧 Configuration

### Marker Size
Edit in GUI or code:
```rust
let config = MarkerDisplayConfig {
    marker_size_percent: 0.75,  // 75% of display size
    margin_percent: 0.125,       // 12.5% margin
};
```

### Quad Mapping
```rust
let config = QuadMapConfig {
    display_scale_factor: 1.5,   // Display is 1.5x marker size
    min_confidence: 0.5,         // Minimum detection confidence
    use_neighbor_scaling: true,  // Use neighbor markers for scale
    bezel_compensation: 0.1,     // 10% bezel gap
};
```

---

## 📸 Calibration Tips

1. **Lighting**: Ensure good lighting on markers
2. **Camera position**: Front-facing is easiest
3. **Marker size**: Larger markers = better detection
4. **Photo quality**: Sharp, no motion blur
5. **All displays**: Try to capture all displays in one photo

---

## 🐛 Troubleshooting

### No markers detected
- Check you're using AprilTag markers (not ArUco)
- Increase marker size in settings
- Improve lighting

### Mapping is wrong
- Use manual corner adjustment
- Check grid size matches your setup
- Verify display IDs are correct

### Performance issues
- Disable per-display adjustments
- Reduce output resolution
- Use simpler test patterns

---

## 📁 Files Added/Modified

| File | Description |
|------|-------------|
| `src/videowall/test_pattern.rs` | NEW - Test pattern generator |
| `src/videowall/mod.rs` | Added test_pattern module |
| `src/videowall/apriltag.rs` | AprilTag implementation |
| `src/videowall/calibration.rs` | Photo/real-time calibration |
| `src/lib.rs` | Added TestPattern export |
| `assets/apriltags/` | Pre-generated marker images |

---

**Ready to test!** The system supports your HDMI matrix setup with up to 4x4 grids, handles partial setups, and includes all the calibration tools you need.
