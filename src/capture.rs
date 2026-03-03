// Screen capture functionality using Win32 BitBlt for maximum performance
use std::path::PathBuf;
use windows::{
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
            GetDIBits, GetDC, GetWindowDC, GetWindowRect, ReleaseDC, SelectObject,
            BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HBITMAP, SRCCOPY,
        },
        UI::WindowsAndMessaging::{
            GetCursorPos, GetDesktopWindow, GetForegroundWindow,
            GetMonitorInfoW, MonitorFromPoint,
            HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
    },
};
use image::{RgbaImage, ImageBuffer};

pub type CaptureResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Get the bounding RECT of whichever monitor the cursor is currently on.
/// This is the single source of truth for monitor selection - call it once
/// at hotkey time and pass the result to both capture and overlay.
pub fn get_cursor_monitor_rect() -> CaptureResult<RECT> {
    unsafe {
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos)?;

        let hmonitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);

        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        GetMonitorInfoW(hmonitor, &mut monitor_info)?;

        Ok(monitor_info.rcMonitor)
    }
}

/// Capture a specific rectangle from the virtual desktop and save to disk immediately.
/// The rect is in virtual screen coordinates (as returned by GetMonitorInfo).
pub fn capture_rect(rect: RECT) -> CaptureResult<PathBuf> {
    unsafe {
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            return Err("Invalid capture dimensions".into());
        }

        let desktop_hwnd = GetDesktopWindow();
        let desktop_dc = GetDC(desktop_hwnd);

        if desktop_dc.is_invalid() {
            return Err("Failed to get desktop DC".into());
        }

        let mem_dc = CreateCompatibleDC(desktop_dc);
        let bitmap = CreateCompatibleBitmap(desktop_dc, width, height);
        let old_bitmap = SelectObject(mem_dc, bitmap);

        // Blit from the monitor's position in virtual screen coordinates
        BitBlt(mem_dc, 0, 0, width, height, desktop_dc, rect.left, rect.top, SRCCOPY);

        let image_path = save_bitmap_to_file(bitmap, width as u32, height as u32)?;

        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(desktop_hwnd, desktop_dc);

        Ok(image_path)
    }
}

/// Capture the monitor the cursor is on. Returns the saved file path and the
/// monitor RECT so the caller can position the overlay on the same monitor.
pub fn capture_monitor_at_cursor() -> CaptureResult<(PathBuf, RECT)> {
    let rect = get_cursor_monitor_rect()?;
    let path = capture_rect(rect)?;
    Ok((path, rect))
}

/// Capture the monitor the cursor is on (convenience wrapper, path only).
/// Kept for any legacy callers - prefer capture_monitor_at_cursor() in new code.
pub fn capture_screen() -> CaptureResult<PathBuf> {
    let (path, _) = capture_monitor_at_cursor()?;
    Ok(path)
}

/// Capture specific window
pub fn capture_window(hwnd: HWND) -> CaptureResult<PathBuf> {
    unsafe {
        // Get window rectangle
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect);
        
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        
        if width <= 0 || height <= 0 {
            return Err("Invalid window dimensions".into());
        }
        
        // Get window DC
        let window_dc = GetWindowDC(hwnd);
        if window_dc.is_invalid() {
            return Err("Failed to get window DC".into());
        }
        
        // Create compatible DC and bitmap
        let mem_dc = CreateCompatibleDC(window_dc);
        let bitmap = CreateCompatibleBitmap(window_dc, width, height);
        let old_bitmap = SelectObject(mem_dc, bitmap);
        
        // Copy window to memory bitmap
        BitBlt(
            mem_dc,
            0, 0,
            width, height,
            window_dc,
            0, 0,
            SRCCOPY,
        );
        
        // Save to file immediately
        let image_path = save_bitmap_to_file(bitmap, width as u32, height as u32)?;
        
        // Cleanup
        SelectObject(mem_dc, old_bitmap);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(hwnd, window_dc);
        
        Ok(image_path)
    }
}

/// Get the currently active window for Alt+PrintScreen
pub fn capture_active_window() -> CaptureResult<PathBuf> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return Err("No active window found".into());
        }
        capture_window(hwnd)
    }
}

/// Convert Win32 bitmap to file immediately (memory-efficient approach)
unsafe fn save_bitmap_to_file(bitmap: HBITMAP, width: u32, height: u32) -> CaptureResult<PathBuf> {
    // Create output directory
    let temp_dir = std::env::temp_dir().join("FSP");
    std::fs::create_dir_all(&temp_dir)?;
    
    // Generate filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let filename = format!("capture_{}.png", timestamp);
    let file_path = temp_dir.join(filename);
    
    // Get bitmap info
    let mut bmp_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Negative for top-down bitmap
            biPlanes: 1,
            biBitCount: 32, // 32-bit RGBA
            biCompression: BI_RGB as u32,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [std::mem::zeroed(); 1],
    };
    
    // Calculate buffer size
    let buffer_size = (width * height * 4) as usize;
    let mut buffer = vec![0u8; buffer_size];
    
    // Get bitmap bits
    let dc = CreateCompatibleDC(HDC::default());
    let lines = GetDIBits(
        dc,
        bitmap,
        0,
        height,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut bmp_info,
        DIB_RGB_COLORS,
    );
    DeleteDC(dc);
    
    if lines == 0 {
        return Err("Failed to get bitmap data".into());
    }
    
    // Convert BGRA to RGBA (Windows bitmap format is BGRA)
    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap B and R channels
    }
    
    // Create image and save directly to file
    let image = ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(width, height, buffer)
        .ok_or("Failed to create image buffer")?;
    
    image.save(&file_path)?;
    
    Ok(file_path)
}

/// Clean up old capture files to prevent disk space issues
pub fn cleanup_old_captures() -> CaptureResult<()> {
    let temp_dir = std::env::temp_dir().join("FSP");
    
    if !temp_dir.exists() {
        return Ok(());
    }
    
    let now = std::time::SystemTime::now();
    let max_age = std::time::Duration::from_secs(24 * 60 * 60); // 24 hours
    
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

/// Get list of recent captures for overlay thumbnail display
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
    
    // Sort by modification time (newest first)
    captures.sort_by(|a, b| {
        let a_time = std::fs::metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        let b_time = std::fs::metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });
    
    captures.truncate(limit);
    Ok(captures)
}