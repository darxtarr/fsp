// Overlay for region selection using native Win32 APIs
use std::path::PathBuf;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM, RECT, POINT},
        Graphics::Gdi::{
            CreateSolidBrush, GetDC, ReleaseDC, FillRect, Rectangle, SetBkMode, 
            TextOutW, CreatePen, SelectObject, DeleteObject, COLORREF, HBRUSH, HPEN,
            PS_SOLID, TRANSPARENT, RGB
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, ShowWindow, UpdateWindow, GetMessageW, TranslateMessage,
            DispatchMessageW, RegisterClassW, DefWindowProcW, DestroyWindow,
            WS_POPUP, WS_EX_TOPMOST, WS_EX_LAYERED, SW_SHOW, MSG, WNDCLASSW,
            WM_PAINT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_KEYDOWN,
            WM_DESTROY, CS_HREDRAW, CS_VREDRAW, LoadCursorW, IDC_CROSS,
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, VK_ESCAPE, VK_RETURN,
            GetCursorPos
        },
        System::LibraryLoader::GetModuleHandleW,
    },
};

#[derive(Debug, Clone)]
pub enum Selection {
    Region { x: i32, y: i32, width: u32, height: u32, image_path: PathBuf },
    FullScreen { image_path: PathBuf },
    ActiveWindow { image_path: PathBuf },
    Cancelled,
}

pub struct Overlay {
    captures: Vec<PathBuf>,
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
    if let Some(overlay_ptr) = OVERLAY_INSTANCE {
        let overlay = &mut *overlay_ptr;
        
        match msg {
            WM_PAINT => {
                overlay.handle_paint();
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                overlay.handle_mouse_down(lparam);
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                overlay.handle_mouse_up(lparam);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if overlay.is_selecting {
                    overlay.handle_mouse_move(lparam);
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                match wparam.0 as u32 {
                    VK_ESCAPE => {
                        overlay.selection_result = Some(Selection::Cancelled);
                        DestroyWindow(hwnd);
                        LRESULT(0)
                    }
                    VK_RETURN => {
                        // Double-click equivalent - full screen
                        overlay.handle_full_screen_selection();
                        LRESULT(0)
                    }
                    _ => DefWindowProcW(hwnd, msg, wparam, lparam)
                }
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
    pub fn new(captures: Vec<PathBuf>) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            captures,
            selection_result: None,
            is_selecting: false,
            start_point: POINT { x: 0, y: 0 },
            current_point: POINT { x: 0, y: 0 },
            hwnd: HWND(0),
        })
    }
    
    pub fn show_and_select(&mut self) -> std::result::Result<Selection, Box<dyn std::error::Error>> {
        unsafe {
            // Capture full screen first for overlay background
            let screen_capture = crate::capture::capture_screen()?;
            
            let hinstance = GetModuleHandleW(None)?;
            
            // Register window class
            let class_name = "FSP_OverlayClass\0";
            let class_name_wide: Vec<u16> = class_name.encode_utf16().collect();
            
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(overlay_window_proc),
                hInstance: hinstance.into(),
                hbrBackground: HBRUSH(0), // No background
                lpszClassName: PCWSTR(class_name_wide.as_ptr()),
                hCursor: LoadCursorW(None, IDC_CROSS)?,
                ..Default::default()
            };
            
            RegisterClassW(&wc);
            
            // Get screen dimensions
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            
            // Create fullscreen overlay window
            let window_name = "FSP Overlay\0";
            let window_name_wide: Vec<u16> = window_name.encode_utf16().collect();
            
            self.hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_LAYERED,
                PCWSTR(class_name_wide.as_ptr()),
                PCWSTR(window_name_wide.as_ptr()),
                WS_POPUP,
                0, 0,
                screen_width, screen_height,
                HWND(0),
                None,
                hinstance,
                None,
            );
            
            if self.hwnd.0 == 0 {
                return Err("Failed to create overlay window".into());
            }
            
            // Set overlay instance for window proc
            OVERLAY_INSTANCE = Some(self as *mut _);
            
            // Show window
            ShowWindow(self.hwnd, SW_SHOW);
            UpdateWindow(self.hwnd);
            
            // Message loop
            let mut msg = MSG::default();
            while self.selection_result.is_none() {
                let result = GetMessageW(&mut msg, None, 0, 0);
                if result.0 == 0 || result.0 == -1 {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            
            // Return result or default to cancelled
            Ok(self.selection_result.take().unwrap_or(Selection::Cancelled))
        }
    }
    
    unsafe fn handle_paint(&self) {
        // Create dim overlay effect
        let dc = GetDC(self.hwnd);
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        
        // Fill with semi-transparent black for dim effect
        let brush = CreateSolidBrush(RGB(0, 0, 0));
        let rect = RECT {
            left: 0,
            top: 0,
            right: screen_width,
            bottom: screen_height,
        };
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
        
        // If selecting, draw selection rectangle
        if self.is_selecting {
            let left = self.start_point.x.min(self.current_point.x);
            let top = self.start_point.y.min(self.current_point.y);
            let right = self.start_point.x.max(self.current_point.x);
            let bottom = self.start_point.y.max(self.current_point.y);
            
            // Draw selection rectangle
            let pen = CreatePen(PS_SOLID, 2, RGB(255, 0, 0));
            let old_pen = SelectObject(dc, pen);
            Rectangle(dc, left, top, right, bottom);
            SelectObject(dc, old_pen);
            DeleteObject(pen);
        }
        
        // TODO: Draw thumbnail strip at bottom for recent captures
        
        ReleaseDC(self.hwnd, dc);
    }
    
    unsafe fn handle_mouse_down(&mut self, lparam: LPARAM) {
        self.is_selecting = true;
        self.start_point.x = (lparam.0 & 0xFFFF) as i32;
        self.start_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;
        self.current_point = self.start_point;
    }
    
    unsafe fn handle_mouse_up(&mut self, lparam: LPARAM) {
        if !self.is_selecting {
            return;
        }
        
        self.is_selecting = false;
        self.current_point.x = (lparam.0 & 0xFFFF) as i32;
        self.current_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;
        
        // Calculate region
        let left = self.start_point.x.min(self.current_point.x);
        let top = self.start_point.y.min(self.current_point.y);
        let width = (self.start_point.x.max(self.current_point.x) - left) as u32;
        let height = (self.start_point.y.max(self.current_point.y) - top) as u32;
        
        // Check if it's a click (no drag) - treat as full screen
        if width < 5 && height < 5 {
            self.handle_full_screen_selection();
            return;
        }
        
        // Capture the selected region
        match self.capture_region(left, top, width, height) {
            Ok(image_path) => {
                self.selection_result = Some(Selection::Region {
                    x: left,
                    y: top,
                    width,
                    height,
                    image_path,
                });
            }
            Err(e) => {
                eprintln!("Failed to capture region: {}", e);
                self.selection_result = Some(Selection::Cancelled);
            }
        }
        
        DestroyWindow(self.hwnd);
    }
    
    unsafe fn handle_mouse_move(&mut self, lparam: LPARAM) {
        self.current_point.x = (lparam.0 & 0xFFFF) as i32;
        self.current_point.y = ((lparam.0 >> 16) & 0xFFFF) as i32;
        
        // Trigger repaint to update selection rectangle
        UpdateWindow(self.hwnd);
    }
    
    unsafe fn handle_full_screen_selection(&mut self) {
        match crate::capture::capture_screen() {
            Ok(image_path) => {
                self.selection_result = Some(Selection::FullScreen { image_path });
            }
            Err(e) => {
                eprintln!("Failed to capture full screen: {}", e);
                self.selection_result = Some(Selection::Cancelled);
            }
        }
        DestroyWindow(self.hwnd);
    }
    
    fn capture_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
        // For now, capture full screen then crop
        // TODO: Optimize to capture only the region
        let full_screen_path = crate::capture::capture_screen()?;
        
        // Load and crop image
        let img = image::open(&full_screen_path)?;
        let cropped = img.crop_imm(x as u32, y as u32, width, height);
        
        // Save cropped image
        let temp_dir = std::env::temp_dir().join("FSP");
        std::fs::create_dir_all(&temp_dir)?;
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let filename = format!("region_{}.png", timestamp);
        let file_path = temp_dir.join(filename);
        
        cropped.save(&file_path)?;
        
        // Clean up full screen capture
        let _ = std::fs::remove_file(full_screen_path);
        
        Ok(file_path)
    }
}