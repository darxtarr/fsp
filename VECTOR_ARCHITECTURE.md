# 🎯 VECTOR ARCHITECTURE UPDATE

## Key Insight: Screenshots are Immutable Backgrounds

We've pivoted to a **vector-first** approach that's cleaner and more memory-efficient:

### The Flow:
1. **Capture** → Save PNG to disk → Free memory immediately
2. **Edit** → Store only vectors (coordinates, colors) in memory
3. **Export** → Load PNG → Apply vectors → Save/Copy → Free everything

### Memory Usage:
- **Old approach**: Hold 1920×1080×4 = ~8MB per screenshot in RAM
- **New approach**: Hold ~1KB of vector data + filepath

### What This Means:

```rust
// During annotation, we NEVER hold the image
struct EditingSession {
    background_path: PathBuf,        // Just the path!
    annotations: Vec<Annotation>,    // Just vectors!
}

// Only at export time:
fn export() {
    let img = load_from_disk(background_path);  // NOW we load
    apply_all_annotations(&mut img);            // Rasterize vectors
    save_or_copy(img);                          // Ship it
    // img drops here, memory freed
}
```

### Benefits:
- **Undo/Redo**: Trivial - just vector operations
- **Memory**: <50MB even with complex annotations
- **CloudPC friendly**: Minimal RAM usage
- **Clean separation**: Vectors for editing, rasters for export

### What Changed:
- `annotation.rs` is now vector-based with deferred rasterization
- We only implement rasterization functions (Wu's algorithm, etc.) for export
- The overlay can show a low-res preview if needed, but the full image stays on disk

This is proper boutique engineering - elegant, efficient, and exactly what's needed.
