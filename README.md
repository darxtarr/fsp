# FSP - Fast Screenshot Program

A boutique Windows 11 screenshot tool. No frameworks, no bloat, just pure efficiency.

## Quick Start for Implementation

1. Start with `main.rs` - get the message pump and hotkey working
2. Implement `capture.rs` - memory-first capture into bounded frame pool
3. Add `overlay.rs` - region selection from RAM frame (no disk in hot path)
4. Implement `annotation.rs` - pure Rust drawing functions
5. Wire up `clipboard.rs` - Windows clipboard integration
6. Polish with `settings.rs` - INI configuration

## Build

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

## Key Principles

- Single binary, no dependencies beyond Windows APIs + `image` crate (PNG only)
- Memory-first capture with a bounded frame pool, then async disk persistence
- Preserve full-frame snapshots so users can re-crop the same moment later
- Pure Rust rasterization (no GDI/Direct2D for annotations)
- CPU-only friendly (no dedicated GPU/VRAM assumptions)
- Bounded resources: hard RAM cap with explicit backpressure, no silent drops
- Zero network calls, zero telemetry

## Files

- `PRD.MD` - Product requirements and design decisions
- `IMPLEMENTATION.md` - Technical guidelines, module descriptions, settings format
- `CAPTURE_PIPELINE.md` - Performance model, RAM-first pipeline, storage decisions
- `IMPLEMENTATION_PLAN.md` - Phased execution checklist
- `VECTOR_ARCHITECTURE.md` - Vector annotation + raster cache interaction model
- `FUTURE-FEATURES.md` - Deferred scope with rationale for each deferral

## Current Status

- Phases 1-3 (old numbering) complete: multi-monitor capture, overlay selection, hotkey wiring
- Building and running on Windows (x86_64-pc-windows-msvc, windows-rs 0.62)
- Next: RAM-first hot path rewrite (Phase 1 in current plan)

Remember: Measure twice, code once. This is boutique software.
