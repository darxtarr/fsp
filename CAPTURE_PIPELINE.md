# Capture and Persistence Pipeline

This document captures architecture decisions for the capture hot path.
It is intentionally explicit so future sessions can continue from the same baseline.

Execution sequencing and implementation checklists live in `IMPLEMENTATION_PLAN.md`.

## Environment Assumptions

- Target includes weak CloudPC/VDI-like machines.
- No dedicated VRAM/GPU should be assumed for core correctness or performance.
- CPU and storage (especially SSD write throughput and latency) are the main bottlenecks.
- User behavior includes bursty back-to-back captures during calls, demos, and video playback.

## First Principles

- A screenshot is a moment in time; do not lose it.
- Selection/overlay must feel instant.
- Disk writes must not block interaction.
- Keep RAM bounded and predictable.

## 4K Size Math (3840x2160)

- Pixels: `8,294,400`
- Raw BGRA: `8,294,400 * 4 = 33,177,600 bytes` (`31.64 MiB`)
- PNG (fast compression): typically 10-30 MB depending on content

## Burst Throughput Math

At 4K raw BGRA:

- `2 captures/s` -> `63.28 MiB/s` ingest
- `5 captures/s` -> `158.20 MiB/s` ingest

Ring buffer memory at 4K:

- `3 slots` -> `94.92 MiB`
- `6 slots` -> `189.84 MiB`

## Pipeline Overview (Hot Path)

1. Capture monitor/window into a preallocated RAM frame slot.
2. Show fullscreen overlay using that RAM frame immediately.
3. Queue full-frame persistence in background (do not block UI).
4. User selects/crops; crop from RAM if frame still resident.
5. Editor opens cropped image while full-frame persistence can continue.
6. Copy/save actions prefer RAM source first, then disk fallback.
7. Once frame is persisted and no session references it, return slot to pool.

## Storage Model

### RAM Frame Pool (bounded)

- Purpose: instant capture/overlay/crop and burst smoothing.
- Suggested default: `6` slots at 4K-class targets (~190 MiB).
- Pool is hard-capped; no unbounded allocation.

### Disk Persistence

- Purpose: durable full-frame history for re-crop/edit after RAM slot reuse.
- Format: **PNG** (fast compression settings).
- Location: `%TEMP%\FSP\` (existing path, unchanged).

Why PNG and not QOI/raw BGRA: The entire point of async persistence is that encode
time doesn't matter — it's off the hot path. PNG is already implemented, every tool
reads it, and the `image` crate handles it. If profiling later shows the background
thread can't keep up during sustained 5+ captures/second bursts, revisit then. Don't
prematurely optimize the part that's now async.

## Backpressure and Failure Policy

- No silent frame drops.
- If RAM pool is exhausted:
  - show immediate user-facing warning,
  - reject/defer new capture command,
  - optionally offer quick cleanup action.
- If disk is near full:
  - raise soft warning at configured threshold,
  - hard-stop persistence before catastrophic failure,
  - surface "cleanup now" action in tray/menu.

## State Model

Storage state per capture:

```
InRam → PersistQueued → Persisted { path, bytes } → Failed { reason }
```

Four states. Each capture has a stable `CaptureId` (monotonic u64) to avoid
mismatches across async operations.

### Why only four states

An earlier draft had 8 states including `OverlayActive` (UI state mixed into
storage state), `CompactionQueued`, `Compacting`, and `Archived` (serving a
compactor process that doesn't exist yet and won't for months). Keeping unused
states in the model adds code paths nobody exercises, bugs nobody catches, and
transitions nobody tests. Add states when the code that uses them ships.

## Communication Model

- UI thread owns authoritative app state.
- Background persistence worker communicates via bounded channel.
- Main loop wakes on posted window message (`WM_APP + N`) to drain events.
- Prefer typed structs/enums over tuple-based cross-module APIs.

## Rapid Capture Behavior

- Design target: repeated `capture -> drag -> save/copy` loops with sub-second cadence.
- RAM pool smooths short bursts.
- Disk persistence guarantees full-frame history for re-crop.

## Rejected Alternatives and Deferred Scope

### Separate compactor process — DEFERRED

An earlier draft designed a long-lived `fsp-compactor` helper process with durable
job queues, atomic write-then-rename, watchdog metrics, and background CPU/IO
priority. This is over-scoped for a tool that doesn't have a working editor yet.

A 4K PNG is ~10-20 MB. You'd need thousands of uncleaned captures before disk
pressure matters on any modern machine. The existing 24-hour auto-expiry (OI-1 in
PRD) handles retention. If disk growth becomes a real problem with real users,
design the compactor then — with real usage data informing the format choice,
queue depth, and throttling policy.

Parked in `FUTURE-FEATURES.md`.

### QOI / raw BGRA for hot spool — REJECTED for now

QOI saves ~50ms encode time on 4K but adds a dependency and a format no other
tool reads natively. Raw BGRA is fastest but 32MB per file. Since persistence is
now async and off the hot path, neither advantage matters. PNG is already working
and universally compatible. Revisit only if background thread throughput becomes
the bottleneck.

### 5-directory module layout — REJECTED

An earlier draft proposed `app/`, `domain/`, `services/`, `ui/`, `platform/win32/`.
This is a 12-file tree for a program that currently has 6 flat files. Keep the flat
`src/` layout. Add files as siblings when needed. Reorganize when the pain of flat
layout is real, not hypothetical. See also: trait abstractions (`CaptureEngine`,
`FramePool`, `SpoolWriter`) — you have one implementation of each, traits buy
nothing until you have two.

### Disk usage indicator in tray — DEFERRED

Good idea, but it's a polish feature. Build the capture pipeline first, then add
the indicator when the tray menu gets its next pass. Tracked in implementation plan.

## Open Decisions

- Default pool size by machine class (6 is the starting guess for 4K).
- Retry strategy for persistence failures (currently: log and mark Failed).
