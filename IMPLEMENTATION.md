# Implementation Guidelines for FSP

## Core Architecture
Single-threaded Windows 11 application with burst capture to disk and minimal memory footprint.

## Dependencies (Cargo.toml)
```toml
[package]
name = "fsp"
version = "0.1.0"
edition = "2021"

[dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging", 
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_System_LibraryLoader",
    "Win32_Graphics_Gdi",
    "Win32_System_DataExchange",
    "Win32_UI_Shell",
    "Graphics_Capture",
    "Graphics_DirectX",
    "Graphics_DirectX_Direct3D11",
    "Storage_Streams",
    "Foundation_Collections",
] }
image = { version = "0.25", default-features = false, features = ["png"] }

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

## Module Structure

### main.rs
- Message pump
- PrintScreen hotkey registration (RegisterHotKey or WH_KEYBOARD_LL hook)
- Tray icon setup
- Window class registration

### capture.rs
- Windows Graphics Capture API implementation
- Capture to memory buffer
- Immediate write to: `%TEMP%\FSP\capture_[timestamp].png`
- Return just the filepath, not the buffer
- Auto-cleanup files older than current session

### overlay.rs
- Fullscreen transparent window for region selection
- Dim effect: Semi-transparent black overlay
- Mouse tracking for rectangle selection
- Double-click = fullscreen
- ALT+PrintScreen = active window
- Show thumbnail strip of last 5 captures at bottom

### annotation.rs
Vector-based annotation system with deferred rasterization:

```rust
// Vector annotation types
enum Annotation {
    Line { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Rectangle { bounds: Rect, color: Rgba<u8>, width: f32, filled: bool },
    Ellipse { center: Point, rx: f32, ry: f32, color: Rgba<u8>, width: f32, filled: bool },
    Arrow { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Text { position: Point, content: String, color: Rgba<u8>, size: f32 },
    Blur { region: Rect, intensity: u8 },
}

struct AnnotationLayer {
    annotations: Vec<Annotation>,
    selected: Option<usize>,
}

// Rasterization functions (only called on export)
fn rasterize_line(img: &mut RgbaImage, start: Point, end: Point, color: Rgba<u8>, width: f32)
fn rasterize_rectangle(img: &mut RgbaImage, bounds: Rect, color: Rgba<u8>, width: f32, filled: bool)  
fn rasterize_ellipse(img: &mut RgbaImage, center: Point, rx: f32, ry: f32, color: Rgba<u8>, width: f32, filled: bool)
fn rasterize_arrow(img: &mut RgbaImage, start: Point, end: Point, color: Rgba<u8>, width: f32)
fn rasterize_text(img: &mut RgbaImage, position: Point, text: &str, color: Rgba<u8>, size: f32)
fn apply_blur(img: &mut RgbaImage, region: Rect, intensity: u8)

// Export function - only time we touch pixels
fn flatten_to_image(background_path: &Path, annotations: &[Annotation]) -> RgbaImage {
    let mut img = image::open(background_path).to_rgba8();
    for annotation in annotations {
        annotation.rasterize(&mut img);
    }
    img
}
```

Anti-aliasing: Implement Wu's line algorithm for smooth lines (~50 lines of code)

### clipboard.rs
- Load selected PNG from disk
- Convert to CF_DIBV5 format for clipboard
- Use OpenClipboard/SetClipboardData Win32 APIs
- Clean up immediately after copy

### settings.rs
```ini
; %APPDATA%\FSP\settings.ini
[Presets]
DarkMode=rectangle:#FF0000,arrow:#00FF00,text:#FFFFFF
LightMode=rectangle:#0000FF,arrow:#FF00FF,text:#000000  
Custom1=rectangle:#...,arrow:#...,text:#...
Custom2=rectangle:#...,arrow:#...,text:#...

[Output]
DefaultPath=%USERPROFILE%\Pictures\Screenshots
FilePattern=screenshot_{timestamp}.png

[Behavior]
AutoStart=false
ShowTrayIcon=true
```

## Memory Management Rules
1. Never hold screenshot buffer in memory
2. Write to disk immediately after capture
3. Keep only filepath + vector annotations in memory during editing
4. Load and rasterize from disk only at export time (save/copy)
5. Target: <20MB idle, <50MB during annotation (just vectors!)

## Capture Flow
1. PrintScreen pressed
2. Capture screen → Save to temp file → Free buffer
3. Show overlay with last 5 thumbnails (just paths, not loaded images)
4. User selects region or image
5. Store selected image PATH (don't load)
6. Vector annotation mode (just storing coordinates)
7. Save/Copy → Load image → Apply vectors → Export → Free everything
8. Return to idle state

## Critical Implementation Notes
- NO async/await - pure message pump
- NO external HTTP calls ever
- NO unwrap() in release code - handle all errors
- Single binary output, statically linked
- Test on CloudPC/RDP environments
- Use Windows High DPI awareness v2

## Embedded Bitmap Font
Include a simple 8x8 or 8x16 bitmap font as a const byte array. Basic ASCII only (32-126).

## Build Command
```bash
cargo build --release --target x86_64-pc-windows-msvc
```

## Testing Checklist
- [ ] PrintScreen intercepts properly
- [ ] Captures work on multi-monitor setups  
- [ ] Memory stays under 100MB during use
- [ ] Works in RDP/CloudPC sessions
- [ ] No admin rights required
- [ ] Binary runs on clean Win11 install
- [ ] Settings persist between sessions
- [ ] Tray icon shows/hides properly
