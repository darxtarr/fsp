// Windows clipboard integration using Win32 APIs
use image::RgbaImage;
use std::ptr;
use windows::{
    Win32::{
        Foundation::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, HANDLE},
        System::DataExchange::{
            OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData, CF_DIB
        },
        Graphics::Gdi::{
            BITMAPINFOHEADER, BITMAPINFO, BI_RGB, RGBQUAD
        },
    },
};

pub type ClipboardResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Copy an RGBA image to the Windows clipboard as a DIB (Device Independent Bitmap)
pub fn copy_to_clipboard(img: &RgbaImage) -> ClipboardResult<()> {
    unsafe {
        // Open clipboard
        if !OpenClipboard(None).as_bool() {
            return Err("Failed to open clipboard".into());
        }
        
        // Clear clipboard
        if !EmptyClipboard().as_bool() {
            CloseClipboard();
            return Err("Failed to empty clipboard".into());
        }
        
        // Create DIB from image
        let dib_data = create_dib_from_image(img)?;
        
        // Allocate global memory
        let h_mem = GlobalAlloc(GMEM_MOVEABLE, dib_data.len())?;
        if h_mem.0 == 0 {
            CloseClipboard();
            return Err("Failed to allocate global memory".into());
        }
        
        // Lock memory and copy data
        let p_mem = GlobalLock(h_mem);
        if p_mem.is_null() {
            CloseClipboard();
            return Err("Failed to lock global memory".into());
        }
        
        ptr::copy_nonoverlapping(dib_data.as_ptr(), p_mem as *mut u8, dib_data.len());
        GlobalUnlock(h_mem);
        
        // Set clipboard data
        let result = SetClipboardData(CF_DIB, HANDLE(h_mem.0));
        
        // Close clipboard
        CloseClipboard();
        
        if result.0 == 0 {
            return Err("Failed to set clipboard data".into());
        }
        
        Ok(())
    }
}

/// Convert RGBA image to DIB format for Windows clipboard
fn create_dib_from_image(img: &RgbaImage) -> ClipboardResult<Vec<u8>> {
    let width = img.width();
    let height = img.height();
    
    // Calculate row padding for DIB (rows must be aligned to 4 bytes)
    let row_size = ((width * 3 + 3) / 4) * 4; // 3 bytes per pixel (RGB), aligned to 4 bytes
    let data_size = row_size * height;
    
    // Create BITMAPINFOHEADER
    let bitmap_info_header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: height as i32, // Positive = bottom-up DIB
        biPlanes: 1,
        biBitCount: 24, // 24-bit RGB
        biCompression: BI_RGB as u32,
        biSizeImage: data_size,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    
    // Create DIB data buffer
    let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
    let total_size = header_size + data_size as usize;
    let mut dib_data = vec![0u8; total_size];
    
    // Copy header
    unsafe {
        ptr::copy_nonoverlapping(
            &bitmap_info_header as *const _ as *const u8,
            dib_data.as_mut_ptr(),
            header_size,
        );
    }
    
    // Convert RGBA to BGR (Windows DIB format) and copy pixel data
    let pixel_data = &mut dib_data[header_size..];
    
    // DIB is stored bottom-up, so we need to flip the rows
    for y in 0..height {
        let src_y = height - 1 - y; // Flip vertically
        let dst_offset = (y * row_size) as usize;
        
        for x in 0..width {
            let src_pixel = img.get_pixel(x, src_y);
            let dst_x = x * 3;
            
            if dst_offset + dst_x as usize + 2 < pixel_data.len() {
                // Convert RGBA to BGR (ignore alpha channel)
                pixel_data[dst_offset + dst_x as usize] = src_pixel[2];     // B
                pixel_data[dst_offset + dst_x as usize + 1] = src_pixel[1]; // G
                pixel_data[dst_offset + dst_x as usize + 2] = src_pixel[0]; // R
            }
        }
        
        // Fill padding bytes with zeros
        let padding_start = dst_offset + (width * 3) as usize;
        let padding_end = dst_offset + row_size as usize;
        for i in padding_start..padding_end.min(pixel_data.len()) {
            pixel_data[i] = 0;
        }
    }
    
    Ok(dib_data)
}

/// Copy image file to clipboard
pub fn copy_file_to_clipboard(image_path: &std::path::Path) -> ClipboardResult<()> {
    let img = image::open(image_path)?.to_rgba8();
    copy_to_clipboard(&img)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    
    #[test]
    fn test_create_dib_from_image() {
        // Create a simple 2x2 test image
        let img: RgbaImage = ImageBuffer::from_fn(2, 2, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 0, 0, 255]) // Red
            } else {
                Rgba([0, 255, 0, 255]) // Green
            }
        });
        
        let dib_data = create_dib_from_image(&img).expect("Failed to create DIB");
        
        // Check that we have header + pixel data
        let expected_header_size = std::mem::size_of::<BITMAPINFOHEADER>();
        assert!(dib_data.len() > expected_header_size);
        
        // Check header values
        let header = unsafe {
            &*(dib_data.as_ptr() as *const BITMAPINFOHEADER)
        };
        assert_eq!(header.biWidth, 2);
        assert_eq!(header.biHeight, 2);
        assert_eq!(header.biBitCount, 24);
    }
    
    #[test]
    fn test_empty_image() {
        // Test with minimal 1x1 image
        let img: RgbaImage = ImageBuffer::from_fn(1, 1, |_, _| {
            Rgba([128, 128, 128, 255]) // Gray
        });
        
        let dib_data = create_dib_from_image(&img).expect("Failed to create DIB");
        assert!(dib_data.len() > 0);
    }
}