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
- PrintScreen hotkey registration (RegisterHotKey)
- Tray icon setup and context menu (enable/disable, exit)
- Window class registration
- Owns and sequences the capture → overlay → editor flow

### capture.rs
- **Multi-monitor aware**: accepts a `RECT` (from GetMonitorInfo) rather than
  assuming primary monitor dimensions
- BitBlt only the specified monitor rectangle
- Immediate write to `%TEMP%\FSP\capture_[timestamp].png` → return filepath, free buffer
- Auto-cleanup files older than current session
- `capture_monitor_at_cursor()` → GetCursorPos + MonitorFromPoint + GetMonitorInfo → capture

### overlay.rs
- Covers **only the monitor returned by capture_monitor_at_cursor()**
- Window positioned at `rcMonitor.left / rcMonitor.top`, sized to monitor width/height
- Dim effect over the captured screenshot (draw bitmap then alpha-blend dark rect on top)
- Mouse tracking for rectangle selection
- Enter / double-click = full monitor capture, Esc = cancel
- Returns a `Selection` (region rect or full monitor) + the capture filepath

### editor.rs  *(new - replaces the old annotation overlay concept)*
- Standard `WS_OVERLAPPEDWINDOW` - title bar, minimize, maximize, resize, close
- Displays capture at **strictly 1:1 pixels** - no scaling ever
- Scrollbars appear automatically when capture > window client area
- Handles mouse events for drawing annotations on the image surface
- Coordinates passed directly to annotation layer (1 screen pixel = 1 capture pixel)
- On close: prompts save if unsaved annotations exist

### toolbar.rs  *(new)*
- Small `WS_OVERLAPPEDWINDOW | WS_EX_TOPMOST` window - always on top
- Single horizontal strip: tool buttons + preset selector + Copy + Save + Open in...
- Draggable, position persisted in settings.ini between sessions
- Communicates tool/preset selection back to editor via shared state or messages
- Independent of editor window - survives editor minimize

### annotation.rs
Vector-based annotation system with deferred rasterization. **Unchanged in concept.**
Annotations store capture-space coordinates (1:1 with the PNG on disk).
Rasterization only happens at Copy / Save / Open in... time.

```rust
enum Annotation {
    Line     { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Rectangle{ bounds: Rect, color: Rgba<u8>, width: f32, filled: bool },
    Ellipse  { center: Point, rx: f32, ry: f32, color: Rgba<u8>, width: f32, filled: bool },
    Arrow    { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Text     { position: Point, content: String, color: Rgba<u8>, size: f32 },
    Blur     { region: Rect, intensity: u8 },
}
```

Anti-aliasing: Wu's line algorithm for smooth lines.

### clipboard.rs
- Load PNG from disk, rasterize annotations onto it
- Convert to CF_DIBV5 for clipboard
- OpenClipboard / SetClipboardData Win32 APIs
- Free everything immediately after copy

### settings.rs
```ini
; %APPDATA%\FSP\settings.ini
[DarkMode]
rectangle=#FF6464
arrow=#64FF64
text=#FFFFFF
line_width=3.0

[LightMode]
rectangle=#C80000
arrow=#009600
text=#000000
line_width=2.0

[Custom1]
...

[Custom2]
...

[Output]
DefaultPath=%USERPROFILE%\Pictures\Screenshots
FilePattern=screenshot_{timestamp}.png
ExternalEditor=mspaint.exe   ; for "Open in..." button

[Behavior]
AutoStart=false
ShowTrayIcon=true
HotkeyEnabled=true

[Toolbar]
LastX=100
LastY=100
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
