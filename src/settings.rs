// Settings management with INI-style configuration
use image::Rgba;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AnnotationPreset {
    pub name: String,
    pub rectangle_color: Rgba<u8>,
    pub arrow_color: Rgba<u8>,
    pub text_color: Rgba<u8>,
    pub line_width: f32,
}

impl AnnotationPreset {
    pub fn dark_mode() -> Self {
        Self {
            name: "Dark Mode".to_string(),
            rectangle_color: Rgba([255, 100, 100, 255]), // Light red
            arrow_color: Rgba([100, 255, 100, 255]),     // Light green
            text_color: Rgba([255, 255, 255, 255]),      // White
            line_width: 3.0,
        }
    }
    
    pub fn light_mode() -> Self {
        Self {
            name: "Light Mode".to_string(),
            rectangle_color: Rgba([200, 0, 0, 255]),     // Dark red
            arrow_color: Rgba([0, 150, 0, 255]),         // Dark green
            text_color: Rgba([0, 0, 0, 255]),            // Black
            line_width: 2.0,
        }
    }
    
    pub fn custom_1() -> Self {
        Self {
            name: "Custom 1".to_string(),
            rectangle_color: Rgba([255, 165, 0, 255]),   // Orange
            arrow_color: Rgba([0, 191, 255, 255]),       // Deep sky blue
            text_color: Rgba([75, 0, 130, 255]),         // Indigo
            line_width: 2.5,
        }
    }
    
    pub fn custom_2() -> Self {
        Self {
            name: "Custom 2".to_string(),
            rectangle_color: Rgba([255, 20, 147, 255]),  // Deep pink
            arrow_color: Rgba([50, 205, 50, 255]),       // Lime green
            text_color: Rgba([30, 144, 255, 255]),       // Dodger blue
            line_width: 4.0,
        }
    }
}

#[derive(Debug)]
pub struct Settings {
    pub presets: Vec<AnnotationPreset>,
    pub current_preset: usize,
    pub auto_start: bool,
    pub show_tray_icon: bool,
    pub output_path: PathBuf,
    pub file_pattern: String,
    pub hotkey_enabled: bool,
    pub cleanup_old_files: bool,
    pub max_file_age_hours: u64,
}

impl Settings {
    /// Load settings from INI file or create defaults
    pub fn load() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let config_path = get_config_path();
        
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let settings = Self::default();
            // Create config directory if it doesn't exist
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            settings.save()?;
            Ok(settings)
        }
    }
    
    /// Load settings from specific file
    fn load_from_file(path: &Path) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config = parse_ini(&content);
        
        let mut settings = Self::default();
        
        // Parse presets
        for i in 0..4 {
            let preset_name = match i {
                0 => "DarkMode",
                1 => "LightMode", 
                2 => "Custom1",
                3 => "Custom2",
                _ => continue,
            };
            
            if let Some(preset_section) = config.get(preset_name) {
                let preset = &mut settings.presets[i];
                
                if let Some(rect_color) = preset_section.get("rectangle") {
                    if let Some(color) = parse_color(rect_color) {
                        preset.rectangle_color = color;
                    }
                }
                
                if let Some(arrow_color) = preset_section.get("arrow") {
                    if let Some(color) = parse_color(arrow_color) {
                        preset.arrow_color = color;
                    }
                }
                
                if let Some(text_color) = preset_section.get("text") {
                    if let Some(color) = parse_color(text_color) {
                        preset.text_color = color;
                    }
                }
                
                if let Some(width) = preset_section.get("line_width") {
                    if let Ok(w) = width.parse::<f32>() {
                        preset.line_width = w;
                    }
                }
            }
        }
        
        // Parse output settings
        if let Some(output_section) = config.get("Output") {
            if let Some(path) = output_section.get("DefaultPath") {
                settings.output_path = expand_env_vars(path);
            }
            
            if let Some(pattern) = output_section.get("FilePattern") {
                settings.file_pattern = pattern.clone();
            }
        }
        
        // Parse behavior settings
        if let Some(behavior_section) = config.get("Behavior") {
            if let Some(auto_start) = behavior_section.get("AutoStart") {
                settings.auto_start = auto_start == "true";
            }
            
            if let Some(show_tray) = behavior_section.get("ShowTrayIcon") {
                settings.show_tray_icon = show_tray == "true";
            }
            
            if let Some(hotkey) = behavior_section.get("HotkeyEnabled") {
                settings.hotkey_enabled = hotkey == "true";
            }
            
            if let Some(cleanup) = behavior_section.get("CleanupOldFiles") {
                settings.cleanup_old_files = cleanup == "true";
            }
            
            if let Some(age) = behavior_section.get("MaxFileAgeHours") {
                if let Ok(hours) = age.parse::<u64>() {
                    settings.max_file_age_hours = hours;
                }
            }
        }
        
        Ok(settings)
    }
    
    /// Save settings to INI file
    pub fn save(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let config_path = get_config_path();
        
        // Create config directory if needed
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut content = String::new();
        
        // Write presets
        let preset_names = ["DarkMode", "LightMode", "Custom1", "Custom2"];
        for (i, preset) in self.presets.iter().enumerate() {
            if i < preset_names.len() {
                content.push_str(&format!("[{}]\n", preset_names[i]));
                content.push_str(&format!("rectangle={}\n", color_to_hex(&preset.rectangle_color)));
                content.push_str(&format!("arrow={}\n", color_to_hex(&preset.arrow_color)));
                content.push_str(&format!("text={}\n", color_to_hex(&preset.text_color)));
                content.push_str(&format!("line_width={}\n", preset.line_width));
                content.push('\n');
            }
        }
        
        // Write output settings
        content.push_str("[Output]\n");
        content.push_str(&format!("DefaultPath={}\n", self.output_path.display()));
        content.push_str(&format!("FilePattern={}\n", self.file_pattern));
        content.push('\n');
        
        // Write behavior settings
        content.push_str("[Behavior]\n");
        content.push_str(&format!("AutoStart={}\n", self.auto_start));
        content.push_str(&format!("ShowTrayIcon={}\n", self.show_tray_icon));
        content.push_str(&format!("HotkeyEnabled={}\n", self.hotkey_enabled));
        content.push_str(&format!("CleanupOldFiles={}\n", self.cleanup_old_files));
        content.push_str(&format!("MaxFileAgeHours={}\n", self.max_file_age_hours));
        
        fs::write(config_path, content)?;
        Ok(())
    }
    
    /// Get the currently active preset
    pub fn get_current_preset(&self) -> &AnnotationPreset {
        &self.presets[self.current_preset.min(self.presets.len() - 1)]
    }
    
    /// Set the current preset by index
    pub fn set_current_preset(&mut self, index: usize) {
        if index < self.presets.len() {
            self.current_preset = index;
        }
    }
    
    /// Update a preset's colors
    pub fn update_preset(
        &mut self,
        index: usize,
        rectangle_color: Option<Rgba<u8>>,
        arrow_color: Option<Rgba<u8>>,
        text_color: Option<Rgba<u8>>,
        line_width: Option<f32>,
    ) {
        if let Some(preset) = self.presets.get_mut(index) {
            if let Some(color) = rectangle_color {
                preset.rectangle_color = color;
            }
            if let Some(color) = arrow_color {
                preset.arrow_color = color;
            }
            if let Some(color) = text_color {
                preset.text_color = color;
            }
            if let Some(width) = line_width {
                preset.line_width = width;
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            presets: vec![
                AnnotationPreset::dark_mode(),
                AnnotationPreset::light_mode(),
                AnnotationPreset::custom_1(),
                AnnotationPreset::custom_2(),
            ],
            current_preset: 0,
            auto_start: false,
            show_tray_icon: true,
            output_path: get_default_output_path(),
            file_pattern: "screenshot_{timestamp}.png".to_string(),
            hotkey_enabled: true,
            cleanup_old_files: true,
            max_file_age_hours: 24,
        }
    }
}

/// Get the configuration file path
fn get_config_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("FSP").join("settings.ini")
    } else {
        PathBuf::from("settings.ini")
    }
}

/// Get the default output path for screenshots
fn get_default_output_path() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile).join("Pictures").join("Screenshots")
    } else {
        PathBuf::from("Screenshots")
    }
}

/// Parse a simple INI file format
fn parse_ini(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut current_section = String::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        
        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len()-1].to_string();
            result.entry(current_section.clone()).or_insert_with(HashMap::new);
        }
        // Key-value pair
        else if let Some(equals_pos) = line.find('=') {
            if !current_section.is_empty() {
                let key = line[..equals_pos].trim().to_string();
                let value = line[equals_pos+1..].trim().to_string();
                
                if let Some(section) = result.get_mut(&current_section) {
                    section.insert(key, value);
                }
            }
        }
    }
    
    result
}

/// Parse a color from hex string (e.g., "#FF0000" or "FF0000")
pub fn parse_color(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Some(Rgba([r, g, b, 255]));
        }
    }
    
    None
}

/// Convert color to hex string
fn color_to_hex(color: &Rgba<u8>) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
}

/// Expand environment variables in path strings
fn expand_env_vars(path: &str) -> PathBuf {
    let mut result = path.to_string();
    
    // Simple environment variable expansion
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        result = result.replace("%USERPROFILE%", &userprofile);
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        result = result.replace("%APPDATA%", &appdata);
    }
    
    PathBuf::from(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#FF0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_color("00FF00"), Some(Rgba([0, 255, 0, 255])));
        assert_eq!(parse_color("0000FF"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(parse_color("invalid"), None);
    }
    
    #[test]
    fn test_color_to_hex() {
        assert_eq!(color_to_hex(&Rgba([255, 0, 0, 255])), "#FF0000");
        assert_eq!(color_to_hex(&Rgba([0, 255, 0, 255])), "#00FF00");
        assert_eq!(color_to_hex(&Rgba([0, 0, 255, 255])), "#0000FF");
    }
    
    #[test]
    fn test_parse_ini() {
        let content = r#"
[Section1]
key1=value1
key2=value2

[Section2]
key3=value3
"#;
        
        let config = parse_ini(content);
        assert_eq!(config.len(), 2);
        assert_eq!(config["Section1"]["key1"], "value1");
        assert_eq!(config["Section2"]["key3"], "value3");
    }
    
    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.presets.len(), 4);
        assert_eq!(settings.current_preset, 0);
        assert_eq!(settings.show_tray_icon, true);
        assert_eq!(settings.hotkey_enabled, true);
    }
}