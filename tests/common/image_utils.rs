//! Image comparison utilities for testing.
//!
//! This module provides helper functions for comparing and verifying images in tests.

use image::{DynamicImage, GenericImageView, ImageFormat, ImageError};
use std::path::Path;

/// Maximum pixel difference for image comparison
pub const DEFAULT_PIXEL_DIFF: u8 = 1;

/// Load an image from a file
pub fn load_image(path: &Path) -> Result<DynamicImage, ImageError> {
    image::open(path)
}

/// Load an image from a byte array
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    image::load_from_memory(bytes)
}

/// Detect image format from bytes
pub fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

/// Compare two images for approximate equality
///
/// # Arguments
///
/// * `actual` - The actual image
/// * `expected` - The expected image
/// * `max_diff` - The maximum allowed difference per pixel component (default: 1)
///
/// # Returns
///
/// * `Ok(())` if the images are approximately equal
/// * `Err(String)` with an error message if they differ
pub fn assert_images_approx_eq(
    actual: &DynamicImage,
    expected: &DynamicImage,
    max_diff: Option<u8>,
) -> Result<(), String> {
    // Check dimensions
    let (actual_width, actual_height) = actual.dimensions();
    let (expected_width, expected_height) = expected.dimensions();
    
    if actual_width != expected_width || actual_height != expected_height {
        return Err(format!(
            "Image dimensions differ: actual = {}x{}, expected = {}x{}",
            actual_width, actual_height, expected_width, expected_height
        ));
    }
    
    let max_diff = max_diff.unwrap_or(DEFAULT_PIXEL_DIFF);
    
    // Check pixel values
    let mut diff_count = 0;
    let mut max_observed_diff = 0u8;
    
    for y in 0..actual_height {
        for x in 0..actual_width {
            let actual_pixel = actual.get_pixel(x, y);
            let expected_pixel = expected.get_pixel(x, y);
            
            for (_i, (a, e)) in actual_pixel.0.iter().zip(expected_pixel.0.iter()).enumerate() {
                let diff = (*a as i16 - *e as i16).abs() as u8;
                if diff > max_diff {
                    diff_count += 1;
                    max_observed_diff = max_observed_diff.max(diff);
                    
                    // Early return if we have too many differences
                    if diff_count > 10 {
                        return Err(format!(
                            "Images differ by more than 10 pixels, max observed diff = {}",
                            max_observed_diff
                        ));
                    }
                }
            }
        }
    }
    
    if diff_count > 0 {
        return Err(format!(
            "Images differ by {} pixels, max observed diff = {}",
            diff_count, max_observed_diff
        ));
    }
    
    Ok(())
}

/// Check if an image has the expected dimensions
///
/// # Arguments
///
/// * `image` - The image to check
/// * `expected_width` - The expected width
/// * `expected_height` - The expected height
///
/// # Returns
///
/// * `Ok(())` if the image has the expected dimensions
/// * `Err(String)` with an error message if the dimensions differ
pub fn assert_image_dimensions(
    image: &DynamicImage,
    expected_width: u32,
    expected_height: u32,
) -> Result<(), String> {
    let (actual_width, actual_height) = image.dimensions();
    
    if actual_width != expected_width || actual_height != expected_height {
        return Err(format!(
            "Image dimensions differ: actual = {}x{}, expected = {}x{}",
            actual_width, actual_height, expected_width, expected_height
        ));
    }
    
    Ok(())
}

/// Check if an image has the expected format
///
/// # Arguments
///
/// * `bytes` - The image bytes
/// * `expected_format` - The expected image format
///
/// # Returns
///
/// * `Ok(())` if the image has the expected format
/// * `Err(String)` with an error message if the format differs
pub fn assert_image_format(
    bytes: &[u8],
    expected_format: ImageFormat,
) -> Result<(), String> {
    let actual_format = detect_image_format(bytes)
        .ok_or_else(|| "Could not detect image format".to_string())?;
    
    if actual_format != expected_format {
        return Err(format!(
            "Image format differs: actual = {:?}, expected = {:?}",
            actual_format, expected_format
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    
    #[test]
    fn test_detect_image_format() {
        // Create a simple PNG image
        let img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(2, 2);
        let mut png_bytes = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png_bytes), ImageFormat::Png)
            .unwrap();
        
        // Test format detection
        let format = detect_image_format(&png_bytes).unwrap();
        assert_eq!(format, ImageFormat::Png);
    }
    
    #[test]
    fn test_assert_image_dimensions() {
        let img = DynamicImage::new_rgb8(10, 20);
        
        // This should pass
        assert!(assert_image_dimensions(&img, 10, 20).is_ok());
        
        // These should fail
        assert!(assert_image_dimensions(&img, 11, 20).is_err());
        assert!(assert_image_dimensions(&img, 10, 21).is_err());
    }
    
    #[test]
    fn test_assert_images_approx_eq() {
        // Create two identical images
        let mut img1 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(3, 3);
        let mut img2 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(3, 3);
        
        // Fill with identical data
        for (x, y, pixel) in img1.enumerate_pixels_mut() {
            *pixel = Rgba([x as u8, y as u8, 100, 255]);
        }
        
        for (x, y, pixel) in img2.enumerate_pixels_mut() {
            *pixel = Rgba([x as u8, y as u8, 100, 255]);
        }
        
        let dyn_img1 = DynamicImage::ImageRgba8(img1);
        let dyn_img2 = DynamicImage::ImageRgba8(img2);
        
        // This should pass (identical images)
        assert!(assert_images_approx_eq(&dyn_img1, &dyn_img2, None).is_ok());
        
        // Modify one pixel slightly in img2
        let mut img3 = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(3, 3);
        for (x, y, pixel) in img3.enumerate_pixels_mut() {
            if x == 1 && y == 1 {
                *pixel = Rgba([x as u8, y as u8, 101, 255]); // Small difference
            } else {
                *pixel = Rgba([x as u8, y as u8, 100, 255]);
            }
        }
        
        let dyn_img3 = DynamicImage::ImageRgba8(img3);
        
        // This should pass with default tolerance (diff = 1)
        assert!(assert_images_approx_eq(&dyn_img1, &dyn_img3, None).is_ok());
        
        // This should fail with stricter tolerance
        assert!(assert_images_approx_eq(&dyn_img1, &dyn_img3, Some(0)).is_err());
    }
}
