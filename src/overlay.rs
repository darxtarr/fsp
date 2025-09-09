// overlay.rs - Fullscreen overlay for region selection

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};
use std::path::PathBuf;

pub struct Overlay {
    hwnd: HWND,
    captures: Vec<PathBuf>,  // Last 5 captures
    selection: Option<RECT>,
}

impl Overlay {
    /// Creates fullscreen dimmed overlay for region selection
    pub fn new(captures: Vec<PathBuf>) -> Result<Self> {
        // TODO: Create fullscreen transparent window
        // Semi-transparent black background
        // Show thumbnail strip at bottom
        todo!("Implement overlay window")
    }
    
    /// Shows the overlay and waits for user selection
    pub fn show_and_select(&mut self) -> Result<Selection> {
        // TODO: Message loop for selection
        // Track mouse for rectangle
        // Handle double-click for fullscreen
        // Handle escape to cancel
        todo!("Implement selection logic")
    }
}

pub enum Selection {
    Region(RECT),
    Fullscreen,
    Window(HWND),
    ExistingCapture(PathBuf),
    Cancelled,
}
