# Implementation Plan (Phased)

Execution checklist for the capture pipeline redesign.
Focuses on measurable latency gains first, then modularity when it earns its keep.

## Success Targets

- Primary UX metric: `hotkey_to_overlay_visible_ms`
- Phase 1 target:
  - p50 < 120 ms at 1080p
  - p95 < 250 ms at 4K local machine
- Burst target (Phase 2): no UI hitching during repeated capture loops until RAM cap/backpressure.
- Reliability target: no silent capture drops; explicit user feedback on pressure/failures.

## Scope and Priorities

Priority order:

1. Fast hot path (memory-first, no blocking encode/write)
2. Correctness under burst load (bounded queues, backpressure)
3. Durable full-frame history for re-crop
4. Disk usage visibility and cleanup UX
5. Modular boundaries (only when flat layout becomes painful)

## File Layout

Flat `src/` — no subdirectories until the pain is real.

Existing files:
- `main.rs` — message pump, hotkeys, tray, sequencing
- `capture.rs` — GDI capture, monitor detection
- `overlay.rs` — fullscreen selection UI
- `annotation.rs` — vector annotation types + rasterization
- `clipboard.rs` — CF_DIB clipboard
- `settings.rs` — INI config

New files (added as needed, as flat siblings):
- `frame_pool.rs` — bounded BGRA slot allocator
- `spool_writer.rs` — background persistence worker
- `editor.rs` — standard window, 1:1 display, scrollable
- `toolbar.rs` — floating tool strip

### Why flat

An earlier draft proposed 5 directories (`app/`, `domain/`, `services/`, `ui/`,
`platform/win32/`) with 12+ files. This is reorganizing a 6-file project into an
enterprise layout before the code exists. The cost of a wrong directory split is
higher than the cost of a flat directory — you can always reorganize later with
`git mv`, but you can't un-abstract a premature module boundary without rewriting
imports everywhere. Similarly, trait abstractions (`CaptureEngine`, `FramePool`,
`SpoolWriter`) are deferred — you have one implementation of each, traits add
indirection for zero benefit until there are two.

## Core Types

```rust
pub struct CaptureId(pub u64);

pub enum StorageState {
    InRam,
    PersistQueued,
    Persisted { path: PathBuf, bytes: u64 },
    Failed { reason: String },
}
```

Message types will be added as concrete structs when the code needs them.
No speculative `AppCommand`/`AppEvent` enums until the message flow is real.

## Phase 1: RAM-First Hot Path + Instrumentation (merged from old Phases 0+1)

### Why merged

The original plan had Phase 0 (add timing probes to existing code) before Phase 1
(rewrite to RAM-first). For a single-developer project, instrumenting code you're
about to rewrite is wasted work. Instead: write the new code with timing probes
built in from the start. You get baseline numbers AND the improvement in one pass.

### Deliverables

- `frame_pool.rs`: bounded BGRA frame allocator (ring of N slots, default 6)
- Refactor `capture.rs`: BitBlt into a preallocated frame slot, return frame handle
- Refactor `overlay.rs`: display from RAM frame (no disk read in hot path)
- Add timing probes at key points:
  - hotkey received → capture complete
  - capture complete → overlay visible
  - overlay visible → selection complete
  - selection complete → editor open
- Keep existing PNG persistence as fallback/safety path
- Background thread: PNG-encode frame and write to `%TEMP%\FSP\` after overlay is shown

### Exit criteria

- Overlay appears from RAM without waiting for any disk encode/write.
- p50/p95 `hotkey_to_overlay_visible_ms` measured and logged.
- Existing functionality (crop, clipboard, save) still works via RAM or disk fallback.

## Phase 2: Background Persistence and Backpressure

### Deliverables

- `spool_writer.rs`: worker thread with bounded channel
- Capture thread enqueues persistence work and returns immediately
- Explicit backpressure when pool or queue is full (user-visible warning, no silent drops)
- Full-frame PNG files kept for re-crop history

### Exit criteria

- Repeated `capture -> drag -> save/copy` loops stay responsive until configured limits.
- No silent drops; user sees clear warning on pressure.

## Phase 3: Editor + Toolbar

### Deliverables

- `editor.rs`: `WS_OVERLAPPEDWINDOW`, 1:1 pixels, scrollable, annotation mouse handling
- `toolbar.rs`: floating `WS_EX_TOPMOST` strip, tool buttons, presets, Copy/Save/Open-in
- Wire editor ↔ toolbar communication
- Toolbar position persisted in `settings.ini`

### Exit criteria

- Full capture → select → annotate → copy/save flow works end-to-end.

## Phase 4: Disk Usage Indicator + Cleanup UX

### Deliverables

- Disk usage accounting for `%TEMP%\FSP\`
- Surface in tray menu: total bytes, capture count, warning state
- "Delete all captures" tray action
- Auto-expiry (24h default, configurable)

### Exit criteria

- User can always see storage footprint and clean up.
- Warning shown when crossing configurable thresholds.

## Phase 5: Modular Refactor (only if needed)

This phase exists as a placeholder. Only execute it if the flat layout is causing
real problems: merge conflicts, circular dependencies, or files exceeding ~500 lines
with unrelated concerns mixed together.

### Potential deliverables

- Extract app state machine if `main.rs` grows too large
- Introduce module boundaries based on actual pain points
- Consider traits/abstractions if there are genuinely two implementations of something

## Risk Register (FSP-Specific)

- **BitBlt into preallocated buffer**: `CreateDIBSection` with pre-existing memory
  may have different lifetime semantics than creating a fresh one per capture.
  Verify reuse is safe or allocate-per-capture and pool the allocation pattern.

- **Frame pool slot reuse while overlay reads it**: If the user holds the overlay
  open and another capture fires, the same slot could be overwritten. Need either
  ref-counting/pinning or treating the overlay's slot as "in use" until dismissed.

- **WM_APP+N event drain starving WM_PAINT**: If persistence events arrive faster
  than the message pump can render, draining the event channel could delay paint.
  Bound the drain count per pump iteration (e.g., process at most 4 events then
  yield to the Windows message queue).

- **BGRA stride alignment**: Some GDI operations require DWORD-aligned strides.
  At 4 bytes/pixel the math works out for any width, but verify with odd monitor
  widths (e.g., 1366px monitors where stride = 5464, which is DWORD-aligned but
  worth confirming).
