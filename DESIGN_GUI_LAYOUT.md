# GUI Layout Design - Visual Mapping Interface

## Overview

A preview-centric GUI layout optimized for projection mapping and video wall configuration. The interface emphasizes visual feedback with source texture preview (showing mapped regions) and output preview (showing final result).

## Layout Structure

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         RUSTY MAPPER CONTROL WINDOW                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────────────────────────────┐  ┌────────────────────────────────┐ │
│  │         LEFT PANEL (70%)           │  │      RIGHT PANEL (30%)         │ │
│  │                                    │  │                                │ │
│  │  ┌──────────────────────────────┐  │  │  ┌──────────────────────────┐  │ │
│  │  │    SOURCE PREVIEW (50%)      │  │  │  │     CONTROLS             │  │ │
│  │  │                              │  │  │  │                          │  │ │
│  │  │  ┌────────────────────────┐  │  │  │  │  ┌────────────────────┐  │  │ │
│  │  │  │                        │  │  │  │  │  │ Tabs               │  │  │ │
│  │  │  │   Input Texture        │  │  │  │  │  │ ┌────┬────┬────┐  │  │  │ │
│  │  │  │   (1920x1080)          │  │  │  │  │  │ │Inp │Map │Out │  │  │  │ │
│  │  │  │                        │  │  │  │  │  │ └────┴────┴────┘  │  │  │ │
│  │  │  │   ┌───┐ ┌───┐         │  │  │  │  │  │                   │  │  │ │
│  │  │  │   │ 0 │ │ 1 │         │  │  │  │  │  │ [Input controls   │  │  │ │
│  │  │  │   └───┘ └───┘         │  │  │  │  │  │  based on tab]    │  │  │ │
│  │  │  │   ┌───┐ ┌───┐         │  │  │  │  │  │                   │  │  │ │
│  │  │  │   │ 2 │ │ 3 │         │  │  │  │  │  │ Sliders, buttons, │  │  │ │
│  │  │  │   └───┘ └───┘         │  │  │  │  │  │ dropdowns, etc    │  │  │ │
│  │  │  │                        │  │  │  │  │  │                   │  │  │ │
│  │  │  │   [Display boxes       │  │  │  │  │  │                   │  │  │ │
│  │  │  │    overlayed on        │  │  │  │  │  │                   │  │  │ │
│  │  │  │    source texture]     │  │  │  │  │  │                   │  │  │ │
│  │  │  │                        │  │  │  │  │  │                   │  │  │ │
│  │  │  └────────────────────────┘  │  │  │  └────────────────────┘  │  │ │
│  │  │                              │  │  │                          │  │ │
│  │  │  Info: Source: 1920x1080     │  │  │  Status: Active          │  │ │
│  │  │        Displays: 4 detected  │  │  │  FPS: 60                 │  │ │
│  │  └──────────────────────────────┘  │  │                          │  │ │
│  │                                    │  │                          │  │ │
│  │  ┌──────────────────────────────┐  │  │  ┌────────────────────┐  │  │ │
│  │  │    OUTPUT PREVIEW (50%)      │  │  │  │  Quick Actions     │  │  │ │
│  │  │                              │  │  │  │                    │  │  │ │
│  │  │  ┌────────────────────────┐  │  │  │  │  [Start NDI]       │  │  │ │
│  │  │  │                        │  │  │  │  │  [Calibrate Wall]  │  │  │ │
│  │  │  │   Final Output         │  │  │  │  │  [Fullscreen]      │  │  │ │
│  │  │  │   (As sent to          │  │  │  │  │  [Save Config]     │  │  │ │
│  │  │  │    virtual device)     │  │  │  │  │                    │  │  │ │
│  │  │  │                        │  │  │  │  │                    │  │  │ │
│  │  │  │   [Shows perspective   │  │  │  │  │                    │  │  │ │
│  │  │  │    corrected quads     │  │  │  │  │                    │  │  │ │
│  │  │  │    as they appear on   │  │  │  │  │                    │  │  │ │
│  │  │  │    physical wall]      │  │  │  │  │                    │  │  │ │
│  │  │  │                        │  │  │  │  │                    │  │  │ │
│  │  │  └────────────────────────┘  │  │  │  └────────────────────┘  │  │ │
│  │  │                              │  │  │                          │  │ │
│  │  │  Info: Output: 3840x2160     │  │  │  Shortcuts:              │  │ │
│  │  │        Mode: Video Wall 2x2  │  │  │  Shift+F = Fullscreen    │  │ │
│  │  └──────────────────────────────┘  │  │  ESC = Exit              │  │ │
│  │                                    │  │  │                          │  │ │
│  └────────────────────────────────────┘  └────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Panel Details

### Left Panel: Visual Previews (70% width)

#### Source Preview (Top Half)

**Purpose**: Show the input texture with overlayed display boxes

**Visual Elements**:
- Full input texture (Input 1 or Input 2)
- Semi-transparent colored boxes showing each display's source region
- Box labels (Display 0, Display 1, etc.)
- Source resolution info

**Interactions**:
- Click and drag box corners to adjust mapping (in Mapping tab)
- Hover to highlight corresponding output display
- Scroll to zoom, middle-drag to pan

```rust
pub struct SourcePreview {
    texture_id: imgui::TextureId,
    display_boxes: Vec<DisplayBox>,
    zoom: f32,
    pan_offset: Vec2,
    show_grid: bool,
}

impl SourcePreview {
    pub fn draw(&mut self, ui: &imgui::Ui, available_size: [f32; 2]) {
        // Draw texture
        ui.image(self.texture_id, available_size);
        
        // Draw overlayed boxes for each display
        for (i, box) in self.display_boxes.iter().enumerate() {
            self.draw_display_box(ui, box, i);
        }
    }
    
    fn draw_display_box(&self, ui: &imgui::Ui, box: &DisplayBox, index: usize) {
        // Semi-transparent fill
        // Border with display index label
        // Corner handles for editing
    }
}
```

**Color Coding**:
- Display 0: Cyan `#00FFFF`
- Display 1: Orange `#FF8000`
- Display 2: Lime `#80FF00`
- Display 3: Magenta `#FF00FF`
- (etc. - high contrast colors)

#### Output Preview (Bottom Half)

**Purpose**: Show what is actually being sent to the output/virtual device

**Visual Elements**:
- Final rendered output (after all transforms)
- Shows perspective-corrected quads as they'll appear on wall
- If in video wall mode: shows full matrix output
- If in normal mode: shows single mapped output

**Interactions**:
- Click to identify which source region maps there
- Shows crosshair at cursor with UV coordinates

```rust
pub struct OutputPreview {
    texture_id: imgui::TextureId,
    output_mode: OutputMode,  // Normal, VideoWall, etc.
    show_crosshair: bool,
}

impl OutputPreview {
    pub fn draw(&mut self, ui: &imgui::Ui, available_size: [f32; 2]) {
        // Draw final output texture
        ui.image(self.texture_id, available_size);
        
        // Optional: Draw crosshair with coordinates
        if self.show_crosshair && ui.is_item_hovered() {
            self.draw_crosshair(ui);
        }
    }
}
```

### Right Panel: Controls (30% width)

#### Tabbed Interface

**Inputs Tab**:
```
┌────────────────────────┐
│ Input Sources          │
├────────────────────────┤
│ Input 1: [Active ▼]    │
│ ├─ Source: NDI "Cam1"  │
│ ├─ Resolution: 1920x1080│
│ └─ [Select Source]     │
│                        │
│ Input 2: [Inactive ▼]  │
│ ├─ Source: None        │
│ └─ [Select Source]     │
│                        │
│ Mix: [========|] 75%   │
└────────────────────────┘
```

**Mapping Tab**:
```
┌────────────────────────┐
│ Projection Mapping     │
├────────────────────────┤
│ Editing: [Display 0 ▼] │
│                        │
│ Corners:               │
│ TL: [0.00] [0.00]     │
│ TR: [0.50] [0.00]     │
│ BR: [0.50] [0.50]     │
│ BL: [0.00] [0.50]     │
│                        │
│ Transform:             │
│ Scale X: [======|] 1.0 │
│ Scale Y: [======|] 1.0 │
│ Rot:     [==|====] 15° │
│                        │
│ Blend:                 │
│ Mode: [Normal ▼]       │
│ Opacity: [====|==] 80% │
│                        │
│ [Reset] [Apply All]    │
└────────────────────────┘
```

**Video Wall Tab**:
```
┌────────────────────────┐
│ Video Wall             │
├────────────────────────┤
│ Mode: [2x2 ▼]          │
│                        │
│ Displays:              │
│ [✓] Display 0 (TL)     │
│ [✓] Display 1 (TR)     │
│ [✓] Display 2 (BL)     │
│ [✓] Display 3 (BR)     │
│                        │
│ [Calibrate...]         │
│ [Load Config]          │
│ [Save Config]          │
│                        │
│ Status: Calibrated     │
│ Source: 1920x1080      │
│ Output: 3840x2160      │
└────────────────────────┘
```

**Output Tab**:
```
┌────────────────────────┐
│ Output Settings        │
├────────────────────────┤
│ [✓] Fullscreen         │
│                        │
│ NDI Output:            │
│ Name: [RustyMapper]    │
│ [Start] [Stop]         │
│ Status: Inactive       │
│                        │
│ Local Sharing:         │
│ [✓] Syphon (macOS)     │
│   Name: RustyMapper    │
│                        │
│ Virtual Resolution:    │
│ Width:  [1920]         │
│ Height: [1080]         │
└────────────────────────┘
```

#### Quick Actions Section (Always Visible)

Below the tabs, always accessible:

```
┌────────────────────────┐
│ Quick Actions          │
├────────────────────────┤
│ [▶ Start NDI Output]   │
│ [📹 Calibrate Wall]    │
│ [⛶ Toggle Fullscreen]  │
│ [💾 Save Config]       │
│                        │
│ Status: Ready          │
│ FPS: 60                │
│ Frame: 12345           │
└────────────────────────┘
```

## Implementation

### Preview Texture Management

```rust
pub struct PreviewManager {
    // Source preview (input texture)
    source_texture: wgpu::Texture,
    source_imgui_id: imgui::TextureId,
    
    // Output preview (final render)
    output_texture: wgpu::Texture,
    output_imgui_id: imgui::TextureId,
    
    // ImGui renderer reference
    imgui_renderer: Arc<Mutex<ImGuiRenderer>>,
}

impl PreviewManager {
    /// Update source preview from input texture
    pub fn update_source(&mut self, input_texture: &wgpu::Texture) {
        // Copy input texture to preview texture
        // If dimensions match: direct copy
        // If different: scale with wgpu
    }
    
    /// Update output preview from final render
    pub fn update_output(&mut self, output_texture: &wgpu::Texture) {
        // Similar to source update
    }
    
    /// Register textures with ImGui
    pub fn register_with_imgui(&mut self, renderer: &mut ImGuiRenderer) {
        self.source_imgui_id = renderer.register_texture(&self.source_texture);
        self.output_imgui_id = renderer.register_texture(&self.output_texture);
    }
}
```

### Layout Calculation

```rust
pub fn calculate_layout(window_size: [f32; 2]) -> Layout {
    let margin = 10.0;
    let right_panel_width = (window_size[0] * 0.30).max(300.0);
    let left_panel_width = window_size[0] - right_panel_width - margin * 3.0;
    let panel_height = window_size[1] - margin * 2.0;
    
    Layout {
        left_panel: Rect {
            x: margin,
            y: margin,
            width: left_panel_width,
            height: panel_height,
        },
        source_preview: Rect {
            x: margin,
            y: margin,
            width: left_panel_width,
            height: panel_height * 0.5 - margin * 0.5,
        },
        output_preview: Rect {
            x: margin,
            y: margin + panel_height * 0.5 + margin * 0.5,
            width: left_panel_width,
            height: panel_height * 0.5 - margin * 0.5,
        },
        right_panel: Rect {
            x: margin * 2.0 + left_panel_width,
            y: margin,
            width: right_panel_width,
            height: panel_height,
        },
    }
}
```

### Drawing the Layout

```rust
impl ControlGui {
    pub fn build_ui(&mut self, ui: &mut imgui::Ui) {
        let window_size = ui.window_size();
        let layout = calculate_layout([window_size[0], window_size[1]]);
        
        // Left panel: Source preview
        ui.set_cursor_pos([layout.source_preview.x, layout.source_preview.y]);
        ui.child_window("SourcePreview")
            .size([layout.source_preview.width, layout.source_preview.height])
            .build(|| {
                self.build_source_preview(ui);
            });
        
        // Left panel: Output preview
        ui.set_cursor_pos([layout.output_preview.x, layout.output_preview.y]);
        ui.child_window("OutputPreview")
            .size([layout.output_preview.width, layout.output_preview.height])
            .build(|| {
                self.build_output_preview(ui);
            });
        
        // Right panel: Controls
        ui.set_cursor_pos([layout.right_panel.x, layout.right_panel.y]);
        ui.child_window("Controls")
            .size([layout.right_panel.width, layout.right_panel.height])
            .build(|| {
                self.build_controls_panel(ui);
            });
    }
    
    fn build_source_preview(&mut self, ui: &imgui::Ui) {
        ui.text("Source Preview");
        ui.same_line();
        ui.text_disabled(format!("({}x{})", self.source_width, self.source_height));
        
        let available = ui.content_region_avail();
        
        // Draw the texture
        if let Some(tex_id) = self.preview_manager.source_imgui_id {
            // Maintain aspect ratio
            let aspect = self.source_width as f32 / self.source_height as f32;
            let display_size = fit_rect(available, aspect);
            
            ui.image(tex_id, display_size);
            
            // Draw overlayed display boxes
            if self.show_display_boxes {
                self.draw_display_boxes(ui, display_size);
            }
        }
    }
    
    fn build_output_preview(&mut self, ui: &imgui::Ui) {
        ui.text("Output Preview");
        ui.same_line();
        ui.text_disabled(format!("({}x{})", self.output_width, self.output_height));
        
        let available = ui.content_region_avail();
        
        if let Some(tex_id) = self.preview_manager.output_imgui_id {
            let aspect = self.output_width as f32 / self.output_height as f32;
            let display_size = fit_rect(available, aspect);
            
            ui.image(tex_id, display_size);
        }
    }
    
    fn build_controls_panel(&mut self, ui: &imgui::Ui) {
        // Tabs at top
        if let Some(_tab_bar) = ui.tab_bar("ControlTabs") {
            if let Some(_tab) = ui.tab_item("Inputs") {
                self.build_inputs_tab(ui);
            }
            if let Some(_tab) = ui.tab_item("Mapping") {
                self.build_mapping_tab(ui);
            }
            if let Some(_tab) = ui.tab_item("Video Wall") {
                self.build_videowall_tab(ui);
            }
            if let Some(_tab) = ui.tab_item("Output") {
                self.build_output_tab(ui);
            }
        }
        
        // Quick actions at bottom (always visible)
        ui.separator();
        self.build_quick_actions(ui);
    }
}

/// Fit rectangle while maintaining aspect ratio
fn fit_rect(available: [f32; 2], aspect_ratio: f32) -> [f32; 2] {
    let available_aspect = available[0] / available[1];
    
    if aspect_ratio > available_aspect {
        // Width constrained
        [available[0], available[0] / aspect_ratio]
    } else {
        // Height constrained
        [available[1] * aspect_ratio, available[1]]
    }
}
```

## Visual Enhancements

### Display Box Rendering

```rust
impl DisplayBox {
    pub fn draw(&self, ui: &imgui::Ui, preview_size: [f32; 2], is_selected: bool) {
        let color = self.color;
        let thickness = if is_selected { 3.0 } else { 2.0 };
        
        // Convert normalized UV to pixel coordinates
        let tl = uv_to_pixel(self.corners[0], preview_size);
        let tr = uv_to_pixel(self.corners[1], preview_size);
        let br = uv_to_pixel(self.corners[2], preview_size);
        let bl = uv_to_pixel(self.corners[3], preview_size);
        
        // Draw semi-transparent fill
        let fill_color = [color[0], color[1], color[2], 0.2]; // 20% opacity
        draw_filled_quad(ui, [tl, tr, br, bl], fill_color);
        
        // Draw border
        draw_line(ui, tl, tr, color, thickness);
        draw_line(ui, tr, br, color, thickness);
        draw_line(ui, br, bl, color, thickness);
        draw_line(ui, bl, tl, color, thickness);
        
        // Draw label
        let center = [(tl[0] + br[0]) * 0.5, (tl[1] + br[1]) * 0.5];
        ui.set_cursor_pos(center);
        ui.text_colored(color, &format!("{}", self.display_id));
        
        // Draw corner handles if selected
        if is_selected {
            self.draw_corner_handles(ui, [tl, tr, br, bl]);
        }
    }
}
```

### Status Indicators

```rust
pub fn draw_status_indicator(ui: &imgui::Ui, status: &SystemStatus) {
    match status {
        SystemStatus::Active => {
            ui.text_colored([0.0, 1.0, 0.0, 1.0], "● Active");
        }
        SystemStatus::Warning(msg) => {
            ui.text_colored([1.0, 1.0, 0.0, 1.0], "● Warning");
            ui.same_line();
            ui.text_disabled(msg);
        }
        SystemStatus::Error(msg) => {
            ui.text_colored([1.0, 0.0, 0.0, 1.0], "● Error");
            ui.same_line();
            ui.text(msg);
        }
    }
}
```

## Responsive Behavior

### Minimum Window Size
- Width: 1000px (300px controls + 700px previews)
- Height: 600px

### Resizing Behavior
- Previews maintain aspect ratio (black bars if needed)
- Control panel has minimum width (300px)
- Below minimum: horizontal scrollbar appears

### Collapsible Sections
If vertical space is limited:
- Source preview can collapse to 30% height
- Output preview expands to 70%
- Or stack vertically with tabs

## Future Enhancements

### Interactive Mapping
- Drag display box corners to adjust mapping
- Real-time preview updates
- Snap to grid option

### Zoom & Pan
- Mouse wheel to zoom in previews
- Middle-drag to pan
- Reset view button

### Comparison Mode
- Side-by-side before/after (corner pin vs no transform)
- Toggle overlay on/off

### Recording
- Record preview to video file
- Screenshot button

## Implementation Priority

1. **Basic Layout** (Week 1)
   - Split pane layout
   - Texture display in previews
   - Control panel structure

2. **Display Boxes** (Week 2)
   - Overlay rendering on source
   - Color coding
   - Selection/highlighting

3. **Interactive Elements** (Week 3)
   - Drag to adjust corners
   - Real-time updates
   - Zoom/pan

4. **Polish** (Week 4)
   - Status indicators
   - Responsive behavior
   - Visual enhancements
