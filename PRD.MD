# Product Requirements Document (PRD)
## Project: Windows Screenshot + Annotation Binary (Rust)

---

### Overview
A lightweight, single-binary Windows application written in Rust that intercepts the PrintScreen key to capture screenshots and provide fast annotation (draw, redact, text, save/clipboard). No installer, telemetry, or cross-platform support.

---

## Stakeholders
- Product Owner: [Your Name or Organization]
- Primary Developer: [To be assigned]
- Target Users: Corporate employees, IT professionals, general Windows users needing compliance, performance, and speed.

---

## Background & Problem Statement
Existing Windows screenshot tools are bloated, slow (esp. on Cloud PCs), or require installers and third-party validation. Most do not allow for full PrintScreen hotkey override, minimal annotation, nor do they operate as lean single-file binaries suitable for locked-down environments.

---

## Goals & Objectives
- Deliver instant screenshot capture via PrintScreen.
- Enable on-the-fly, minimal annotation: boxes, arrows, text, redact/blur.
- Export annotated images to clipboard and file.
- Run as a portable, single executable with native Windows feel.
- Provide zero telemetry, auto-updates, or web dependencies.
- Ensure rapid response and minimal resource footprint.

---

## Features

### Core Features
- Global hotkey interception: PrintScreen key.
- High-speed full or region screenshot using Windows Graphics Capture API.
- Mouse-driven annotation overlay (rectangle, ellipse, arrow, text).
- Area blur/redact for compliance.
- Region selection: Screen dimmed, click+drag to select area, double-click for full screen, ALT+PrintScreen for active window.
- Four annotation presets: Dark Mode defaults, Light Mode defaults, Custom 1, Custom 2.
- Quick save (PNG) and clipboard copy.
- Optionally configurable output path.
- Tray icon with basic enable/disable, exit.

### Technical Requirements
- Rust, target: x86_64-pc-windows-msvc
- Architecture: Native Win32 APIs for maximum ISRM approval confidence
- Use crates:
  - `windows` (official Microsoft Win32 bindings)
  - `image` (PNG encoding only)
- Single statically linked binary (`.exe`)
- Release build flags: `opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip="symbols"`
- Memory optimization priority: Minimal idle footprint for CloudPC environments (8-16GB)
- Optional auto-start with OS capability
- No input/output beyond user clipboard, file output, and required notification.

### Compliance & Security
- No Internet connectivity, update checks, or telemetry.
- Process runs in user space, no admin required.
- Invite IT audit of binary: reproducible and validated builds.
- Optional registry modification to disable Win11 Snipping Tool for PrintScreen override (clear warning, documented for IT).

---

## User Stories

- As a user, pressing PrintScreen instantly launches the capture and annotation overlay.
- As a user, I can draw a box/arrow or blur sensitive areas before saving.
- As a user, I can copy the annotated image to clipboard or save as PNG file with one click.
- As IT, I can review, validate, and deploy a single `.exe` on all Windows workstations.

---

## Out of Scope

- No cross-platform (Linux/Mac) support.
- No advanced editing (multi-layer, multi-step undo/redo, etc.).
- No installer, package manager, or bundled drivers.
- No external plugins or macro scripting.

---

## Success Metrics

- RAM usage < 50MB at idle, < 150MB when active on CloudPC (memory is critical on 8-16GB systems).
- Launch time (hotkey pressed to overlay displayed): < 250ms.
- Annotation + export workflow completed in < 2 seconds on a typical CloudPC.
- Deployable/validatable by IT with no network dependencies.
- Binary size irrelevant (disk space not a constraint in terabyte era).

---

## Milestones/Roadmap

1. **Research & Architecture** (Day 1-2)
2. **Hotkey Interception Demo** (Day 3)
3. **Screen Capture Integration** (Day 4-5)
4. **Annotation Overlay MVP** (Day 6-8)
5. **Save/Clipboard Functionality** (Day 9)
6. **Release Build + Static Link** (Day 10)
7. **Internal IT Validation** (Day 11)
8. **QA, Documentation, Delivery** (Day 12)

---

## Questions/Open Issues

- No live preview: dimmed screen with click+drag selection
- Four annotation presets: Dark/Light/Custom1/Custom2
- Native Win32 for ISRM approval and minimal memory footprint
- Is tray icon mandatory for compliance and user comfort?
- Are there required compliance watermarks?

---

## Revised Design Decisions
*(Supersedes any conflicting earlier notes)*

### Multi-Monitor Behavior

FSP captures **only the monitor the cursor is on** when PrintScreen is pressed.

**Rationale:** Composite virtual desktop approaches (dimming all monitors) break on
non-rectangular arrangements (e.g. 32" above a Kamvas tablet, or two 24"s side by side
above a laptop). Coordinate math becomes fragile, DWM compositing fights the overlay,
and dead zones appear in the virtual bounding box. The cursor already encodes intent -
the user is looking at one screen, they want to capture that screen.

**Implementation:**
1. `GetCursorPos()` → cursor coordinates
2. `MonitorFromPoint()` → identify the HMONITOR
3. `GetMonitorInfo()` → get exact `rcMonitor` bounds (position + size in virtual coords)
4. BitBlt only that monitor's rectangle
5. Overlay window positioned and sized exactly to that monitor
6. All region-selection coordinates are relative to that monitor's origin

Alt+PrintScreen continues to capture the active window (unchanged).

---

### Annotation Editor Window

The annotation editor is a **standard OS window showing the capture at 1:1 pixels.**

**Rules:**
- Image is ALWAYS displayed at exactly 1 pixel = 1 pixel. No scaling. Ever.
- If the capture is larger than the available screen space, the window scrolls.
- Standard title bar: minimize, maximize/restore, close, drag, resize - all normal OS behavior.
- The window can be moved to any monitor by the user.

**Why strictly 1:1:**
- Annotation coordinates map directly to capture pixels - zero coordinate mapping math.
- Lines, boxes, text are always crisp - no aliasing from a scaled canvas.
- Exported image is exactly what is shown on screen - true WYSIWYG.
- Eliminates the zoom-level coordination problem entirely.

**What we explicitly do NOT build:**
- No zoom controls
- No "fit to window" scaling
- No "100% / 50% / fit" view modes
- No image resize handles

---

### Annotation Toolbar

The toolbar is a **small, separate, always-on-top floating window** independent of the
editor window.

```
┌──────────────────────────────────────┐
│ □  ○  →  T  ░  │ Dark ▾ │ Copy  Save │
└──────────────────────────────────────┘
```

- Contains: tool buttons, preset selector, Copy, Save
- Draggable anywhere (even to a different monitor)
- Always on top so it is never lost behind the editor
- Thin and unobtrusive - a single strip

**Rationale:** For large captures (full web page + devtools pane = nearly fullscreen),
a toolbar embedded in the editor window would either cover the content or require the
user to scroll to reach it. A separate floating toolbar can be tucked into any free
corner and dragged back when needed.

The toolbar remembers its last position between sessions.

---

### The Escape Hatch

The toolbar includes an **"Open in..."** button that hands the current capture (with
annotations already rasterized) to an external application (Paint, GIMP, etc.).

FSP's job is the fast 90% case: annotate and copy/save in under 15 seconds. For
precise editing, FSP gets out of the way immediately.

The external application is configurable in settings.ini.

---

### The Speed Contract

The **F in FSP means Fast.** Every design decision is measured against this:

| Action                              | Target     |
|-------------------------------------|------------|
| PrintScreen → overlay visible       | < 250ms    |
| Region selected → editor open       | < 500ms    |
| Click Copy → image in clipboard     | < 100ms    |
| Total PrintScreen → clipboard       | < 15s (human annotation time dominates) |

Anything that risks breaking this timeline is cut or deferred to "Open in...".

---

### Revised Application Flow

```
PrintScreen pressed
    ↓
GetCursorPos → MonitorFromPoint → GetMonitorInfo
Capture ONLY that monitor → RAM frame slot (BGRA, bounded pool)
    ↓
Dim overlay appears from RAM immediately (no disk wait)
Background: queue PNG write to %TEMP%\FSP\
User drags to select region  [Esc = cancel, Enter/dblclick = full monitor]
    ↓
Overlay closes, screen returns to normal
    ↓
Editor window opens (standard OS window, 1:1 pixels, scrollable)
Floating toolbar appears (last known position, always-on-top)
    ↓
User annotates: draws on editor window, tools from floating toolbar
Toolbar can be dragged to any monitor at any time
    ↓
Copy  → rasterize annotations (RAM frame or disk fallback) → clipboard → done
Save  → same but writes to %USERPROFILE%\Pictures\Screenshots\
Open in... → rasterize → launch external app with temp file
    ↓
Editor and toolbar close. Back to tray icon, idle.
```

---

## Open Issues — Deferred to Dedicated Sessions

The two topics below were identified during implementation and deliberately left
unresolved. Each needs a focused design pass before any code is written.

---

### OI-1 · Security Model for Captured Screenshots

**Current state (temporary):**
Full-monitor PNG captures accumulate indefinitely in `%TEMP%\FSP\`. Files are
written immediately to disk and never held in RAM (by design — see memory rules),
which means the folder can grow to days or weeks of screenshots with no automatic
cleanup.

**Why this is a real risk:**
A single week of full-monitor captures is a significant data exposure surface.
Any process or person with file-system access to the current Windows user account
can open and read them. ISRM teams correctly flag this regardless of the "but you
could do more damage with the account anyway" argument — policy is policy.

**Options identified (not yet evaluated):**

| Option | Protection level | UX cost | Complexity |
|---|---|---|---|
| Short auto-expiry (e.g. 24 h) + tray "Clean up now" | Minimal | None | Low |
| Move storage to `%APPDATA%\FSP\captures\` | No encryption, but less exposed than TEMP | None | Trivial |
| Windows DPAPI encryption | Files tied to current Windows login; unreadable to other OS users or on other machines. No separate password. | None (transparent) | Medium |
| Master password + AES-256-GCM | Files unreadable even to someone with the Windows account. Survives account sharing. | Password prompt at launch | High |

**Likely direction (to be confirmed):**
A layered approach — short retention limit (24 h hard cap) + DPAPI encryption as
the baseline, with an opt-in master-password layer for high-security users. The
tray menu should always surface a "Delete all captures now" action.

**TODO (code):** see `capture.rs` above `cleanup_old_captures()` for the inline note.

---

### OI-2 · Internal Image Format: Speed vs. File Size — PARTIALLY RESOLVED

**Resolution (March 2026):**
The RAM-first capture pipeline (see `CAPTURE_PIPELINE.md`) eliminates the core
problem. The overlay now displays from a BGRA frame in RAM — no PNG encode in the
hot path at all. PNG is written asynchronously in a background thread while the
user interacts with the overlay.

**What this solves:**
- No "PNG tax" blocking overlay display.
- No double-encode (the old flow wrote PNG for overlay, then re-read it for crop).
- Overlay latency is now bounded by BitBlt + window creation, not by encode speed.

**What remains open:**
- The background spool still uses PNG. This is fine for now — encode time doesn't
  matter when it's async. If profiling shows the background thread can't sustain
  5+ captures/second bursts, consider QOI or raw BGRA for the spool. But don't
  optimize this until it's measured as a bottleneck.
- Export format is PNG (user-facing). No change planned.

**Rejected alternatives:** QOI (adds dependency, nothing reads it natively),
raw BGRA (32MB per 4K frame on disk), two-phase compression (unnecessary when
persistence is fully async). See `CAPTURE_PIPELINE.md` "Rejected Alternatives".

---
