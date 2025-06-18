//! Image generation endpoint handler.
//!
//! Returns a PNG/JPEG image rendering of a variable over a specified region and time.

use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use image::{ImageBuffer, RgbaImage};
use ndarray::ArrayView2;
use serde::Deserialize;
use std::io::Cursor;
use std::sync::Arc;

use crate::colormaps::{self, Colormap};
use crate::error::{Result, RossbyError};
use crate::state::AppState;

/// Default image dimensions
const DEFAULT_WIDTH: u32 = 800;
const DEFAULT_HEIGHT: u32 = 600;

/// Default colormap
const DEFAULT_COLORMAP: &str = "viridis";

/// Default output format
const DEFAULT_FORMAT: &str = "png";

/// Query parameters for image endpoint
#[derive(Debug, Deserialize)]
pub struct ImageQuery {
    /// Variable name to render
    pub var: String,
    /// Time index (0-based)
    pub time_index: Option<usize>,
    /// Bounding box as "min_lon,min_lat,max_lon,max_lat"
    pub bbox: Option<String>,
    /// Image width in pixels
    pub width: Option<u32>,
    /// Image height in pixels
    pub height: Option<u32>,
    /// Colormap name (e.g., viridis, plasma, coolwarm)
    pub colormap: Option<String>,
    /// Interpolation method for resampling
    pub interpolation: Option<String>,
    /// Output format (png or jpeg)
    pub format: Option<String>,
    /// Map centering (eurocentric, americas, pacific, or custom longitude)
    pub center: Option<String>,
    /// Allow bounding boxes that cross the dateline/prime meridian
    pub wrap_longitude: Option<bool>,
    /// Upsampling/downsampling quality (auto, nearest, bilinear, bicubic)
    pub resampling: Option<String>,
}

/// Parse bounding box string into values
///
/// Format: "min_lon,min_lat,max_lon,max_lat"
fn parse_bbox(bbox_str: &str, wrap_longitude: bool) -> Result<(f32, f32, f32, f32)> {
    let parts: Vec<&str> = bbox_str.split(',').collect();

    if parts.len() != 4 {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Bounding box must be in format 'min_lon,min_lat,max_lon,max_lat'".to_string(),
        });
    }

    let min_lon = parts[0]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Could not parse min_lon as a float".to_string(),
        })?;

    let min_lat = parts[1]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Could not parse min_lat as a float".to_string(),
        })?;

    let max_lon = parts[2]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Could not parse max_lon as a float".to_string(),
        })?;

    let max_lat = parts[3]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Could not parse max_lat as a float".to_string(),
        })?;

    // Allow bounding boxes that cross the dateline if wrap_longitude is true
    if min_lon >= max_lon && !wrap_longitude {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Bounding box min_lon must be less than max_lon (or use wrap_longitude=true)"
                .to_string(),
        });
    }

    if min_lat >= max_lat {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Bounding box min_lat must be less than max_lat".to_string(),
        });
    }

    Ok((min_lon, min_lat, max_lon, max_lat))
}

/// Normalize longitude to the range based on the map center
///
/// - For Eurocentric view: -180 to 180
/// - For Americas view: -90 to 270
/// - For Pacific view: 0 to 360
/// - For custom center: center-180 to center+180
fn normalize_longitude(lon: f32, center: &str) -> f32 {
    // Special case for exact 180 degrees in eurocentric mode
    if center == "eurocentric" && (lon == 180.0 || lon == -180.0) {
        return 180.0;
    }

    // Define the min longitude for each center option
    let (min_lon, max_lon) = match center {
        "eurocentric" => (-180.0, 180.0),
        "americas" => (-90.0, 270.0),
        "pacific" => (0.0, 360.0),
        custom => {
            let center_val = custom.parse::<f32>().unwrap_or(0.0);
            (center_val - 180.0, center_val + 180.0)
        }
    };

    // Normalize to the defined range
    let mut normalized = lon;

    // Adjust the longitude to be within the range
    while normalized < min_lon {
        normalized += 360.0;
    }

    while normalized >= max_lon {
        normalized -= 360.0;
    }

    normalized
}

/// Adjust bounding box for the selected map centering
///
/// This function normalizes the min and max longitudes based on the selected map center
/// and handles cases where the bounding box crosses the dateline.
fn adjust_bbox_for_center(
    min_lon: f32,
    min_lat: f32,
    max_lon: f32,
    max_lat: f32,
    center: &str,
    wrap_longitude: bool,
) -> (f32, f32, f32, f32) {
    if !wrap_longitude {
        // Simple case - no wrapping needed
        return (
            normalize_longitude(min_lon, center),
            min_lat,
            normalize_longitude(max_lon, center),
            max_lat,
        );
    }

    // Handle dateline crossing
    let normalized_min = normalize_longitude(min_lon, center);
    let normalized_max = normalize_longitude(max_lon, center);

    // If after normalization min > max, we're crossing the dateline
    if normalized_min > normalized_max {
        // For data fetching, we'll need to fetch two separate regions
        // But for now, we're just adjusting the bbox to the full longitude range
        // to ensure we get all necessary data
        let center_lon = match center {
            "eurocentric" => 0.0,
            "americas" => -90.0,
            "pacific" => -180.0,
            custom => custom.parse::<f32>().unwrap_or(0.0),
        };

        return (
            center_lon - 180.0, // Minimum possible longitude
            min_lat,
            center_lon + 180.0, // Maximum possible longitude
            max_lat,
        );
    }

    (normalized_min, min_lat, normalized_max, max_lat)
}

/// Generate an image from 2D data array using specified colormap and interpolation method
fn generate_image(
    data: ArrayView2<f32>,
    width: u32,
    height: u32,
    colormap: &dyn Colormap,
    resampling: &str,
) -> Result<RgbaImage> {
    // Find min/max values for normalization
    let mut min_val = f32::INFINITY;
    let mut max_val = f32::NEG_INFINITY;

    for &val in data.iter() {
        if val.is_finite() {
            min_val = min_val.min(val);
            max_val = max_val.max(val);
        }
    }

    // Create a new image buffer
    let mut img = ImageBuffer::new(width, height);

    // Get the interpolator based on the resampling method
    let interpolator = match resampling {
        "nearest" => crate::interpolation::get_interpolator("nearest")?,
        "bilinear" => crate::interpolation::get_interpolator("bilinear")?,
        "bicubic" => crate::interpolation::get_interpolator("bicubic")?,
        // Default to bilinear for "auto" or any other value
        _ => crate::interpolation::get_interpolator("bilinear")?,
    };

    let data_height = data.shape()[0];
    let data_width = data.shape()[1];

    // Flatten the 2D array for the interpolator
    let flat_data: Vec<f32> = data.iter().cloned().collect();
    let shape = vec![data_height, data_width];

    for y in 0..height {
        for x in 0..width {
            // Map image coordinates to data coordinates (fractional indices)
            let data_x = x as f64 * (data_width - 1) as f64 / (width - 1) as f64;
            let data_y = y as f64 * (data_height - 1) as f64 / (height - 1) as f64;

            // Perform interpolation to get the value at this pixel
            let indices = vec![data_y, data_x];
            let data_value = match interpolator.interpolate(&flat_data, &shape, &indices) {
                Ok(val) => val,
                Err(_) => f32::NAN, // Use NaN for interpolation errors
            };

            // Map value to color
            let color = if data_value.is_finite() {
                colormap.map(data_value, min_val, max_val)
            } else {
                // Use transparent black for NaN/missing values
                [0, 0, 0, 0]
            };

            // Set pixel color
            img.put_pixel(x, y, image::Rgba(color));
        }
    }

    Ok(img)
}

/// Handle GET /image requests
pub async fn image_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ImageQuery>,
) -> Response {
    // Process the request
    match generate_image_response(state, params) {
        Ok(response) => response,
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
            .into_response(),
    }
}

/// Helper function to generate image response
fn generate_image_response(state: Arc<AppState>, params: ImageQuery) -> Result<Response> {
    // Get variable name from query
    let var_name = params.var;

    // Verify variable exists
    if !state.has_variable(&var_name) {
        return Err(RossbyError::VariableNotFound { name: var_name });
    }

    // Get time index (default to 0)
    let time_index = params.time_index.unwrap_or(0);

    // Check time index is in bounds
    if time_index >= state.time_dim_size() {
        return Err(RossbyError::IndexOutOfBounds {
            param: "time_index".to_string(),
            value: time_index.to_string(),
            max: state.time_dim_size() - 1,
        });
    }

    // Get map centering (default to eurocentric)
    let center = params.center.as_deref().unwrap_or("eurocentric");

    // Get longitude wrapping setting (default to false)
    let wrap_longitude = params.wrap_longitude.unwrap_or(false);

    // Parse bounding box (if provided)
    let (min_lon, min_lat, max_lon, max_lat) = if let Some(ref bbox) = params.bbox {
        parse_bbox(bbox, wrap_longitude)?
    } else {
        // Use full domain if no bbox specified
        state.get_lat_lon_bounds()?
    };

    // Adjust bounding box for the selected map center
    let (adj_min_lon, adj_min_lat, adj_max_lon, adj_max_lat) =
        adjust_bbox_for_center(min_lon, min_lat, max_lon, max_lat, center, wrap_longitude);

    // Get image dimensions
    let width = params.width.unwrap_or(DEFAULT_WIDTH);
    let height = params.height.unwrap_or(DEFAULT_HEIGHT);

    // Get colormap
    let colormap_name = params.colormap.as_deref().unwrap_or(DEFAULT_COLORMAP);
    let colormap = colormaps::get_colormap(colormap_name)?;

    // Get resampling method (default to auto)
    let resampling = params.resampling.as_deref().unwrap_or("auto");

    // Get output format
    let format = params
        .format
        .as_deref()
        .unwrap_or(DEFAULT_FORMAT)
        .to_lowercase();
    if format != "png" && format != "jpeg" {
        return Err(RossbyError::InvalidParameter {
            param: "format".to_string(),
            message: "Format must be 'png' or 'jpeg'".to_string(),
        });
    }

    // Get data for the specified time slice with adjusted coordinates
    let data = state.get_data_slice(
        &var_name,
        time_index,
        adj_min_lon,
        adj_min_lat,
        adj_max_lon,
        adj_max_lat,
    )?;

    // Generate the image with the specified interpolation method
    let img = generate_image(data.view(), width, height, colormap.as_ref(), resampling)?;

    // Encode the image to the specified format
    let mut buffer = Cursor::new(Vec::new());

    match format.as_str() {
        "png" => {
            img.write_to(&mut buffer, image::ImageFormat::Png)
                .map_err(|e| RossbyError::ImageGeneration {
                    message: format!("Failed to encode PNG: {}", e),
                })?;
        }
        "jpeg" => {
            img.write_to(&mut buffer, image::ImageFormat::Jpeg)
                .map_err(|e| RossbyError::ImageGeneration {
                    message: format!("Failed to encode JPEG: {}", e),
                })?;
        }
        _ => unreachable!(), // We've already validated the format
    }

    // Set appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        match format.as_str() {
            "png" => "image/png",
            "jpeg" => "image/jpeg",
            _ => unreachable!(),
        }
        .parse()
        .unwrap(),
    );

    // Return the image
    Ok((StatusCode::OK, headers, buffer.into_inner()).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bbox() {
        // Valid bbox - without wrapping
        let result = parse_bbox("10.5,20.5,30.5,40.5", false);
        assert!(result.is_ok());
        let (min_lon, min_lat, max_lon, max_lat) = result.unwrap();
        assert_eq!(min_lon, 10.5);
        assert_eq!(min_lat, 20.5);
        assert_eq!(max_lon, 30.5);
        assert_eq!(max_lat, 40.5);

        // Invalid format
        assert!(parse_bbox("10.5,20.5,30.5", false).is_err());

        // Invalid numbers
        assert!(parse_bbox("10.5,20.5,not_a_number,40.5", false).is_err());

        // Invalid bounds without wrapping
        assert!(parse_bbox("30.5,20.5,10.5,40.5", false).is_err()); // min_lon > max_lon
        assert!(parse_bbox("10.5,40.5,30.5,20.5", false).is_err()); // min_lat > max_lat

        // Crossing the dateline with wrapping enabled
        let result = parse_bbox("170.0,20.5,-170.0,40.5", true);
        assert!(result.is_ok());
        let (min_lon, _min_lat, max_lon, _max_lat) = result.unwrap();
        assert_eq!(min_lon, 170.0);
        assert_eq!(max_lon, -170.0); // This is valid with wrapping

        // Invalid latitude even with wrapping
        assert!(parse_bbox("170.0,40.5,-170.0,20.5", true).is_err()); // min_lat > max_lat
    }

    #[test]
    fn test_normalize_longitude() {
        // Eurocentric normalization (-180 to 180)
        assert_eq!(normalize_longitude(185.0, "eurocentric"), -175.0);
        assert_eq!(normalize_longitude(-185.0, "eurocentric"), 175.0);
        assert_eq!(normalize_longitude(0.0, "eurocentric"), 0.0);
        assert_eq!(normalize_longitude(180.0, "eurocentric"), 180.0);
        assert_eq!(normalize_longitude(-180.0, "eurocentric"), 180.0); // Special case: -180 becomes 180 in eurocentric

        // Americas-centered normalization (-90 to 270)
        assert_eq!(normalize_longitude(275.0, "americas"), -85.0); // 275 is outside range, so normalized to -85
        assert_eq!(normalize_longitude(-95.0, "americas"), 265.0);
        assert_eq!(normalize_longitude(-90.0, "americas"), -90.0);
        assert_eq!(normalize_longitude(90.0, "americas"), 90.0);

        // Pacific-centered normalization (0 to 360)
        assert_eq!(normalize_longitude(-10.0, "pacific"), 350.0);
        assert_eq!(normalize_longitude(370.0, "pacific"), 10.0);
        assert_eq!(normalize_longitude(0.0, "pacific"), 0.0);
        assert_eq!(normalize_longitude(360.0, "pacific"), 0.0);

        // Custom center (e.g., 90E)
        assert_eq!(normalize_longitude(280.0, "90"), -80.0); // 280 is outside range of -90 to 270, so normalized to -80
        assert_eq!(normalize_longitude(-100.0, "90"), 260.0); // -100 is outside range of -90 to 270, so normalized to 260
    }

    #[test]
    fn test_adjust_bbox_for_center() {
        // Simple case - no wrapping
        let (min_lon, min_lat, max_lon, max_lat) =
            adjust_bbox_for_center(10.0, 20.0, 30.0, 40.0, "eurocentric", false);
        assert_eq!(min_lon, 10.0);
        assert_eq!(min_lat, 20.0);
        assert_eq!(max_lon, 30.0);
        assert_eq!(max_lat, 40.0);

        // Wrapping case - crossing the dateline
        let (min_lon, min_lat, max_lon, max_lat) =
            adjust_bbox_for_center(170.0, 20.0, -170.0, 40.0, "eurocentric", true);
        // Should expand to full longitude range in eurocentric view
        assert_eq!(min_lon, -180.0);
        assert_eq!(min_lat, 20.0);
        assert_eq!(max_lon, 180.0);
        assert_eq!(max_lat, 40.0);

        // Non-wrapping case, but with normalization
        let (min_lon, min_lat, max_lon, max_lat) =
            adjust_bbox_for_center(190.0, 20.0, 200.0, 40.0, "eurocentric", false);
        assert_eq!(min_lon, -170.0); // 190 normalized to eurocentric
        assert_eq!(min_lat, 20.0);
        assert_eq!(max_lon, -160.0); // 200 normalized to eurocentric
        assert_eq!(max_lat, 40.0);
    }
}
