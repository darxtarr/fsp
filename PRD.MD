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


