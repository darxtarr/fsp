# FSP - Fast Screenshot Program

A boutique Windows 11 screenshot tool. No frameworks, no bloat, just pure efficiency.

## Quick Start for Implementation

1. Start with `main.rs` - get the message pump and hotkey working
2. Implement `capture.rs` - just get screenshots saving to disk
3. Add `overlay.rs` - region selection UI
4. Implement `annotation.rs` - pure Rust drawing functions  
5. Wire up `clipboard.rs` - Windows clipboard integration
6. Polish with `settings.rs` - INI configuration

## Build

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

## Key Principles

- Single binary, no dependencies beyond Windows APIs
- Burst capture to disk, not memory
- Pure Rust rasterization (no GDI/Direct2D)
- Under 100MB RAM during use
- Zero network calls

## Files

- `IMPLEMENTATION.md` - Technical details
- `PRD.MD` - Product requirements  
- `FUTURE-FEATURES.md` - Post-MVP enhancements

Remember: Measure twice, code once. This is boutique software.
