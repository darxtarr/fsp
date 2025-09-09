// Windows clipboard integration
use image::RgbaImage;

pub fn copy_to_clipboard(_img: &RgbaImage) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement clipboard copy
    Err("Not implemented yet".into())
}