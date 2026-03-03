// Screen capture functionality using Win32 BitBlt for maximum performance
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

/// Capture a specific rectangle from the virtual desktop and save to disk.
/// Returns (path, raw BGRA pixels, width, height).
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

        let (image_path, raw_bgra) = save_bitmap_to_file(bitmap, width as u32, height as u32)?;

        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(Some(desktop_hwnd), desktop_dc);

        Ok((image_path, raw_bgra, width as u32, height as u32))
    }
}

/// Capture the monitor the cursor is on.
/// Returns (path, monitor RECT, raw BGRA pixels, width, height).
pub fn capture_monitor_at_cursor() -> CaptureResult<(PathBuf, RECT, Vec<u8>, u32, u32)> {
    let rect = get_cursor_monitor_rect()?;
    let (path, pixels, w, h) = capture_rect(rect)?;
    Ok((path, rect, pixels, w, h))
}

/// Capture the monitor the cursor is on (path only).
pub fn capture_screen() -> CaptureResult<PathBuf> {
    let (path, _, _, _, _) = capture_monitor_at_cursor()?;
    Ok(path)
}

/// Capture a specific window.
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

        let (image_path, _) = save_bitmap_to_file(bitmap, width as u32, height as u32)?;

        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem_dc);
        ReleaseDC(Some(hwnd), window_dc);

        Ok(image_path)
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

/// Capture raw bitmap pixels, write a fast PNG to disk, and return both the
/// file path and the raw BGRA pixel buffer (re-used by the overlay to skip
/// the decode step).
unsafe fn save_bitmap_to_file(bitmap: HBITMAP, width: u32, height: u32) -> CaptureResult<(PathBuf, Vec<u8>)> {
    let temp_dir = std::env::temp_dir().join("FSP");
    std::fs::create_dir_all(&temp_dir)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let file_path = temp_dir.join(format!("capture_{}.png", timestamp));

    let mut bmp_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
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

    // Keep raw BGRA for the overlay (avoids a PNG decode round-trip)
    let raw_bgra = buffer.clone();

    // Convert BGRA → RGBA for PNG
    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2);
    }

    // TODO(perf): Fast compression trades speed for file size — a full-monitor
    // capture at 1080p+ easily hits 10 MB. Consider a two-phase approach:
    // write Fast now so the overlay appears immediately, then re-compress
    // aggressively in a background thread while the user annotates or after
    // the session ends. Balance point TBD (Deflate level 6 ~3–4× smaller,
    // ~2–3× slower; also worth evaluating QOI for lossless speed).
    let file = std::fs::File::create(&file_path)?;
    PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        CompressionType::Fast,
        FilterType::NoFilter,
    ).write_image(&buffer, width, height, ExtendedColorType::Rgba8)?;

    Ok((file_path, raw_bgra))
}

// TODO(security): %TEMP%\FSP accumulates full-monitor PNGs indefinitely if
// the user never cleans up. A week of screenshots is a significant data
// exposure risk — any process or person with file-system access to this user
// account can read them. Security model to be decided in a dedicated session:
// options include DPAPI encryption, a master-password + AES scheme, moving
// storage to %APPDATA% with tighter ACLs, and/or automatic retention limits.
// At minimum, enforce a short auto-expiry (e.g. 24 h) and surface a "clean up
// now" action in the tray menu so the user is never sitting on stale captures.

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
