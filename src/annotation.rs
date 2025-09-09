// annotation.rs - Vector-based annotation system with deferred rasterization

use image::{RgbaImage, Rgba};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub enum Annotation {
    Line { 
        start: Point, 
        end: Point, 
        color: Rgba<u8>, 
        width: f32 
    },
    Rectangle { 
        bounds: Rect, 
        color: Rgba<u8>, 
        width: f32, 
        filled: bool 
    },
    Ellipse { 
        center: Point, 
        rx: f32, 
        ry: f32, 
        color: Rgba<u8>, 
        width: f32, 
        filled: bool 
    },
    Arrow { 
        start: Point, 
        end: Point, 
        color: Rgba<u8>, 
        width: f32 
    },
    Text { 
        position: Point, 
        content: String, 
        color: Rgba<u8>, 
        size: f32 
    },
    Blur { 
        region: Rect, 
        intensity: u8 
    },
}

pub struct AnnotationLayer {
    annotations: Vec<Annotation>,
    selected: Option<usize>,
    background_path: Option<std::path::PathBuf>,
}

impl AnnotationLayer {
    pub fn new() -> Self {
        AnnotationLayer {
            annotations: Vec::new(),
            selected: None,
            background_path: None,
        }
    }
    
    pub fn set_background(&mut self, path: std::path::PathBuf) {
        self.background_path = Some(path);
    }
    
    pub fn add_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }
    
    pub fn undo(&mut self) {
        self.annotations.pop();
        self.selected = None;
    }
    
    pub fn clear(&mut self) {
        self.annotations.clear();
        self.selected = None;
    }
    
    /// Export: The ONLY time we load the image and rasterize
    pub fn export(&self) -> Result<RgbaImage, image::ImageError> {
        let path = self.background_path.as_ref()
            .ok_or_else(|| image::ImageError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "No background image")
            ))?;
            
        // Load the screenshot from disk
        let mut img = image::open(path)?.to_rgba8();
        
        // Apply all vector annotations
        for annotation in &self.annotations {
            rasterize_annotation(&mut img, annotation);
        }
        
        Ok(img)
    }
}

/// Rasterize a single annotation onto the image
fn rasterize_annotation(img: &mut RgbaImage, annotation: &Annotation) {
    match annotation {
        Annotation::Line { start, end, color, width } => {
            rasterize_line(img, start, end, *color, *width);
        },
        Annotation::Rectangle { bounds, color, width, filled } => {
            rasterize_rectangle(img, bounds, *color, *width, *filled);
        },
        Annotation::Ellipse { center, rx, ry, color, width, filled } => {
            rasterize_ellipse(img, center, *rx, *ry, *color, *width, *filled);
        },
        Annotation::Arrow { start, end, color, width } => {
            rasterize_arrow(img, start, end, *color, *width);
        },
        Annotation::Text { position, content, color, size } => {
            rasterize_text(img, position, content, *color, *size);
        },
        Annotation::Blur { region, intensity } => {
            apply_blur(img, region, *intensity);
        },
    }
}

// ===== Rasterization Functions =====
// These are only called during export, not during editing

/// Draw an anti-aliased line using Wu's algorithm
fn rasterize_line(img: &mut RgbaImage, start: &Point, end: &Point, color: Rgba<u8>, width: f32) {
    // TODO: Implement Wu's line algorithm with width support
    // For now, simple Bresenham as placeholder
    todo!("Implement anti-aliased line with width")
}

/// Draw a rectangle (outline or filled)
fn rasterize_rectangle(img: &mut RgbaImage, bounds: &Rect, color: Rgba<u8>, width: f32, filled: bool) {
    if filled {
        // TODO: Fill rectangle
        todo!("Implement filled rectangle")
    } else {
        // TODO: Draw rectangle outline with given width
        todo!("Implement rectangle outline")
    }
}

/// Draw an ellipse (outline or filled)
fn rasterize_ellipse(img: &mut RgbaImage, center: &Point, rx: f32, ry: f32, color: Rgba<u8>, width: f32, filled: bool) {
    // TODO: Implement midpoint ellipse algorithm
    todo!("Implement ellipse")
}

/// Draw an arrow with head
fn rasterize_arrow(img: &mut RgbaImage, start: &Point, end: &Point, color: Rgba<u8>, width: f32) {
    // Draw the line
    rasterize_line(img, start, end, color, width);
    
    // TODO: Add arrowhead triangles at the end
    todo!("Add arrowhead")
}

/// Draw text using embedded bitmap font
fn rasterize_text(img: &mut RgbaImage, position: &Point, text: &str, color: Rgba<u8>, size: f32) {
    // TODO: Scale and render bitmap font
    todo!("Implement text rendering")
}

/// Apply blur effect to a region
fn apply_blur(img: &mut RgbaImage, region: &Rect, intensity: u8) {
    // TODO: Box blur or pixelation based on intensity
    todo!("Implement blur/pixelation")
}

// ===== Hit Testing for Selection =====
// For interactive editing of vectors

impl Annotation {
    /// Test if a point hits this annotation (for selection)
    pub fn hit_test(&self, point: &Point, tolerance: f32) -> bool {
        match self {
            Annotation::Line { start, end, .. } => {
                point_to_line_distance(point, start, end) < tolerance
            },
            Annotation::Rectangle { bounds, .. } => {
                point_in_rect(point, bounds)
            },
            // TODO: Implement other hit tests
            _ => false
        }
    }
}

fn point_to_line_distance(point: &Point, line_start: &Point, line_end: &Point) -> f32 {
    // TODO: Calculate perpendicular distance from point to line segment
    todo!("Implement point to line distance")
}

fn point_in_rect(point: &Point, rect: &Rect) -> bool {
    point.x >= rect.x && 
    point.x <= rect.x + rect.width &&
    point.y >= rect.y && 
    point.y <= rect.y + rect.height
}

/// Embedded bitmap font (8x16 ASCII characters 32-126)
const FONT_BITMAP: &[u8] = &[
    // TODO: Add actual font data
    // Each character is 8 bits wide, 16 bits tall
    // Total: 95 characters * 16 bytes = 1520 bytes
];
