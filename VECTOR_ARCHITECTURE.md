# VECTOR ARCHITECTURE UPDATE

## Key Insight: Keep Vectors for Edits, Keep a Bounded Raster Cache for Responsiveness

Vector-first editing is still the right model, but capture is now memory-first so
overlay and first interaction are not blocked by disk writes.

### Updated Flow
1. **Capture** -> write into RAM frame slot (BGRA)
2. **Overlay/Select** -> use RAM frame immediately
3. **Persist** -> full-frame PNG write to disk in background
4. **Edit** -> store only vectors in memory
5. **Export** -> rasterize vectors onto RAM source if present, otherwise disk source

### Memory Model

- Editing state remains vector-heavy and compact.
- Capture pipeline uses a bounded frame pool (ring buffer) for low latency.
- Full-frame durability is guaranteed by background persistence, not by keeping all
  captures in RAM forever.

```rust
struct EditingSession {
    capture_id: CaptureId,
    background_ref: BackgroundRef,    // Ram(frame_id) or Disk(path)
    annotations: Vec<Annotation>,
}
```

### Why This Split Works

- **Latency**: selection starts as soon as pixels are in RAM
- **Safety**: full-frame moment is persisted for re-crop
- **Memory bounds**: RAM stays capped even during capture sprees
- **Clean architecture**: vectors stay independent from storage format decisions

### Practical Notes

- Do not decode PNG in the hot path when RAM pixels are already available.
- Do not silently drop captures when the pool is full; apply explicit backpressure.

See `CAPTURE_PIPELINE.md` for sizing math and storage model.
