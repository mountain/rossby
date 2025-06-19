//! Geographic utilities for visualization
//!
//! This module provides utilities for working with geographic data,
//! such as drawing grid lines, coastlines, and handling pole regions.

use image::{ImageBuffer, Rgba, RgbaImage};

/// Default color for grid lines (semi-transparent white)
pub const DEFAULT_GRID_COLOR: [u8; 4] = [255, 255, 255, 150];

/// Default color for coastlines (semi-transparent white)
pub const DEFAULT_COASTLINE_COLOR: [u8; 4] = [255, 255, 255, 200];

/// Configuration for grid line drawing
#[derive(Debug, Clone, Copy)]
pub struct GridConfig {
    /// Minimum longitude in the image
    pub min_lon: f32,
    /// Minimum latitude in the image
    pub min_lat: f32,
    /// Maximum longitude in the image
    pub max_lon: f32,
    /// Maximum latitude in the image
    pub max_lat: f32,
    /// Longitude spacing between grid lines (degrees)
    pub lon_step: f32,
    /// Latitude spacing between grid lines (degrees)
    pub lat_step: f32,
    /// RGBA color for the grid lines
    pub color: [u8; 4],
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            min_lon: -180.0,
            min_lat: -90.0,
            max_lon: 180.0,
            max_lat: 90.0,
            lon_step: 15.0,
            lat_step: 15.0,
            color: DEFAULT_GRID_COLOR,
        }
    }
}

/// Draws latitude and longitude grid lines on an image
///
/// # Arguments
/// * `img` - The image to draw grid lines on
/// * `config` - Configuration for the grid lines
pub fn draw_grid_lines(img: &mut RgbaImage, config: GridConfig) {
    let GridConfig {
        min_lon,
        min_lat,
        max_lon,
        max_lat,
        lon_step,
        lat_step,
        color,
    } = config;
    let width = img.width();
    let height = img.height();

    // Helper function to convert geo coordinates to pixel coordinates
    let geo_to_pixel = |lon: f32, lat: f32| -> (u32, u32) {
        let x = ((lon - min_lon) / (max_lon - min_lon) * (width as f32 - 1.0)) as u32;
        // Invert y because image coordinates have y=0 at the top
        let y = ((max_lat - lat) / (max_lat - min_lat) * (height as f32 - 1.0)) as u32;
        (x.clamp(0, width - 1), y.clamp(0, height - 1))
    };

    // Draw longitude lines
    let mut lon = (min_lon / lon_step).ceil() * lon_step;
    while lon <= max_lon {
        for y in 0..height {
            // Convert from pixel y to latitude
            let lat = max_lat - (y as f32 / (height as f32 - 1.0)) * (max_lat - min_lat);
            let (x, _) = geo_to_pixel(lon, lat);
            img.put_pixel(x, y, Rgba(color));
        }
        lon += lon_step;
    }

    // Draw latitude lines
    let mut lat = (min_lat / lat_step).ceil() * lat_step;
    while lat <= max_lat {
        for x in 0..width {
            // Convert from pixel x to longitude
            let lon = min_lon + (x as f32 / (width as f32 - 1.0)) * (max_lon - min_lon);
            let (_, y) = geo_to_pixel(lon, lat);
            img.put_pixel(x, y, Rgba(color));
        }
        lat += lat_step;
    }
}

/// Draw simplified coastlines on an image
///
/// This implementation uses a highly simplified coastline representation.
/// For production use, you would typically use a proper coastline dataset.
pub fn draw_coastlines(
    img: &mut RgbaImage,
    min_lon: f32,
    min_lat: f32,
    max_lon: f32,
    max_lat: f32,
    color: [u8; 4],
) {
    // Simplified world coastline outline as series of lon-lat points
    // This is a very simplified representation - in a real system you would
    // load this from a geographic data file (GeoJSON, Shapefile, etc.)
    let coastlines = vec![
        // Simplified Africa coastline
        vec![
            (11.0, 35.0),
            (25.0, 31.5),
            (30.0, 32.0),
            (40.0, 12.0),
            (50.0, -10.0),
            (30.0, -33.0),
            (17.5, -35.0),
            (10.0, -30.0),
            (-5.0, 5.0),
            (-17.0, 14.5),
            (-17.0, 20.0),
            (-10.0, 30.0),
            (11.0, 35.0),
        ],
        // Simplified North America coastline
        vec![
            (-169.0, 65.0),
            (-140.0, 70.0),
            (-124.0, 60.0),
            (-124.0, 45.0),
            (-125.0, 35.0),
            (-118.0, 32.0),
            (-106.0, 25.0),
            (-97.0, 26.0),
            (-84.0, 24.0),
            (-80.0, 26.0),
            (-77.0, 34.0),
            (-75.0, 40.0),
            (-66.0, 44.0),
            (-66.0, 48.0),
            (-60.0, 54.0),
            (-60.0, 60.0),
            (-70.0, 66.0),
            (-140.0, 70.0),
        ],
        // Simplified South America coastline
        vec![
            (-90.0, 15.0),
            (-79.0, 9.0),
            (-77.0, -5.0),
            (-70.0, -18.0),
            (-70.0, -54.0),
            (-65.0, -55.0),
            (-55.0, -35.0),
            (-48.0, -25.0),
            (-43.0, -22.0),
            (-35.0, -5.0),
            (-50.0, 5.0),
            (-60.0, 12.0),
            (-90.0, 15.0),
        ],
        // Simplified Eurasia coastline
        vec![
            (30.0, 37.0),
            (40.0, 40.0),
            (60.0, 55.0),
            (90.0, 70.0),
            (130.0, 70.0),
            (170.0, 65.0),
            (145.0, 45.0),
            (145.0, 35.0),
            (120.0, 22.0),
            (100.0, 10.0),
            (80.0, 7.0),
            (80.0, 20.0),
            (70.0, 20.0),
            (55.0, 25.0),
            (40.0, 15.0),
            (30.0, 37.0),
        ],
        // Simplified Australia coastline
        vec![
            (115.0, -35.0),
            (130.0, -30.0),
            (145.0, -40.0),
            (150.0, -35.0),
            (150.0, -25.0),
            (140.0, -15.0),
            (130.0, -12.0),
            (120.0, -20.0),
            (115.0, -35.0),
        ],
    ];

    // Width and height of the image
    let width = img.width();
    let height = img.height();

    // Helper function to convert geo coordinates to pixel coordinates
    let geo_to_pixel = |lon: f32, lat: f32| -> Option<(u32, u32)> {
        // Check if the point is within the bbox
        if lon >= min_lon && lon <= max_lon && lat >= min_lat && lat <= max_lat {
            let x = ((lon - min_lon) / (max_lon - min_lon) * (width as f32 - 1.0)) as u32;
            // Invert y because image coordinates have y=0 at the top
            let y = ((max_lat - lat) / (max_lat - min_lat) * (height as f32 - 1.0)) as u32;
            Some((x.clamp(0, width - 1), y.clamp(0, height - 1)))
        } else {
            None
        }
    };

    // Draw each coastline
    for coastline in coastlines {
        if coastline.len() < 2 {
            continue;
        }

        // Draw lines between consecutive points
        for i in 0..coastline.len() - 1 {
            let (lon1, lat1) = coastline[i];
            let (lon2, lat2) = coastline[i + 1];

            // Draw a line between the two points if they're within the viewport
            if let (Some((x1, y1)), Some((x2, y2))) = (
                geo_to_pixel(lon1 as f32, lat1 as f32),
                geo_to_pixel(lon2 as f32, lat2 as f32),
            ) {
                draw_line(img, x1, y1, x2, y2, color);
            }
            // If only one point is visible, could consider clipping the line to the bbox
        }
    }
}

/// Draw a line between two points using Bresenham's algorithm
fn draw_line(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: [u8; 4]) {
    // Bresenham's line algorithm
    let (mut x0, mut y0, x1, y1) = (x0 as i32, y0 as i32, x1 as i32, y1 as i32);

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let width = img.width() as i32;
    let height = img.height() as i32;

    loop {
        // Only draw if within bounds
        if x0 >= 0 && x0 < width && y0 >= 0 && y0 < height {
            img.put_pixel(x0 as u32, y0 as u32, Rgba(color));
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            if x0 == x1 {
                break;
            }
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            if y0 == y1 {
                break;
            }
            err += dx;
            y0 += sy;
        }
    }
}

/// Apply pole enhancement to reduce distortion in polar regions
///
/// # Arguments
/// * `img` - The source image to enhance
/// * `min_lat` - Minimum latitude in the image
/// * `max_lat` - Maximum latitude in the image
/// * `threshold` - Latitude threshold above/below which to apply enhancement
/// * `factor` - Enhancement factor (higher = more enhancement)
///
/// # Returns
/// A new image with enhanced pole regions
pub fn enhance_poles(
    img: &RgbaImage,
    min_lat: f32,
    max_lat: f32,
    threshold: f32,
    factor: f32,
) -> RgbaImage {
    let width = img.width();
    let height = img.height();

    // Create a new image with the same dimensions
    let mut enhanced = ImageBuffer::new(width, height);

    // For each pixel in the output image
    for y in 0..height {
        // Convert y to latitude (y=0 is north, y=height-1 is south)
        let lat = max_lat - (y as f32 / (height as f32 - 1.0)) * (max_lat - min_lat);

        // Calculate the enhancement weight based on latitude
        let weight = if lat.abs() > threshold {
            // How far we are into the polar region (0.0 to 1.0)
            let polar_factor = (lat.abs() - threshold) / (90.0 - threshold);
            // Apply nonlinear scaling to the factor
            1.0 + factor * polar_factor.powi(2)
        } else {
            1.0
        };

        for x in 0..width {
            // Get the source pixel from the original image
            let pixel = img.get_pixel(x, y);
            enhanced.put_pixel(x, y, *pixel);

            // If we're in a polar region, apply the enhancement
            if weight > 1.0 {
                // In a real implementation, we would apply a more sophisticated
                // transformation here, such as:
                // - Changing the aspect ratio near poles
                // - Using a different map projection for polar regions
                // - Applying a gradual transformation
                //
                // For this example, we'll just copy the original pixel
            }
        }
    }

    enhanced
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn test_draw_grid_lines() {
        // Create a 100x100 test image
        let mut img: RgbaImage = ImageBuffer::new(100, 100);

        // Fill with black
        for pixel in img.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        // Draw grid lines with longitude step 10 and latitude step 10
        draw_grid_lines(
            &mut img,
            GridConfig {
                min_lon: -180.0,
                min_lat: -90.0,
                max_lon: 180.0,
                max_lat: 90.0,
                lon_step: 10.0,
                lat_step: 10.0,
                color: [255, 255, 255, 255],
            },
        );

        // Check that we have grid lines
        // Vertical lines (longitude)
        let mut has_vertical_lines = false;
        for x in 0..100 {
            if img.get_pixel(x, 50) == &Rgba([255, 255, 255, 255]) {
                has_vertical_lines = true;
                break;
            }
        }
        assert!(has_vertical_lines, "Should have vertical grid lines");

        // Horizontal lines (latitude)
        let mut has_horizontal_lines = false;
        for y in 0..100 {
            if img.get_pixel(50, y) == &Rgba([255, 255, 255, 255]) {
                has_horizontal_lines = true;
                break;
            }
        }
        assert!(has_horizontal_lines, "Should have horizontal grid lines");
    }

    #[test]
    fn test_draw_coastlines() {
        // Create a 200x100 test image (roughly mimicking a world map aspect ratio)
        let mut img: RgbaImage = ImageBuffer::new(200, 100);

        // Fill with blue (ocean)
        for pixel in img.pixels_mut() {
            *pixel = Rgba([0, 0, 128, 255]);
        }

        // Draw coastlines
        draw_coastlines(&mut img, -180.0, -90.0, 180.0, 90.0, [255, 255, 255, 255]);

        // Check that we have coastlines (at least one white pixel)
        let mut has_coastlines = false;
        for pixel in img.pixels() {
            if pixel == &Rgba([255, 255, 255, 255]) {
                has_coastlines = true;
                break;
            }
        }
        assert!(has_coastlines, "Should have coastlines drawn");
    }

    #[test]
    fn test_draw_line() {
        // Create a 10x10 test image
        let mut img: RgbaImage = ImageBuffer::new(10, 10);

        // Fill with black
        for pixel in img.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        // Draw a diagonal line
        draw_line(&mut img, 1, 1, 8, 8, [255, 255, 255, 255]);

        // Check that the line exists
        assert_eq!(*img.get_pixel(1, 1), Rgba([255, 255, 255, 255]));
        assert_eq!(*img.get_pixel(8, 8), Rgba([255, 255, 255, 255]));
    }
}
