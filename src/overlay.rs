// Overlay for region selection - covers only the monitor the cursor is on.
use std::path::PathBuf;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::Gdi::{
            AlphaBlend, BeginPaint, BitBlt, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER,
            BI_RGB, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
            CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DIB_RGB_COLORS,
            EndPaint, FillRect, GetStockObject, InvalidateRect, NULL_BRUSH,
            PAINTSTRUCT, PS_SOLID, Rectangle, SelectObject,
            SRCCOPY, HBITMAP, HDC, HBRUSH, UpdateWindow,
        },
        UI::{
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetMessageW, LoadCursorW, RegisterClassW, ShowWindow, TranslateMessage,
                IDC_CROSS, MSG, SW_SHOW, WNDCLASSW, WM_DESTROY, WM_ERASEBKGND,
                WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
                WS_EX_TOPMOST, WS_POPUP, CS_HREDRAW, CS_VREDRAW,
            },
            Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN},
        },
        System::LibraryLoader::GetModuleHandleW,
    },
};

#[derive(Debug, Clone)]
pub enum Selection {
    Region { x: i32, y: i32, width: u32, height: u32, image_path: PathBuf },
    FullScreen { image_path: PathBuf },
    Cancelled,
}

pub struct Overlay {
    capture_path: PathBuf,
    monitor_rect: RECT,
    // Raw BGRA pixels from the capture — used to load the GDI bitmap without
    // a PNG decode round-trip.
    pixels: Vec<u8>,
    pixel_width: u32,
    pixel_height: u32,
    screenshot_dc: HDC,
    screenshot_bitmap: HBITMAP,
    selection_result: Option<Selection>,
    is_selecting: bool,
    start_point: POINT,
    current_point: POINT,
    hwnd: HWND,
}

static mut OVERLAY_INSTANCE: Option<*mut Overlay> = None;

unsafe extern "system" fn overlay_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(ptr) = OVERLAY_INSTANCE {
        let overlay = &mut *ptr;
        match msg {
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                overlay.handle_paint();
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                overlay.handle_mouse_down(lparam);
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                overlay.handle_mouse_up(hwnd, lparam);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if overlay.is_selecting {
                    overlay.handle_mouse_move(hwnd, lparam);
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk = wparam.0 as u16;
                if vk == VK_ESCAPE.0 {
                    overlay.selection_result = Some(Selection::Cancelled);
                    let _ = DestroyWindow(hwnd);
                } else if vk == VK_RETURN.0 {
                    overlay.handle_full_screen_selection(hwnd);
                } else {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                OVERLAY_INSTANCE = None;
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

impl Overlay {
    pub fn new(
        capture_path: PathBuf,
        monitor_rect: RECT,
        pixels: Vec<u8>,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Self {
        Self {
            capture_path,
            monitor_rect,
            pixels,
            pixel_width,
            pixel_height,
            screenshot_dc: HDC::default(),
            screenshot_bitmap: HBITMAP::default(),
            selection_result: None,
            is_selecting: false,
            start_point: POINT { x: 0, y: 0 },
            current_point: POINT { x: 0, y: 0 },
            hwnd: HWND::default(),
        }
    }

    pub fn show_and_select(
        mut self,
    ) -> std::result::Result<Selection, Box<dyn std::error::Error>> {
        unsafe {
            // Load the screenshot into a GDI DC directly from raw pixels —
            // no PNG decode, so the overlay appears much faster.
            let (dc, bitmap) = load_screenshot_bitmap_from_raw(
                &self.pixels,
                self.pixel_width as i32,
                self.pixel_height as i32,
            )?;
            self.screenshot_dc = dc;
            self.screenshot_bitmap = bitmap;

            let hinstance = GetModuleHandleW(None)?;

            let class_name: Vec<u16> = "FSP_OverlayClass\0".encode_utf16().collect();
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_window_proc),
                hInstance: hinstance.into(),
                hbrBackground: HBRUSH::default(),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                hCursor: LoadCursorW(None, IDC_CROSS)?,
                ..Default::default()
            };
            RegisterClassW(&wc);

            let width = self.monitor_rect.right - self.monitor_rect.left;
            let height = self.monitor_rect.bottom - self.monitor_rect.top;
            let window_name: Vec<u16> = "FSP Overlay\0".encode_utf16().collect();

            let hwnd = match CreateWindowExW(
                WS_EX_TOPMOST,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(window_name.as_ptr()),
                WS_POPUP,
                self.monitor_rect.left,
                self.monitor_rect.top,
                width,
                height,
                None,
                None,
                Some(hinstance.into()),
                None,
            ) {
                Ok(h) => h,
                Err(_) => {
                    self.cleanup_bitmaps();
                    return Err("Failed to create overlay window".into());
                }
            };
            self.hwnd = hwnd;

            OVERLAY_INSTANCE = Some(&mut self as *mut _);

            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let _ = UpdateWindow(self.hwnd);

            let mut msg = MSG::default();
            while self.selection_result.is_none() {
                let result = GetMessageW(&mut msg, None, 0, 0);
                if result.0 == 0 || result.0 == -1 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            self.cleanup_bitmaps();
            Ok(self.selection_result.take().unwrap_or(Selection::Cancelled))
        }
    }

    unsafe fn handle_paint(&self) {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(self.hwnd, &mut ps);

        let width = self.monitor_rect.right - self.monitor_rect.left;
        let height = self.monitor_rect.bottom - self.monitor_rect.top;

        // ── Back buffer ──────────────────────────────────────────────────────
        // All drawing goes to back_dc; one final blit to the window DC
        // eliminates the flicker caused by incremental screen updates.
        let back_dc = CreateCompatibleDC(Some(dc));
        let back_bmp = CreateCompatibleBitmap(dc, width, height);
        let old_back = SelectObject(back_dc, back_bmp.into());

        // Step 1: full screenshot at 100% brightness
        BitBlt(back_dc, 0, 0, width, height, Some(self.screenshot_dc), 0, 0, SRCCOPY).ok();

        // Step 2: semi-transparent black dim over everything
        let dim_dc = CreateCompatibleDC(Some(back_dc));
        let dim_bmp = CreateCompatibleBitmap(back_dc, width, height);
        let old_dim = SelectObject(dim_dc, dim_bmp.into());
        let black_brush = CreateSolidBrush(COLORREF(0));
        let full_rect = RECT { left: 0, top: 0, right: width, bottom: height };
        FillRect(dim_dc, &full_rect, black_brush);
        let _ = DeleteObject(black_brush.into());
        let blend = BLENDFUNCTION {
            BlendOp: 0,
            BlendFlags: 0,
            SourceConstantAlpha: 140,
            AlphaFormat: 0,
        };
        let _ = AlphaBlend(back_dc, 0, 0, width, height, dim_dc, 0, 0, width, height, blend);
        SelectObject(dim_dc, old_dim);
        let _ = DeleteObject(dim_bmp.into());
        let _ = DeleteDC(dim_dc);

        // Step 3: punch the selected region through the dim
        if self.is_selecting {
            let left   = self.start_point.x.min(self.current_point.x);
            let top    = self.start_point.y.min(self.current_point.y);
            let right  = self.start_point.x.max(self.current_point.x);
            let bottom = self.start_point.y.max(self.current_point.y);
            let sel_w  = right - left;
            let sel_h  = bottom - top;

            if sel_w > 0 && sel_h > 0 {
                BitBlt(back_dc, left, top, sel_w, sel_h, Some(self.screenshot_dc), left, top, SRCCOPY).ok();

                // Neon green border for maximum contrast
                let pen = CreatePen(PS_SOLID, 2, COLORREF(0x0000FF00));
                let old_pen   = SelectObject(back_dc, pen.into());
                let old_brush = SelectObject(back_dc, GetStockObject(NULL_BRUSH));
                let _ = Rectangle(back_dc, left, top, right, bottom);
                SelectObject(back_dc, old_pen);
                SelectObject(back_dc, old_brush);
                let _ = DeleteObject(pen.into());
            }
        }

        // Single blit → window DC (no partial updates visible to the user)
        BitBlt(dc, 0, 0, width, height, Some(back_dc), 0, 0, SRCCOPY).ok();

        SelectObject(back_dc, old_back);
        let _ = DeleteObject(back_bmp.into());
        let _ = DeleteDC(back_dc);

        let _ = EndPaint(self.hwnd, &ps);
    }

    unsafe fn handle_mouse_down(&mut self, lparam: LPARAM) {
        self.is_selecting = true;
        self.start_point.x = (lparam.0 & 0xFFFF) as i32;
        self.start_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;
        self.current_point = self.start_point;
    }

    unsafe fn handle_mouse_move(&mut self, hwnd: HWND, lparam: LPARAM) {
        self.current_point.x = (lparam.0 & 0xFFFF) as i32;
        self.current_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }

    unsafe fn handle_mouse_up(&mut self, hwnd: HWND, lparam: LPARAM) {
        if !self.is_selecting {
            return;
        }
        self.is_selecting = false;
        self.current_point.x = (lparam.0 & 0xFFFF) as i32;
        self.current_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;

        let left   = self.start_point.x.min(self.current_point.x);
        let top    = self.start_point.y.min(self.current_point.y);
        let width  = (self.start_point.x.max(self.current_point.x) - left) as u32;
        let height = (self.start_point.y.max(self.current_point.y) - top) as u32;

        if width < 5 && height < 5 {
            self.handle_full_screen_selection(hwnd);
            return;
        }

        match self.crop_region(left, top, width, height) {
            Ok(path) => {
                self.selection_result = Some(Selection::Region {
                    x: left, y: top, width, height,
                    image_path: path,
                });
            }
            Err(_) => {
                self.selection_result = Some(Selection::Cancelled);
            }
        }
        let _ = DestroyWindow(hwnd);
    }

    unsafe fn handle_full_screen_selection(&mut self, hwnd: HWND) {
        self.selection_result = Some(Selection::FullScreen {
            image_path: self.capture_path.clone(),
        });
        let _ = DestroyWindow(hwnd);
    }

    fn crop_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
        let img = image::open(&self.capture_path)?;
        let cropped = img.crop_imm(x as u32, y as u32, width, height);

        let temp_dir = std::env::temp_dir().join("FSP");
        std::fs::create_dir_all(&temp_dir)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let path = temp_dir.join(format!("region_{}.png", timestamp));
        cropped.save(&path)?;
        Ok(path)
    }

    unsafe fn cleanup_bitmaps(&self) {
        if !self.screenshot_dc.is_invalid() {
            let _ = DeleteDC(self.screenshot_dc);
        }
        if !self.screenshot_bitmap.0.is_null() {
            let _ = DeleteObject(self.screenshot_bitmap.into());
        }
    }
}

/// Build a GDI memory DC from raw BGRA pixel data (no file I/O, no decode).
unsafe fn load_screenshot_bitmap_from_raw(
    bgra: &[u8],
    width: i32,
    height: i32,
) -> std::result::Result<(HDC, HBITMAP), Box<dyn std::error::Error>> {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        bmiColors: [unsafe { std::mem::zeroed() }; 1],
    };

    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
    let bitmap = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)?;

    std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());

    let mem_dc = CreateCompatibleDC(None);
    SelectObject(mem_dc, bitmap.into());

    Ok((mem_dc, bitmap))
}
