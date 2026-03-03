// Windows clipboard integration using Win32 APIs
use image::RgbaImage;
use std::ptr;
use windows::{
    Win32::{
        Foundation::{HANDLE, HGLOBAL},
        System::{
            DataExchange::{OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData},
            Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
        },
        Graphics::Gdi::{BITMAPINFOHEADER, BI_RGB},
    },
};

pub type ClipboardResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Copy an RGBA image to the Windows clipboard as a DIB (Device Independent Bitmap)
pub fn copy_to_clipboard(img: &RgbaImage) -> ClipboardResult<()> {
    unsafe {
        OpenClipboard(None)?;

        if let Err(e) = empty_and_set_clipboard(img) {
            let _ = CloseClipboard();
            return Err(e);
        }

        let _ = CloseClipboard();
        Ok(())
    }
}

unsafe fn empty_and_set_clipboard(img: &RgbaImage) -> ClipboardResult<()> {
    EmptyClipboard()?;

    let dib_data = create_dib_from_image(img)?;

    let h_mem: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, dib_data.len())?;

    let p_mem = GlobalLock(h_mem);
    if p_mem.is_null() {
        return Err("Failed to lock global memory".into());
    }

    ptr::copy_nonoverlapping(dib_data.as_ptr(), p_mem as *mut u8, dib_data.len());
    let _ = GlobalUnlock(h_mem);

    // CF_DIB = 8
    SetClipboardData(8u32, Some(HANDLE(h_mem.0)))?;

    Ok(())
}

/// Convert RGBA image to DIB format for Windows clipboard
fn create_dib_from_image(img: &RgbaImage) -> ClipboardResult<Vec<u8>> {
    let width = img.width();
    let height = img.height();

    let row_size = ((width * 3 + 3) / 4) * 4;
    let data_size = row_size * height;

    let bitmap_info_header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: height as i32, // Positive = bottom-up DIB
        biPlanes: 1,
        biBitCount: 24,
        biCompression: BI_RGB.0,
        biSizeImage: data_size,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };

    let header_size = std::mem::size_of::<BITMAPINFOHEADER>();
    let total_size = header_size + data_size as usize;
    let mut dib_data = vec![0u8; total_size];

    unsafe {
        ptr::copy_nonoverlapping(
            &bitmap_info_header as *const _ as *const u8,
            dib_data.as_mut_ptr(),
            header_size,
        );
    }

    let pixel_data = &mut dib_data[header_size..];

    for y in 0..height {
        let src_y = height - 1 - y;
        let dst_offset = (y * row_size) as usize;

        for x in 0..width {
            let src_pixel = img.get_pixel(x, src_y);
            let dst_x = x * 3;

            if dst_offset + dst_x as usize + 2 < pixel_data.len() {
                pixel_data[dst_offset + dst_x as usize] = src_pixel[2];     // B
                pixel_data[dst_offset + dst_x as usize + 1] = src_pixel[1]; // G
                pixel_data[dst_offset + dst_x as usize + 2] = src_pixel[0]; // R
            }
        }

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
        let img: RgbaImage = ImageBuffer::from_fn(2, 2, |x, y| {
            if (x + y) % 2 == 0 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 255, 0, 255])
            }
        });

        let dib_data = create_dib_from_image(&img).expect("Failed to create DIB");

        let expected_header_size = std::mem::size_of::<BITMAPINFOHEADER>();
        assert!(dib_data.len() > expected_header_size);

        let header = unsafe {
            &*(dib_data.as_ptr() as *const BITMAPINFOHEADER)
        };
        assert_eq!(header.biWidth, 2);
        assert_eq!(header.biHeight, 2);
        assert_eq!(header.biBitCount, 24);
    }

    #[test]
    fn test_empty_image() {
        let img: RgbaImage = ImageBuffer::from_fn(1, 1, |_, _| {
            Rgba([128, 128, 128, 255])
        });

        let dib_data = create_dib_from_image(&img).expect("Failed to create DIB");
        assert!(dib_data.len() > 0);
    }
}
