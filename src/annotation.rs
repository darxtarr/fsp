// Vector annotation system with deferred rasterization for memory efficiency
use image::{RgbaImage, Rgba, ImageBuffer};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    pub fn distance_to(&self, other: &Point) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.x && point.x <= self.x + self.width &&
        point.y >= self.y && point.y <= self.y + self.height
    }
    
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
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

impl Annotation {
    /// Check if a point is near this annotation (for selection/editing)
    pub fn contains_point(&self, point: &Point, tolerance: f32) -> bool {
        match self {
            Annotation::Line { start, end, width, .. } => {
                point_to_line_distance(point, start, end) <= width.max(tolerance)
            }
            Annotation::Rectangle { bounds, .. } => {
                bounds.contains(point)
            }
            Annotation::Ellipse { center, rx, ry, .. } => {
                let dx = (point.x - center.x) / rx;
                let dy = (point.y - center.y) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Annotation::Arrow { start, end, width, .. } => {
                point_to_line_distance(point, start, end) <= width.max(tolerance)
            }
            Annotation::Text { position, size, .. } => {
                // Approximate text bounds
                let text_width = size * 10.0; // Rough estimate
                let text_height = size;
                let bounds = Rect::new(position.x, position.y, text_width, text_height);
                bounds.contains(point)
            }
            Annotation::Blur { region, .. } => {
                region.contains(point)
            }
        }
    }
    
    /// Rasterize this annotation onto the image (only called at export time)
    pub fn rasterize(&self, img: &mut RgbaImage) {
        match self {
            Annotation::Line { start, end, color, width } => {
                rasterize_line(img, start, end, *color, *width);
            }
            Annotation::Rectangle { bounds, color, width, filled } => {
                rasterize_rectangle(img, bounds, *color, *width, *filled);
            }
            Annotation::Ellipse { center, rx, ry, color, width, filled } => {
                rasterize_ellipse(img, center, *rx, *ry, *color, *width, *filled);
            }
            Annotation::Arrow { start, end, color, width } => {
                rasterize_arrow(img, start, end, *color, *width);
            }
            Annotation::Text { position, content, color, size } => {
                rasterize_text(img, position, content, *color, *size);
            }
            Annotation::Blur { region, intensity } => {
                apply_blur(img, region, *intensity);
            }
        }
    }
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
    
    pub fn add_annotation(&mut self, annotation: Annotation) -> usize {
        self.annotations.push(annotation);
        let index = self.annotations.len() - 1;
        self.selected = Some(index);
        index
    }
    
    pub fn remove_selected(&mut self) -> bool {
        if let Some(index) = self.selected {
            if index < self.annotations.len() {
                self.annotations.remove(index);
                self.selected = None;
                return true;
            }
        }
        false
    }
    
    pub fn select_at_point(&mut self, point: &Point, tolerance: f32) -> Option<usize> {
        // Search from back to front (top to bottom)
        for (index, annotation) in self.annotations.iter().enumerate().rev() {
            if annotation.contains_point(point, tolerance) {
                self.selected = Some(index);
                return Some(index);
            }
        }
        self.selected = None;
        None
    }
    
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }
    
    /// Flatten all annotations to a single image (memory-efficient approach)
    pub fn flatten_to_image(&self, background_path: &Path) -> std::result::Result<RgbaImage, Box<dyn std::error::Error>> {
        // Load background image
        let mut img = image::open(background_path)?.to_rgba8();
        
        // Apply each annotation in order
        for annotation in &self.annotations {
            annotation.rasterize(&mut img);
        }
        
        Ok(img)
    }
    
    /// Get the currently selected annotation
    pub fn get_selected(&self) -> Option<&Annotation> {
        self.selected.and_then(|index| self.annotations.get(index))
    }
    
    /// Get a mutable reference to the currently selected annotation
    pub fn get_selected_mut(&mut self) -> Option<&mut Annotation> {
        if let Some(index) = self.selected {
            self.annotations.get_mut(index)
        } else {
            None
        }
    }
}

/// Wu's line algorithm for anti-aliased lines
fn rasterize_line(img: &mut RgbaImage, start: &Point, end: &Point, color: Rgba<u8>, width: f32) {
    let x0 = start.x as i32;
    let y0 = start.y as i32;
    let x1 = end.x as i32;
    let y1 = end.y as i32;
    
    // Simple line drawing for now - can be enhanced with Wu's algorithm
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;
    
    let half_width = (width / 2.0) as i32;
    
    loop {
        // Draw thick line by drawing a small rectangle at each point
        for dx in -half_width..=half_width {
            for dy in -half_width..=half_width {
                let px = x + dx;
                let py = y + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
        
        if x == x1 && y == y1 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

fn rasterize_rectangle(img: &mut RgbaImage, bounds: &Rect, color: Rgba<u8>, width: f32, filled: bool) {
    let x = bounds.x as i32;
    let y = bounds.y as i32;
    let w = bounds.width as i32;
    let h = bounds.height as i32;
    let line_width = width as i32;
    
    if filled {
        // Fill the rectangle
        for py in y..(y + h) {
            for px in x..(x + w) {
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    } else {
        // Draw rectangle outline
        for i in 0..line_width {
            // Top and bottom lines
            for px in x..(x + w) {
                if px >= 0 && (px as u32) < img.width() {
                    if y + i >= 0 && (y + i) < img.height() as i32 {
                        img.put_pixel(px as u32, (y + i) as u32, color);
                    }
                    if y + h - 1 - i >= 0 && (y + h - 1 - i) < img.height() as i32 {
                        img.put_pixel(px as u32, (y + h - 1 - i) as u32, color);
                    }
                }
            }
            
            // Left and right lines
            for py in y..(y + h) {
                if py >= 0 && (py as u32) < img.height() {
                    if x + i >= 0 && (x + i) < img.width() as i32 {
                        img.put_pixel((x + i) as u32, py as u32, color);
                    }
                    if x + w - 1 - i >= 0 && (x + w - 1 - i) < img.width() as i32 {
                        img.put_pixel((x + w - 1 - i) as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

fn rasterize_ellipse(img: &mut RgbaImage, center: &Point, rx: f32, ry: f32, color: Rgba<u8>, _width: f32, filled: bool) {
    let cx = center.x as i32;
    let cy = center.y as i32;
    let rx = rx as i32;
    let ry = ry as i32;
    
    // Simple ellipse drawing using the equation (x-cx)²/rx² + (y-cy)²/ry² <= 1
    for py in (cy - ry)..(cy + ry + 1) {
        for px in (cx - rx)..(cx + rx + 1) {
            if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                let dx = px - cx;
                let dy = py - cy;
                let distance_sq = (dx * dx) as f32 / (rx * rx) as f32 + (dy * dy) as f32 / (ry * ry) as f32;
                
                if filled {
                    if distance_sq <= 1.0 {
                        img.put_pixel(px as u32, py as u32, color);
                    }
                } else {
                    // Draw outline (simple approximation)
                    if distance_sq >= 0.9 && distance_sq <= 1.1 {
                        img.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

fn rasterize_arrow(img: &mut RgbaImage, start: &Point, end: &Point, color: Rgba<u8>, width: f32) {
    // Draw the line first
    rasterize_line(img, start, end, color, width);
    
    // Calculate arrow head
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    
    if length > 0.0 {
        let arrow_length = width * 3.0;
        let arrow_width = width * 2.0;
        
        let unit_x = dx / length;
        let unit_y = dy / length;
        
        // Arrow head points
        let head1 = Point::new(
            end.x - arrow_length * unit_x + arrow_width * unit_y,
            end.y - arrow_length * unit_y - arrow_width * unit_x,
        );
        let head2 = Point::new(
            end.x - arrow_length * unit_x - arrow_width * unit_y,
            end.y - arrow_length * unit_y + arrow_width * unit_x,
        );
        
        // Draw arrow head lines
        rasterize_line(img, end, &head1, color, width);
        rasterize_line(img, end, &head2, color, width);
    }
}

fn rasterize_text(img: &mut RgbaImage, position: &Point, text: &str, color: Rgba<u8>, size: f32) {
    // Simple bitmap font rendering - for now just draw a placeholder rectangle
    // TODO: Implement proper bitmap font rendering
    let x = position.x as i32;
    let y = position.y as i32;
    let char_width = (size * 0.6) as i32;
    let char_height = size as i32;
    
    for (i, _ch) in text.chars().enumerate() {
        let char_x = x + (i as i32) * char_width;
        
        // Draw a simple rectangle for each character (placeholder)
        for py in y..(y + char_height) {
            for px in char_x..(char_x + char_width) {
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    // Simple character outline
                    if py == y || py == y + char_height - 1 || px == char_x || px == char_x + char_width - 1 {
                        img.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

fn apply_blur(img: &mut RgbaImage, region: &Rect, intensity: u8) {
    let x = region.x as u32;
    let y = region.y as u32;
    let w = region.width as u32;
    let h = region.height as u32;
    
    let blur_radius = intensity as u32;
    
    // Simple box blur
    for py in y..(y + h).min(img.height()) {
        for px in x..(x + w).min(img.width()) {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            let mut count = 0u32;
            
            // Average pixels in blur radius
            for by in py.saturating_sub(blur_radius)..=(py + blur_radius).min(img.height() - 1) {
                for bx in px.saturating_sub(blur_radius)..=(px + blur_radius).min(img.width() - 1) {
                    let pixel = img.get_pixel(bx, by);
                    r += pixel[0] as u32;
                    g += pixel[1] as u32;
                    b += pixel[2] as u32;
                    a += pixel[3] as u32;
                    count += 1;
                }
            }
            
            if count > 0 {
                let blurred = Rgba([
                    (r / count) as u8,
                    (g / count) as u8,
                    (b / count) as u8,
                    (a / count) as u8,
                ]);
                img.put_pixel(px, py, blurred);
            }
        }
    }
}

/// Calculate distance from point to line segment
fn point_to_line_distance(point: &Point, line_start: &Point, line_end: &Point) -> f32 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let length_sq = dx * dx + dy * dy;
    
    if length_sq == 0.0 {
        // Line start and end are the same point
        return point.distance_to(line_start);
    }
    
    // Calculate projection of point onto line
    let t = ((point.x - line_start.x) * dx + (point.y - line_start.y) * dy) / length_sq;
    let t = t.clamp(0.0, 1.0);
    
    // Find closest point on line segment
    let closest = Point::new(
        line_start.x + t * dx,
        line_start.y + t * dy,
    );
    
    point.distance_to(&closest)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }
    
    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 10.0, 20.0, 20.0);
        assert!(rect.contains(&Point::new(15.0, 15.0)));
        assert!(!rect.contains(&Point::new(5.0, 5.0)));
    }
    
    #[test]
    fn test_annotation_layer() {
        let mut layer = AnnotationLayer::new();
        let line = Annotation::Line {
            start: Point::new(0.0, 0.0),
            end: Point::new(10.0, 10.0),
            color: Rgba([255, 0, 0, 255]),
            width: 2.0,
        };
        
        let index = layer.add_annotation(line);
        assert_eq!(index, 0);
        assert_eq!(layer.annotations.len(), 1);
        assert_eq!(layer.selected, Some(0));
    }
}