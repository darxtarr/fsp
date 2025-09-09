// FSP - Fast Screenshot Program
// A boutique screenshot tool, crafted with care

#![windows_subsystem = "windows"]

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};

mod capture;
mod overlay;
mod annotation;
mod clipboard;
mod settings;

const HOTKEY_ID: i32 = 1;
const WM_TRAY_ICON: u32 = WM_USER + 1;

fn main() -> Result<()> {
    // TODO: Initialize
    // 1. Register window class
    // 2. Create message-only window
    // 3. Register PrintScreen hotkey
    // 4. Create tray icon
    // 5. Message pump
    
    println!("FSP - Fast Screenshot Program");
    println!("Boutique quality, no bloat");
    
    Ok(())
}
