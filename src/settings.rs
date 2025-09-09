// settings.rs - Configuration management (INI file)

use std::path::PathBuf;
use std::fs;
use image::Rgba;

pub struct Settings {
    pub presets: [Preset; 4],
    pub output_path: PathBuf,
    pub file_pattern: String,
    pub auto_start: bool,
    pub show_tray_icon: bool,
}

pub struct Preset {
    pub name: String,
    pub rectangle_color: Rgba<u8>,
    pub arrow_color: Rgba<u8>,
    pub text_color: Rgba<u8>,
}

impl Settings {
    /// Load settings from %APPDATA%\FSP\settings.ini
    pub fn load() -> Self {
        let config_path = get_config_path();
        
        // TODO: Parse INI file
        // If not exists, create with defaults
        
        Self::default()
    }
    
    /// Save settings to INI file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = get_config_path();
        
        // TODO: Write INI format
        todo!("Implement settings save")
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            presets: [
                Preset {
                    name: "Dark Mode".to_string(),
                    rectangle_color: Rgba([255, 0, 0, 255]),
                    arrow_color: Rgba([0, 255, 0, 255]),
                    text_color: Rgba([255, 255, 255, 255]),
                },
                Preset {
                    name: "Light Mode".to_string(),
                    rectangle_color: Rgba([0, 0, 255, 255]),
                    arrow_color: Rgba([255, 0, 255, 255]),
                    text_color: Rgba([0, 0, 0, 255]),
                },
                Preset {
                    name: "Custom 1".to_string(),
                    rectangle_color: Rgba([255, 128, 0, 255]),
                    arrow_color: Rgba([0, 128, 255, 255]),
                    text_color: Rgba([255, 255, 0, 255]),
                },
                Preset {
                    name: "Custom 2".to_string(),
                    rectangle_color: Rgba([128, 0, 128, 255]),
                    arrow_color: Rgba([0, 128, 128, 255]),
                    text_color: Rgba([192, 192, 192, 255]),
                },
            ],
            output_path: get_default_output_path(),
            file_pattern: "screenshot_{timestamp}.png".to_string(),
            auto_start: false,
            show_tray_icon: true,
        }
    }
}

fn get_config_path() -> PathBuf {
    // %APPDATA%\FSP\settings.ini
    let mut path = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("FSP");
    fs::create_dir_all(&path).ok();
    path.push("settings.ini");
    path
}

fn get_default_output_path() -> PathBuf {
    // %USERPROFILE%\Pictures\Screenshots
    let mut path = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    path.push("Pictures");
    path.push("Screenshots");
    path
}

/// Parse color from hex string (#RRGGBB)
pub fn parse_color(hex: &str) -> Option<Rgba<u8>> {
    // TODO: Parse hex color
    todo!("Implement color parsing")
}
