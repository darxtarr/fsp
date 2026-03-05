// Editor window — displays captured image at 1:1 with scroll, copy, save.
// Non-blocking: creates window and returns to the main message pump.

use std::path::PathBuf;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
            DeleteDC, DeleteObject, EndPaint, HDC, HBITMAP,
            InvalidateRect, PAINTSTRUCT, SelectObject, SRCCOPY,
        },
        UI::{
            Controls::SetScrollInfo,
            Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_ESCAPE},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
                GetScrollInfo, LoadCursorW, RegisterClassW, ShowWindow,
                IDC_ARROW, SCROLLINFO, SIF_ALL, WNDCLASSW,
                WM_CLOSE, WM_DESTROY, WM_ERASEBKGND,
                WM_HSCROLL, WM_KEYDOWN, WM_MOUSEWHEEL, WM_PAINT, WM_SIZE,
                WM_VSCROLL, WS_HSCROLL, WS_OVERLAPPEDWINDOW, WS_VSCROLL,
                CS_HREDRAW, CS_VREDRAW, SW_SHOW, WINDOW_EX_STYLE,
                SB_HORZ, SB_VERT, SB_LINEUP, SB_LINEDOWN, SB_PAGEUP,
                SB_PAGEDOWN, SB_THUMBTRACK,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
    },
};

static mut EDITOR_INSTANCE: Option<Box<Editor>> = None;

struct Editor {
    hwnd: HWND,
    _pixels: Vec<u8>,
    image_path: PathBuf,
    img_width: i32,
    img_height: i32,
    screenshot_dc: HDC,
    screenshot_bitmap: HBITMAP,
    scroll_x: i32,
    scroll_y: i32,
}

unsafe extern "system" fn editor_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(editor) = EDITOR_INSTANCE.as_mut() {
        match msg {
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                editor.handle_paint();
                LRESULT(0)
            }
            WM_SIZE => {
                editor.update_scrollbars();
                LRESULT(0)
            }
            WM_HSCROLL => {
                editor.handle_hscroll(wparam);
                LRESULT(0)
            }
            WM_VSCROLL => {
                editor.handle_vscroll(wparam);
                LRESULT(0)
            }
            WM_MOUSEWHEEL => {
                editor.handle_mousewheel(wparam);
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u16;
                let ctrl = GetKeyState(VK_CONTROL.0 as i32) < 0;
                if vk == VK_ESCAPE.0 {
                    let _ = DestroyWindow(hwnd);
                } else if ctrl && vk == 0x43 {
                    // Ctrl+C
                    editor.copy_to_clipboard();
                } else if ctrl && vk == 0x53 {
                    // Ctrl+S
                    editor.save_to_disk();
                } else {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                if let Some(ed) = EDITOR_INSTANCE.take() {
                    if !ed.screenshot_dc.is_invalid() {
                        let _ = DeleteDC(ed.screenshot_dc);
                    }
                    if !ed.screenshot_bitmap.0.is_null() {
                        let _ = DeleteObject(ed.screenshot_bitmap.into());
                    }
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl Editor {
    unsafe fn handle_paint(&self) {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(self.hwnd, &mut ps);

        let mut client = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut client);
        let cw = client.right - client.left;
        let ch = client.bottom - client.top;

        // Double-buffer: draw to back_dc, then single blit to window
        let back_dc = CreateCompatibleDC(Some(dc));
        let back_bmp = CreateCompatibleBitmap(dc, cw, ch);
        let old_back = SelectObject(back_dc, back_bmp.into());

        // Fill background with dark gray for areas beyond the image
        let bg_brush = windows::Win32::Graphics::Gdi::CreateSolidBrush(
            windows::Win32::Foundation::COLORREF(0x00303030),
        );
        let bg_rect = RECT { left: 0, top: 0, right: cw, bottom: ch };
        windows::Win32::Graphics::Gdi::FillRect(back_dc, &bg_rect, bg_brush);
        let _ = DeleteObject(bg_brush.into());

        // Blit the image at 1:1 with scroll offset
        let src_x = self.scroll_x;
        let src_y = self.scroll_y;
        let blit_w = cw.min(self.img_width - src_x);
        let blit_h = ch.min(self.img_height - src_y);

        if blit_w > 0 && blit_h > 0 {
            BitBlt(back_dc, 0, 0, blit_w, blit_h,
                   Some(self.screenshot_dc), src_x, src_y, SRCCOPY).ok();
        }

        // Single blit to window
        BitBlt(dc, 0, 0, cw, ch, Some(back_dc), 0, 0, SRCCOPY).ok();

        SelectObject(back_dc, old_back);
        let _ = DeleteObject(back_bmp.into());
        let _ = DeleteDC(back_dc);

        let _ = EndPaint(self.hwnd, &ps);
    }

    unsafe fn update_scrollbars(&self) {
        let mut client = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut client);
        let cw = client.right - client.left;
        let ch = client.bottom - client.top;

        let si_h = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            nMin: 0,
            nMax: self.img_width - 1,
            nPage: cw as u32,
            nPos: self.scroll_x,
            nTrackPos: 0,
        };
        let _ = SetScrollInfo(self.hwnd, SB_HORZ, &si_h, true);

        let si_v = SCROLLINFO {
            cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
            fMask: SIF_ALL,
            nMin: 0,
            nMax: self.img_height - 1,
            nPage: ch as u32,
            nPos: self.scroll_y,
            nTrackPos: 0,
        };
        let _ = SetScrollInfo(self.hwnd, SB_VERT, &si_v, true);
    }

    unsafe fn handle_hscroll(&mut self, wparam: WPARAM) {
        let code = (wparam.0 & 0xFFFF) as u32;
        let mut client = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut client);
        let page = (client.right - client.left) as i32;

        let new_pos = match code {
            x if x == SB_LINEUP.0 as u32 => self.scroll_x - 40,
            x if x == SB_LINEDOWN.0 as u32 => self.scroll_x + 40,
            x if x == SB_PAGEUP.0 as u32 => self.scroll_x - page,
            x if x == SB_PAGEDOWN.0 as u32 => self.scroll_x + page,
            x if x == SB_THUMBTRACK.0 as u32 => {
                let mut si = SCROLLINFO {
                    cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                    fMask: SIF_ALL,
                    ..Default::default()
                };
                let _ = GetScrollInfo(self.hwnd, SB_HORZ, &mut si);
                si.nTrackPos
            }
            _ => return,
        };

        let max = (self.img_width - page).max(0);
        self.scroll_x = new_pos.clamp(0, max);
        self.update_scrollbars();
        let _ = InvalidateRect(Some(self.hwnd), None, false);
    }

    unsafe fn handle_vscroll(&mut self, wparam: WPARAM) {
        let code = (wparam.0 & 0xFFFF) as u32;
        let mut client = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut client);
        let page = (client.bottom - client.top) as i32;

        let new_pos = match code {
            x if x == SB_LINEUP.0 as u32 => self.scroll_y - 40,
            x if x == SB_LINEDOWN.0 as u32 => self.scroll_y + 40,
            x if x == SB_PAGEUP.0 as u32 => self.scroll_y - page,
            x if x == SB_PAGEDOWN.0 as u32 => self.scroll_y + page,
            x if x == SB_THUMBTRACK.0 as u32 => {
                let mut si = SCROLLINFO {
                    cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
                    fMask: SIF_ALL,
                    ..Default::default()
                };
                let _ = GetScrollInfo(self.hwnd, SB_VERT, &mut si);
                si.nTrackPos
            }
            _ => return,
        };

        let max = (self.img_height - page).max(0);
        self.scroll_y = new_pos.clamp(0, max);
        self.update_scrollbars();
        let _ = InvalidateRect(Some(self.hwnd), None, false);
    }

    unsafe fn handle_mousewheel(&mut self, wparam: WPARAM) {
        // High word of wparam is wheel delta (signed)
        let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
        let scroll_amount = -(delta as i32) * 40 / 120;

        let mut client = RECT::default();
        let _ = GetClientRect(self.hwnd, &mut client);
        let page = (client.bottom - client.top) as i32;
        let max = (self.img_height - page).max(0);

        self.scroll_y = (self.scroll_y + scroll_amount).clamp(0, max);
        self.update_scrollbars();
        let _ = InvalidateRect(Some(self.hwnd), None, false);
    }

    fn copy_to_clipboard(&self) {
        let _ = crate::clipboard::copy_file_to_clipboard(&self.image_path);
    }

    fn save_to_disk(&self) {
        let _ = (|| -> Result<(), Box<dyn std::error::Error>> {
            let settings = crate::settings::Settings::load()?;
            std::fs::create_dir_all(&settings.output_path)?;

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis();
            let filename = settings.file_pattern
                .replace("{timestamp}", &timestamp.to_string());
            let dest = settings.output_path.join(&filename);

            std::fs::copy(&self.image_path, &dest)?;
            Ok(())
        })();
    }
}

/// Open the editor window with the given BGRA pixels. Non-blocking.
pub fn open(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    image_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // Close existing editor if any
        close();

        let (dc, bitmap) = crate::overlay::load_screenshot_bitmap_from_raw(
            &pixels, width as i32, height as i32,
        )?;

        let hinstance = GetModuleHandleW(None)?;

        let class_name: Vec<u16> = "FSP_EditorClass\0".encode_utf16().collect();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(editor_window_proc),
            hInstance: hinstance.into(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH::default(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Determine window size: min(image, 80% of work area)
        let mut work_area = RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA,
            0,
            Some(&mut work_area as *mut _ as *mut _),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        let wa_w = work_area.right - work_area.left;
        let wa_h = work_area.bottom - work_area.top;

        let win_w = (width as i32).min(wa_w * 80 / 100);
        let win_h = (height as i32).min(wa_h * 80 / 100);

        // Center on work area
        let x = work_area.left + (wa_w - win_w) / 2;
        let y = work_area.top + (wa_h - win_h) / 2;

        // Window title from filename
        let fname = image_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("capture");
        let title: Vec<u16> = format!("FSP - {}\0", fname).encode_utf16().collect();

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_HSCROLL | WS_VSCROLL,
            x, y, win_w, win_h,
            None,
            None,
            Some(hinstance.into()),
            None,
        )?;

        let editor = Box::new(Editor {
            hwnd,
            _pixels: pixels,
            image_path,
            img_width: width as i32,
            img_height: height as i32,
            screenshot_dc: dc,
            screenshot_bitmap: bitmap,
            scroll_x: 0,
            scroll_y: 0,
        });

        EDITOR_INSTANCE = Some(editor);

        let _ = ShowWindow(hwnd, SW_SHOW);

        // Set initial scrollbar ranges
        if let Some(ed) = EDITOR_INSTANCE.as_ref() {
            ed.update_scrollbars();
        }

        Ok(())
    }
}

/// Close the editor window if open.
pub fn close() {
    unsafe {
        if let Some(editor) = EDITOR_INSTANCE.as_ref() {
            let hwnd = editor.hwnd;
            let _ = DestroyWindow(hwnd);
            // WM_DESTROY handler takes care of cleanup
        }
    }
}

/// Check if the editor window is currently open.
pub fn is_open() -> bool {
    unsafe { EDITOR_INSTANCE.is_some() }
}
