// Screen capture functionality using Win32 BitBlt for maximum performance
//
// Hot path: BitBlt → extract_pixels (BGRA Vec) → return immediately.
// PNG write is spawned on a background thread so the overlay never waits for disk.
use std::path::PathBuf;
use windows::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
            GetDIBits, GetDC, GetWindowDC, ReleaseDC, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, SRCCOPY,
            GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{
            GetCursorPos, GetDesktopWindow, GetForegroundWindow, GetWindowRect,
        },
    },
};
use image::{ExtendedColorType, ImageEncoder};
use image::codecs::png::{PngEncoder, CompressionType, FilterType};

pub type CaptureResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Get the bounding RECT of whichever monitor the cursor is currently on.
pub fn get_cursor_monitor_rect() -> CaptureResult<RECT> {
    unsafe {
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos)?;

        let hmonitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(hmonitor, &mut monitor_info).ok()?;

        Ok(monitor_info.rcMonitor)
    }
}

/// Extract raw BGRA pixels from a GDI bitmap into a Vec.
/// This is the fast synchronous path — no file I/O.
unsafe fn extract_pixels(bitmap: HBITMAP, width: u32, height: u32) -> CaptureResult<Vec<u8>> {
    let mut bmp_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [std::mem::zeroed(); 1],
    };

    let buffer_size = (width * height * 4) as usize;
    let mut buffer = vec![0u8; buffer_size];

    let dc = CreateCompatibleDC(None);
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        height,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut bmp_info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(dc);

    if lines == 0 {
        return Err("Failed to get bitmap data".into());
    }

    Ok(buffer)
}

/// Generate a timestamped file path under %TEMP%\FSP\.
fn make_capture_path(prefix: &str) -> CaptureResult<PathBuf> {
    let temp_dir = std::env::temp_dir().join("FSP");
    std::fs::create_dir_all(&temp_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    Ok(temp_dir.join(format!("{}_{}.png", prefix, timestamp)))
}

/// Write BGRA pixels as PNG on a background thread. Returns the path immediately.
/// The file may not exist yet when this function returns — it's being written
/// concurrently. This is fine because nothing in the overlay reads from disk.
fn write_png_background(pixels: Vec<u8>, width: u32, height: u32) -> CaptureResult<PathBuf> {
    let path = make_capture_path("capture")?;
    let thread_path = path.clone();

    std::thread::spawn(move || {
        let mut rgba = pixels;
        for chunk in rgba.chunks_exact_mut(4) {
            chunk.swap(0, 2); // BGRA → RGBA
        }
        if let Ok(file) = std::fs::File::create(&thread_path) {
            let _ = PngEncoder::new_with_quality(
                std::io::BufWriter::new(file),
                CompressionType::Fast,
                FilterType::NoFilter,
            ).write_image(&rgba, width, height, ExtendedColorType::Rgba8);
        }
    });

    Ok(path)
}

/// Write BGRA pixels as PNG synchronously. Used for paths where we need the
/// file to exist before returning (e.g. capture_window, crop exports).
fn write_png_sync(pixels: &[u8], width: u32, height: u32, prefix: &str) -> CaptureResult<PathBuf> {
    let path = make_capture_path(prefix)?;

    let mut rgba = pixels.to_vec();
    for chunk in rgba.chunks_exact_mut(4) {
        chunk.swap(0, 2); // BGRA → RGBA
    }

    let file = std::fs::File::create(&path)?;
    PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        CompressionType::Fast,
        FilterType::NoFilter,
    ).write_image(&rgba, width, height, ExtendedColorType::Rgba8)?;

    Ok(path)
}

/// Capture a specific rectangle from the virtual desktop.
/// Returns (path, raw BGRA pixels, width, height).
/// The PNG file is written in the background — the pixels are returned immediately.
pub fn capture_rect(rect: RECT) -> CaptureResult<(PathBuf, Vec<u8>, u32, u32)> {
    unsafe {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return Err("Invalid capture dimensions".into());
        }

        let desktop_hwnd = GetDesktopWindow();
        let desktop_dc = GetDC(Some(desktop_hwnd));

        if desktop_dc.is_invalid() {
            return Err("Failed to get desktop DC".into());
        }

        let mem_dc = CreateCompatibleDC(Some(desktop_dc));
        let bitmap = CreateCompatibleBitmap(desktop_dc, width, height);
        let old_bitmap = SelectObject(mem_dc, bitmap.into());

        BitBlt(mem_dc, 0, 0, width, height, Some(desktop_dc), rect.left, rect.top, SRCCOPY)?;

        let pixels = extract_pixels(bitmap, width as u32, height as u32)?;

        // Clean up GDI objects before spawning the thread
        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(Some(desktop_hwnd), desktop_dc);

        // Clone for the background PNG writer; pixels stay for the overlay
        let path = write_png_background(pixels.clone(), width as u32, height as u32)?;

        Ok((path, pixels, width as u32, height as u32))
    }
}

/// Capture the monitor the cursor is on.
/// Returns (path, monitor RECT, raw BGRA pixels, width, height).
pub fn capture_monitor_at_cursor() -> CaptureResult<(PathBuf, RECT, Vec<u8>, u32, u32)> {
    let rect = get_cursor_monitor_rect()?;
    let (path, pixels, w, h) = capture_rect(rect)?;
    Ok((path, rect, pixels, w, h))
}

/// Capture the monitor the cursor is on (path only, synchronous PNG write).
pub fn capture_screen() -> CaptureResult<PathBuf> {
    let (path, _, _, _, _) = capture_monitor_at_cursor()?;
    Ok(path)
}

/// Capture a specific window (synchronous PNG write).
pub fn capture_window(hwnd: HWND) -> CaptureResult<PathBuf> {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return Err("Invalid window dimensions".into());
        }

        let window_dc = GetWindowDC(Some(hwnd));
        if window_dc.is_invalid() {
            return Err("Failed to get window DC".into());
        }

        let mem_dc = CreateCompatibleDC(Some(window_dc));
        let bitmap = CreateCompatibleBitmap(window_dc, width, height);
        let old_bitmap = SelectObject(mem_dc, bitmap.into());

        BitBlt(mem_dc, 0, 0, width, height, Some(window_dc), 0, 0, SRCCOPY)?;

        let pixels = extract_pixels(bitmap, width as u32, height as u32)?;

        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(Some(hwnd), window_dc);

        let path = write_png_sync(&pixels, width as u32, height as u32, "capture")?;
        Ok(path)
    }
}

/// Get the currently active window for Alt+PrintScreen.
pub fn capture_active_window() -> CaptureResult<PathBuf> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Err("No active window found".into());
        }
        capture_window(hwnd)
    }
}

/// Clean up capture files older than 24 hours.
pub fn cleanup_old_captures() -> CaptureResult<()> {
    let temp_dir = std::env::temp_dir().join("FSP");

    if !temp_dir.exists() {
        return Ok(());
    }

    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(24 * 60 * 60);

    for entry in std::fs::read_dir(temp_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "png" {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(created) = metadata.created() {
                        if now.duration_since(created).unwrap_or_default() > max_age {
                            let _ = std::fs::remove_file(path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Get list of recent captures.
pub fn get_recent_captures(limit: usize) -> CaptureResult<Vec<PathBuf>> {
    let temp_dir = std::env::temp_dir().join("FSP");

    if !temp_dir.exists() {
        return Ok(Vec::new());
    }

    let mut captures = Vec::new();

    for entry in std::fs::read_dir(temp_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "png" && path.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with("capture_")) {
                captures.push(path);
            }
        }
    }

    captures.sort_by(|a, b| {
        let a_time = std::fs::metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        let b_time = std::fs::metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });

    captures.truncate(limit);
    Ok(captures)
}
