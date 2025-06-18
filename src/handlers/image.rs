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
}

/// Parse bounding box string into values
///
/// Format: "min_lon,min_lat,max_lon,max_lat"
fn parse_bbox(bbox_str: &str) -> Result<(f32, f32, f32, f32)> {
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

    if min_lon >= max_lon || min_lat >= max_lat {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Bounding box min values must be less than max values".to_string(),
        });
    }

    Ok((min_lon, min_lat, max_lon, max_lat))
}

/// Generate an image from 2D data array using specified colormap
fn generate_image(
    data: ArrayView2<f32>,
    width: u32,
    height: u32,
    colormap: &dyn Colormap,
) -> RgbaImage {
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

    // For simplicity, we're using a direct mapping from data grid to image pixels
    // A more sophisticated approach would use interpolation to handle different resolutions

    let data_height = data.shape()[0] as u32;
    let data_width = data.shape()[1] as u32;

    for y in 0..height {
        for x in 0..width {
            // Map image coordinates to data indices
            let data_x = (x as f32 * data_width as f32 / width as f32).floor() as usize;
            let data_y = (y as f32 * data_height as f32 / height as f32).floor() as usize;

            // Get data value - default to black for out of bounds
            let data_value = if data_x < data_width as usize && data_y < data_height as usize {
                data[[data_y, data_x]]
            } else {
                f32::NAN
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

    img
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

    // Parse bounding box (if provided)
    let (min_lon, min_lat, max_lon, max_lat) = if let Some(ref bbox) = params.bbox {
        parse_bbox(bbox)?
    } else {
        // Use full domain if no bbox specified
        state.get_lat_lon_bounds()?
    };

    // Get image dimensions
    let width = params.width.unwrap_or(DEFAULT_WIDTH);
    let height = params.height.unwrap_or(DEFAULT_HEIGHT);

    // Get colormap
    let colormap_name = params.colormap.as_deref().unwrap_or(DEFAULT_COLORMAP);
    let colormap = colormaps::get_colormap(colormap_name)?;

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

    // Get data for the specified time slice
    let data = state.get_data_slice(&var_name, time_index, min_lon, min_lat, max_lon, max_lat)?;

    // Generate the image
    let img = generate_image(data.view(), width, height, colormap.as_ref());

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
        // Valid bbox
        let result = parse_bbox("10.5,20.5,30.5,40.5");
        assert!(result.is_ok());
        let (min_lon, min_lat, max_lon, max_lat) = result.unwrap();
        assert_eq!(min_lon, 10.5);
        assert_eq!(min_lat, 20.5);
        assert_eq!(max_lon, 30.5);
        assert_eq!(max_lat, 40.5);

        // Invalid format
        assert!(parse_bbox("10.5,20.5,30.5").is_err());

        // Invalid numbers
        assert!(parse_bbox("10.5,20.5,not_a_number,40.5").is_err());

        // Invalid bounds
        assert!(parse_bbox("30.5,20.5,10.5,40.5").is_err()); // min_lon > max_lon
        assert!(parse_bbox("10.5,40.5,30.5,20.5").is_err()); // min_lat > max_lat
    }
}
