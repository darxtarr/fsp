# Implementation Guidelines for FSP

## Core Architecture
Windows 11 app with a single UI thread plus background persistence workers.
Capture is memory-first (bounded frame pool), disk writes are async so selection UI
stays responsive even on weak storage.

See `CAPTURE_PIPELINE.md` for the performance model, state machine, and
backpressure design.

## Runtime Constraints
- Optimize for CPU + storage bottlenecks (not GPU acceleration).
- Assume no dedicated VRAM and potentially high IO latency.
- Preserve full-frame snapshots so users can re-crop without re-capturing.

## Dependencies (matches actual Cargo.toml)
```toml
[package]
name = "fsp"
version = "0.1.0"
edition = "2021"
authors = ["Code Boutique"]
description = "Fast Screenshot Program - A lean Windows screenshot tool"

[dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_Controls",
    "Win32_System_LibraryLoader",
    "Win32_Graphics_Gdi",
    "Win32_System_DataExchange",
    "Win32_System_Memory",
    "Win32_UI_Shell",
] }
image = { version = "0.25", default-features = false, features = ["png"] }

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"

[build-dependencies]
winres = "0.1"
```

Note: An earlier draft listed `Graphics_Capture`, `Graphics_DirectX`,
`Graphics_DirectX_Direct3D11`, `Storage_Streams`, and `Foundation_Collections`
features. These were speculative (for potential DXGI duplication capture) and are
not used. The actual build uses GDI BitBlt. If DXGI capture is ever needed, add
the features then.

## Module Structure (flat src/)

All files live directly in `src/`. No subdirectories until the flat layout causes
real pain. See `IMPLEMENTATION_PLAN.md` "Why flat" for rationale.

### main.rs
- Message pump, hotkey registration (PrintScreen + Alt+PrintScreen)
- Tray icon setup and context menu (enable/disable, exit)
- Owns and sequences the capture → overlay → editor flow
- Drains background worker events via `WM_APP + N`

### capture.rs
- **Multi-monitor aware**: `capture_monitor_at_cursor()` → GetCursorPos +
  MonitorFromPoint + GetMonitorInfo → BitBlt that monitor's RECT
- Capture into a preallocated BGRA frame buffer (from frame pool)
- Return frame handle immediately for overlay/editor
- Queue async PNG persistence to `%TEMP%\FSP\` in background
- Auto-cleanup files older than retention limit

### frame_pool.rs *(new — Phase 1)*
- Bounded BGRA slot allocator (ring of N slots, default 6 at 4K = ~190 MiB)
- Hard cap: no unbounded allocation
- Ref-counting or pinning to prevent reuse while overlay/editor holds a reference

### spool_writer.rs *(new — Phase 2)*
- Background thread with bounded channel
- Receives raw BGRA frames, encodes to PNG, writes to disk
- Posts `WM_APP + N` to UI thread on completion or failure

### overlay.rs
- Covers only the monitor from `capture_monitor_at_cursor()`
- Positioned at `rcMonitor.left / rcMonitor.top`, sized to monitor
- Displays from RAM frame (no disk read in hot path)
- Dim effect via AlphaBlend, mouse rectangle selection
- Enter/dblclick = full monitor, Esc = cancel

### editor.rs *(new — Phase 3)*
- Standard `WS_OVERLAPPEDWINDOW` - title bar, minimize, maximize, resize, close
- Displays capture at **strictly 1:1 pixels** - no scaling ever
- Scrollbars when capture > window client area
- Mouse events for annotation drawing
- On close: prompt save if unsaved annotations

### toolbar.rs *(new — Phase 3)*
- Small `WS_OVERLAPPEDWINDOW | WS_EX_TOPMOST` - always on top
- Single horizontal strip: tool buttons + preset selector + Copy + Save + Open in...
- Draggable, position persisted in settings.ini
- Independent of editor window

### annotation.rs
Vector-based annotation system with deferred rasterization. Annotations store
capture-space coordinates (1:1 with the PNG on disk). Rasterization only at
Copy / Save / Open-in time.

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
- Prefer in-memory frame source when resident; fallback to disk if evicted
- Rasterize annotations onto source
- Convert to CF_DIB for clipboard
- OpenClipboard / SetClipboardData Win32 APIs

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
1. Keep a bounded reusable frame pool (ring buffer), default 6 slots
2. Never block overlay/editor on disk writes
3. Persist full frames in background for re-crop history
4. Enforce a hard RAM cap; when saturated, apply backpressure and notify user

## Capture Flow
1. PrintScreen pressed
2. Capture monitor into RAM frame from pool (or reject with warning if pool exhausted)
3. Immediately show overlay from RAM frame
4. In parallel: queue background PNG persistence
5. User selects region; editor opens cropped view
6. Copy/save uses RAM frame if still resident, disk fallback otherwise
7. Once persisted and no UI references it, frame returns to pool
8. Return to idle

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
- [ ] Memory stays within configured cap
- [ ] Works in RDP/CloudPC sessions
- [ ] No admin rights required
- [ ] Binary runs on clean Win11 install
- [ ] Settings persist between sessions
- [ ] Tray icon shows/hides properly
- [ ] P95 hotkey-to-overlay latency meets target
