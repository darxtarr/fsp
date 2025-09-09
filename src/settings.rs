// Settings management
use image::Rgba;

#[derive(Clone, Debug)]
pub struct AnnotationPreset {
    pub name: String,
    pub rectangle_color: Rgba<u8>,
    pub arrow_color: Rgba<u8>,
    pub text_color: Rgba<u8>,
    pub line_width: f32,
}

pub struct Settings {
    pub presets: Vec<AnnotationPreset>,
    pub current_preset: usize,
    pub auto_start: bool,
    pub show_tray_icon: bool,
    pub output_path: std::path::PathBuf,
}

impl Settings {
    pub fn load() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        // TODO: Load from INI file
        Ok(Self::default())
    }
    
    pub fn save(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // TODO: Save to INI file
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            presets: vec![
                AnnotationPreset {
                    name: "Dark Mode".to_string(),
                    rectangle_color: Rgba([255, 0, 0, 255]),
                    arrow_color: Rgba([0, 255, 0, 255]),
                    text_color: Rgba([255, 255, 255, 255]),
                    line_width: 2.0,
                },
                AnnotationPreset {
                    name: "Light Mode".to_string(),
                    rectangle_color: Rgba([0, 0, 255, 255]),
                    arrow_color: Rgba([255, 0, 255, 255]),
                    text_color: Rgba([0, 0, 0, 255]),
                    line_width: 2.0,
                },
            ],
            current_preset: 0,
            auto_start: false,
            show_tray_icon: true,
            output_path: std::env::var("USERPROFILE")
                .map(|p| std::path::PathBuf::from(p).join("Pictures").join("Screenshots"))
                .unwrap_or_else(|_| std::path::PathBuf::from("Screenshots")),
        }
    }
}