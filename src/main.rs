// FSP - Fast Screenshot Program
// A boutique screenshot tool, crafted with care

#![windows_subsystem = "windows"]

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            WindowsAndMessaging::*,
            Input::KeyboardAndMouse::*,
            Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW},
        },
        Graphics::Gdi::CreateSolidBrush,
    },
};

mod capture;
mod overlay;
mod annotation;
mod clipboard;
mod editor;
mod settings;

const HOTKEY_PRINT_SCREEN: i32 = 1;
const HOTKEY_ALT_PRINT_SCREEN: i32 = 2;
const WM_TRAY_ICON: u32 = WM_USER + 1;

/// Guard against re-entrant hotkey handling. The overlay runs a nested message
/// loop that receives WM_HOTKEY — without this flag, pressing PrintScreen while
/// the overlay is up creates stacked overlays that can't be dismissed.
static mut OVERLAY_ACTIVE: bool = false;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            if OVERLAY_ACTIVE {
                return LRESULT(0);
            }
            match wparam.0 as i32 {
                HOTKEY_PRINT_SCREEN => {
                    let t0 = std::time::Instant::now();
                    if let Ok((path, rect, pixels, pw, ph)) = crate::capture::capture_monitor_at_cursor() {
                        let t1 = std::time::Instant::now();
                        // Close existing editor before new capture
                        crate::editor::close();
                        OVERLAY_ACTIVE = true;
                        let selection = crate::overlay::Overlay::new(path, rect, pixels, pw, ph).show_and_select();
                        OVERLAY_ACTIVE = false;
                        let t2 = std::time::Instant::now();
                        log_timing(
                            t1.duration_since(t0).as_millis(),
                            t2.duration_since(t1).as_millis(),
                            t2.duration_since(t0).as_millis(),
                        );
                        if let Ok(sel) = selection {
                            match sel {
                                crate::overlay::Selection::Region { pixels, pixel_width, pixel_height, image_path, .. }
                                | crate::overlay::Selection::FullScreen { pixels, pixel_width, pixel_height, image_path } => {
                                    let _ = crate::editor::open(pixels, pixel_width, pixel_height, image_path);
                                }
                                crate::overlay::Selection::Cancelled => {}
                            }
                        }
                    }
                }
                HOTKEY_ALT_PRINT_SCREEN => {
                    let _ = crate::capture::capture_active_window();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            if lparam.0 as u32 == WM_RBUTTONUP {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                if let Ok(menu) = CreatePopupMenu() {
                    let exit_label: Vec<u16> = "Exit\0".encode_utf16().collect();
                    AppendMenuW(menu, MF_STRING, 1usize, PCWSTR(exit_label.as_ptr())).ok();
                    let _ = SetForegroundWindow(hwnd);
                    let cmd = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_RIGHTBUTTON,
                        pt.x, pt.y, Some(0), hwnd, None,
                    );
                    DestroyMenu(menu).ok();
                    if cmd.0 == 1 {
                        PostQuitMessage(0);
                    }
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Append timing data to %TEMP%\FSP\timing.log.
/// capture_ms: hotkey → pixels in RAM (BitBlt + GetDIBits + clone, no PNG)
/// overlay_ms: overlay shown → selection complete (includes user interaction time)
/// total_ms:   hotkey → selection complete
fn log_timing(capture_ms: u128, overlay_ms: u128, total_ms: u128) {
    let _ = (|| -> std::io::Result<()> {
        use std::io::Write;
        let dir = std::env::temp_dir().join("FSP");
        let _ = std::fs::create_dir_all(&dir);
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("timing.log"))?;
        writeln!(f, "capture={}ms overlay={}ms total={}ms", capture_ms, overlay_ms, total_ms)?;
        Ok(())
    })();
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let hinstance = GetModuleHandleW(None)?;

        // Register window class
        let class_name: Vec<u16> = "FSP_WindowClass\0".encode_utf16().collect();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: hinstance.into(),
            hbrBackground: CreateSolidBrush(COLORREF(0)),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Create message-only window (no visible UI, just receives messages)
        let window_name: Vec<u16> = "FSP\0".encode_utf16().collect();
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            Some(HWND_MESSAGE),
            None,
            Some(hinstance.into()),
            None,
        )?;

        // Register hotkeys
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_PRINT_SCREEN, HOT_KEY_MODIFIERS(0), VK_SNAPSHOT.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_ALT_PRINT_SCREEN, MOD_ALT, VK_SNAPSHOT.0 as u32);

        // Create tray icon
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY_ICON;
        let tooltip: Vec<u16> = "FSP - Fast Screenshot Program\0".encode_utf16().collect();
        let copy_len = tooltip.len().min(127);
        for (i, &ch) in tooltip.iter().take(copy_len).enumerate() {
            nid.szTip[i] = ch;
        }
        nid.hIcon = LoadIconW(None, IDI_APPLICATION)?;
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);

        // Message pump
        let mut msg = MSG::default();
        loop {
            let result = GetMessageW(&mut msg, None, 0, 0);
            if result.0 == 0 || result.0 == -1 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_PRINT_SCREEN);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ALT_PRINT_SCREEN);

        Ok(())
    }
}
