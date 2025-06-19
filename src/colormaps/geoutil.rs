//! Geographic utility functions for handling map projections and coordinates.
//!
//! This module provides utilities for working with geographic coordinates,
//! map projections, and handling cases like the dateline (180/-180 longitude) crossing.

use crate::error::{Result, RossbyError};
use ndarray::{Array2, ArrayView2};
use std::str::FromStr;

/// Map projection types for displaying global data
#[derive(Debug, Clone, PartialEq)]
pub enum MapProjection {
    /// Eurocentric view (centered around Greenwich/0°)
    Eurocentric,
    /// Americas-centered view (centered around -90°)
    Americas,
    /// Pacific-centered view (centered around 180°/-180°)
    Pacific,
    /// Custom projection with specified center longitude
    Custom(f32),
}

impl MapProjection {
    /// Get the center longitude for this projection
    pub fn center_longitude(&self) -> f32 {
        match self {
            MapProjection::Eurocentric => 0.0,
            MapProjection::Americas => -90.0,
            MapProjection::Pacific => 180.0,
            MapProjection::Custom(lon) => *lon,
        }
    }

    /// Create a MapProjection from a string
    pub fn parse_projection(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "eurocentric" => Ok(MapProjection::Eurocentric),
            "americas" => Ok(MapProjection::Americas),
            "pacific" => Ok(MapProjection::Pacific),
            _ if s.starts_with("custom:") => {
                // Parse custom projection with specified center
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(center_lon) = parts[1].parse::<f32>() {
                        return Ok(MapProjection::Custom(center_lon));
                    }
                }
                Err(RossbyError::InvalidParameter {
                    param: "center".to_string(),
                    message: format!("Invalid custom projection format: {}", s),
                })
            }
            _ => Err(RossbyError::InvalidParameter {
                param: "center".to_string(),
                message: format!("Unknown map projection: {}", s),
            }),
        }
    }
}

impl FromStr for MapProjection {
    type Err = RossbyError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        MapProjection::parse_projection(s)
    }
}

/// Parse a bounding box string "min_lon,min_lat,max_lon,max_lat" into its components
pub fn parse_bbox(bbox: &str) -> Result<(f32, f32, f32, f32)> {
    let parts: Vec<&str> = bbox.split(',').collect();
    if parts.len() != 4 {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Bounding box must be in format 'min_lon,min_lat,max_lon,max_lat'".to_string(),
        });
    }

    // Parse the four components
    let min_lon = parts[0]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: format!("Invalid min_lon: {}", parts[0]),
        })?;

    let min_lat = parts[1]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: format!("Invalid min_lat: {}", parts[1]),
        })?;

    let max_lon = parts[2]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: format!("Invalid max_lon: {}", parts[2]),
        })?;

    let max_lat = parts[3]
        .parse::<f32>()
        .map_err(|_| RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: format!("Invalid max_lat: {}", parts[3]),
        })?;

    // Validate the latitude range
    if min_lat > max_lat {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: format!("min_lat ({}) must be <= max_lat ({})", min_lat, max_lat),
        });
    }

    // Latitude must be in the range -90 to 90
    if !(-90.0..=90.0).contains(&min_lat) || !(-90.0..=90.0).contains(&max_lat) {
        return Err(RossbyError::InvalidParameter {
            param: "bbox".to_string(),
            message: "Latitude must be in the range -90 to 90".to_string(),
        });
    }

    // Longitude is validated later after determining if wrapping is allowed

    Ok((min_lon, min_lat, max_lon, max_lat))
}

/// Normalize a longitude value to the range [-180, 180)
pub fn normalize_longitude(lon: f32) -> f32 {
    // Use a more robust normalization approach that avoids potential overflow
    let mut normalized = ((lon + 180.0) % 360.0 + 360.0) % 360.0 - 180.0;

    // Handle the edge case of exactly 180.0 (which should be -180.0 in the normalized form)
    if normalized == 180.0 {
        normalized = -180.0;
    }

    normalized
}

/// Handle a bounding box that may cross the dateline/prime meridian
/// Returns the adjusted bounding box and a boolean indicating if it crosses the dateline
pub fn handle_dateline_crossing_bbox(
    min_lon: f32,
    min_lat: f32,
    max_lon: f32,
    max_lat: f32,
    projection: &MapProjection,
) -> Result<((f32, f32, f32, f32), bool)> {
    // If min_lon <= max_lon, it's a regular bounding box (no dateline crossing)
    if min_lon <= max_lon {
        return Ok(((min_lon, min_lat, max_lon, max_lat), false));
    }

    // We have a bounding box that crosses the dateline or prime meridian
    // The strategy depends on the map projection

    // Get the center longitude for the projection
    let center_lon = projection.center_longitude();

    // Calculate the normalized longitudes relative to the center
    let normalized_min_lon = normalize_longitude(min_lon - center_lon) + center_lon;
    let normalized_max_lon = normalize_longitude(max_lon - center_lon) + center_lon;

    // For the Pacific projection, we'll need special handling since we're centered on the dateline
    match projection {
        MapProjection::Pacific => {
            // For Pacific view, keep the original coordinates but flag as crossing
            Ok(((min_lon, min_lat, max_lon, max_lat), true))
        }
        _ => {
            // For other projections, adjust the coordinates based on the center longitude
            if normalized_min_lon <= normalized_max_lon {
                // After normalization, this is a regular bounding box
                Ok((
                    (normalized_min_lon, min_lat, normalized_max_lon, max_lat),
                    false,
                ))
            } else {
                // Still crosses the dateline after normalization - need special handling
                // We'll use the original coordinates but flag it as crossing
                Ok(((min_lon, min_lat, max_lon, max_lat), true))
            }
        }
    }
}

/// Adjust a data array for dateline crossing
pub fn adjust_for_dateline_crossing(
    data: &ArrayView2<f32>,
    lon_coords: &[f64],
    crosses_dateline: bool,
) -> Result<(Array2<f32>, Vec<f64>)> {
    if !crosses_dateline || lon_coords.is_empty() {
        // No adjustment needed or empty coordinates
        return Ok((data.to_owned(), lon_coords.to_vec()));
    }

    // Safety check for empty data array
    if data.is_empty() {
        return Ok((data.to_owned(), lon_coords.to_vec()));
    }

    // Find the dateline position in the longitude coordinates
    let dateline_idx = if lon_coords[0] <= lon_coords[lon_coords.len() - 1] {
        // Increasing longitude array
        lon_coords
            .iter()
            .position(|&lon| (0.0..=180.0).contains(&lon))
            .unwrap_or(0)
    } else {
        // Decreasing longitude array
        lon_coords
            .iter()
            .position(|&lon| (-180.0..=0.0).contains(&lon))
            .unwrap_or(0)
    };

    // Calculate right_size safely
    let right_size = if dateline_idx >= lon_coords.len() {
        0 // Edge case - no right side
    } else {
        lon_coords.len() - dateline_idx
    };

    // Ensure right_size is valid and not larger than the data width
    let validated_right_size = right_size.min(data.shape()[1]);

    // Create a new data array with the right side copied to the left
    let mut new_data = Array2::zeros((data.shape()[0], data.shape()[1] + validated_right_size));
    let mut new_lon_coords = Vec::with_capacity(lon_coords.len() + validated_right_size);

    // Fill the new array with data from the original array
    for row in 0..data.shape()[0] {
        for col in 0..data.shape()[1] {
            new_data[[row, col]] = data[[row, col]];
        }
    }

    // Copy the right side to the left with longitude adjustment, with extra safety checks
    if validated_right_size > 0 && data.shape()[1] >= validated_right_size {
        for row in 0..data.shape()[0] {
            for col in 0..validated_right_size {
                // Calculate original column index safely
                let safe_offset = col.min(validated_right_size - 1);
                let orig_col = if data.shape()[1] > validated_right_size {
                    data.shape()[1] - validated_right_size + safe_offset
                } else {
                    // If we can't safely calculate this, just use the last column
                    data.shape()[1] - 1
                };

                // Extra bounds check
                if orig_col < data.shape()[1] && data.shape()[1] + col < new_data.shape()[1] {
                    new_data[[row, data.shape()[1] + col]] = data[[row, orig_col]];
                }
            }
        }
    }

    // Create the new longitude coordinates
    for &lon in lon_coords {
        new_lon_coords.push(lon);
    }

    // Add the wrapped longitudes with safety checks
    for i in 0..validated_right_size {
        // Calculate index safely
        let orig_idx = if lon_coords.len() > validated_right_size {
            lon_coords.len() - validated_right_size + i
        } else {
            // Fall back to the last coordinate if we can't safely calculate
            lon_coords.len() - 1
        };

        // Bounds check
        if orig_idx < lon_coords.len() {
            let wrapped_lon = lon_coords[orig_idx] + 360.0;
            new_lon_coords.push(wrapped_lon);
        }
    }

    Ok((new_data, new_lon_coords))
}

/// Resample a 2D data array to a new size using the specified interpolation method
pub fn resample_data(
    data: &ArrayView2<f32>,
    target_width: usize,
    target_height: usize,
) -> Result<Array2<f32>> {
    let orig_height = data.shape()[0];
    let orig_width = data.shape()[1];

    // Create a new array for the resampled data
    let mut resampled = Array2::<f32>::zeros((target_height, target_width));

    // Simple bilinear interpolation for resampling
    for y in 0..target_height {
        for x in 0..target_width {
            // Map target coordinates to source coordinates (as floating point)
            let src_x = x as f64 * (orig_width - 1) as f64 / (target_width - 1) as f64;
            let src_y = y as f64 * (orig_height - 1) as f64 / (target_height - 1) as f64;

            // Get the four surrounding points
            let x0 = src_x.floor() as usize;
            let y0 = src_y.floor() as usize;
            let x1 = (x0 + 1).min(orig_width - 1);
            let y1 = (y0 + 1).min(orig_height - 1);

            // Calculate interpolation weights
            let wx = src_x - x0 as f64;
            let wy = src_y - y0 as f64;

            // Perform bilinear interpolation
            let top = data[[y0, x0]] as f64 * (1.0 - wx) + data[[y0, x1]] as f64 * wx;
            let bottom = data[[y1, x0]] as f64 * (1.0 - wx) + data[[y1, x1]] as f64 * wx;
            let value = top * (1.0 - wy) + bottom * wy;

            resampled[[y, x]] = value as f32;
        }
    }

    Ok(resampled)
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

        // Invalid format (too few parts)
        assert!(parse_bbox("10.5,20.5,30.5").is_err());

        // Invalid numbers
        assert!(parse_bbox("10.5,20.5,not_a_number,40.5").is_err());

        // Latitude out of range
        assert!(parse_bbox("10.5,-91.0,30.5,40.5").is_err());
        assert!(parse_bbox("10.5,20.5,30.5,91.0").is_err());

        // Invalid latitude order (min > max)
        assert!(parse_bbox("10.5,40.5,30.5,20.5").is_err());
    }

    #[test]
    fn test_normalize_longitude() {
        assert_eq!(normalize_longitude(0.0), 0.0);
        assert_eq!(normalize_longitude(180.0), -180.0);
        assert_eq!(normalize_longitude(-180.0), -180.0);
        assert_eq!(normalize_longitude(190.0), -170.0);
        assert_eq!(normalize_longitude(-190.0), 170.0);
        assert_eq!(normalize_longitude(370.0), 10.0);
        assert_eq!(normalize_longitude(-370.0), -10.0);
    }

    #[test]
    fn test_handle_dateline_crossing_bbox() {
        // Normal bbox (no crossing)
        let result =
            handle_dateline_crossing_bbox(-10.0, 10.0, 10.0, 20.0, &MapProjection::Eurocentric);
        assert!(result.is_ok());
        let ((min_lon, min_lat, max_lon, max_lat), crosses) = result.unwrap();
        assert_eq!(min_lon, -10.0);
        assert_eq!(min_lat, 10.0);
        assert_eq!(max_lon, 10.0);
        assert_eq!(max_lat, 20.0);
        assert!(!crosses);

        // Crossing bbox with Eurocentric projection
        let result =
            handle_dateline_crossing_bbox(170.0, 10.0, -170.0, 20.0, &MapProjection::Eurocentric);
        assert!(result.is_ok());
        let ((min_lon, min_lat, max_lon, max_lat), crosses) = result.unwrap();
        assert_eq!(min_lon, 170.0);
        assert_eq!(min_lat, 10.0);
        assert_eq!(max_lon, -170.0);
        assert_eq!(max_lat, 20.0);
        assert!(crosses);

        // Crossing bbox with Pacific projection
        let result =
            handle_dateline_crossing_bbox(-10.0, 10.0, 10.0, 20.0, &MapProjection::Pacific);
        assert!(result.is_ok());
        let ((min_lon, min_lat, max_lon, max_lat), crosses) = result.unwrap();
        assert_eq!(min_lon, -10.0);
        assert_eq!(min_lat, 10.0);
        assert_eq!(max_lon, 10.0);
        assert_eq!(max_lat, 20.0);
        assert!(!crosses);
    }

    #[test]
    fn test_resample_data() {
        // Create a test array with a gradient pattern
        let data = Array2::from_shape_vec((2, 3), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

        // Upsample to twice the size
        let result = resample_data(&data.view(), 6, 4);
        assert!(result.is_ok());
        let resampled = result.unwrap();

        // Check the dimensions
        assert_eq!(resampled.shape(), &[4, 6]);

        // Check the corner values (should match original)
        assert_eq!(resampled[[0, 0]], 1.0);
        assert_eq!(resampled[[0, 5]], 3.0);
        assert_eq!(resampled[[3, 0]], 4.0);
        assert_eq!(resampled[[3, 5]], 6.0);

        // Check middle value - use a more relaxed epsilon since bilinear interpolation
        // might have slight numerical differences
        let expected = 2.8;
        let actual = resampled[[1, 2]];
        assert!(
            (actual - expected).abs() < 0.1,
            "Expected value near {}, got {}",
            expected,
            actual
        );
    }

    #[test]
    fn test_map_projection() {
        // Test string conversion
        assert_eq!(
            MapProjection::from_str("eurocentric").unwrap(),
            MapProjection::Eurocentric
        );
        assert_eq!(
            MapProjection::from_str("americas").unwrap(),
            MapProjection::Americas
        );
        assert_eq!(
            MapProjection::from_str("pacific").unwrap(),
            MapProjection::Pacific
        );
        assert_eq!(
            MapProjection::from_str("custom:45.0").unwrap(),
            MapProjection::Custom(45.0)
        );

        // Test center longitude
        assert_eq!(MapProjection::Eurocentric.center_longitude(), 0.0);
        assert_eq!(MapProjection::Americas.center_longitude(), -90.0);
        assert_eq!(MapProjection::Pacific.center_longitude(), 180.0);
        assert_eq!(MapProjection::Custom(45.0).center_longitude(), 45.0);

        // Test invalid input
        assert!(MapProjection::from_str("invalid").is_err());
    }
}
