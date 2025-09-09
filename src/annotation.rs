// Vector annotation system
use image::{RgbaImage, Rgba};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub enum Annotation {
    Line { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Rectangle { bounds: Rect, color: Rgba<u8>, width: f32, filled: bool },
    Ellipse { center: Point, rx: f32, ry: f32, color: Rgba<u8>, width: f32, filled: bool },
    Arrow { start: Point, end: Point, color: Rgba<u8>, width: f32 },
    Text { position: Point, content: String, color: Rgba<u8>, size: f32 },
    Blur { region: Rect, intensity: u8 },
}

pub struct AnnotationLayer {
    pub annotations: Vec<Annotation>,
    pub selected: Option<usize>,
}

impl AnnotationLayer {
    pub fn new() -> Self {
        Self {
            annotations: Vec::new(),
            selected: None,
        }
    }
    
    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }
    
    pub fn flatten_to_image(&self, background_path: &Path) -> std::result::Result<RgbaImage, Box<dyn std::error::Error>> {
        let _img = image::open(background_path)?.to_rgba8();
        // TODO: Apply annotations
        Err("Not implemented yet".into())
    }
}