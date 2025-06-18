//! Point query endpoint handler.
//!
//! Returns interpolated values for one or more variables at a specific point in space-time.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::RossbyError;
use crate::state::AppState;

/// Query parameters for point endpoint
#[derive(Debug, Deserialize)]
pub struct PointQuery {
    /// Longitude coordinate
    pub lon: f64,
    /// Latitude coordinate
    pub lat: f64,
    /// Time index (0-based)
    pub time_index: Option<usize>,
    /// Comma-separated list of variables to query
    pub vars: String,
    /// Interpolation method (nearest, bilinear, bicubic)
    pub interpolation: Option<String>,
}

/// Response for point query
#[derive(Debug, Serialize)]
pub struct PointResponse {
    #[serde(flatten)]
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// Handle GET /point requests
pub async fn point_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PointQuery>,
) -> Response {
    match process_point_query(state, params) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": error.to_string()
            })),
        )
            .into_response(),
    }
}

/// Process a point query
fn process_point_query(
    state: Arc<AppState>,
    params: PointQuery,
) -> Result<PointResponse, RossbyError> {
    // Get coordinates
    let lon = params.lon;
    let lat = params.lat;

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

    // Get the list of variables to query
    let variables: Vec<String> = params
        .vars
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if variables.is_empty() {
        return Err(RossbyError::InvalidParameter {
            param: "vars".to_string(),
            message: "No variables specified".to_string(),
        });
    }

    // Get interpolation method (default to bilinear)
    let interpolation_method = params.interpolation.as_deref().unwrap_or("bilinear");
    let interpolator = crate::interpolation::get_interpolator(interpolation_method)?;

    // Results map
    let mut values = serde_json::Map::new();

    // Process each variable
    for var_name in variables {
        // Check if variable exists
        if !state.has_variable(&var_name) {
            return Err(RossbyError::VariableNotFound { name: var_name });
        }

        // Get variable dimensions
        let dimensions = state.get_variable_dimensions(&var_name)?;

        // Find dimension indices for lat, lon, and time
        let mut lat_dim_idx = None;
        let mut lon_dim_idx = None;
        let mut time_dim_idx = None;

        for (i, dim) in dimensions.iter().enumerate() {
            if dim == "lat" {
                lat_dim_idx = Some(i);
            } else if dim == "lon" {
                lon_dim_idx = Some(i);
            } else if dim == "time" {
                time_dim_idx = Some(i);
            }
        }

        // Ensure we have lat and lon dimensions
        let lat_dim_idx = lat_dim_idx.ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable {} does not have a lat dimension", var_name),
        })?;

        let lon_dim_idx = lon_dim_idx.ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable {} does not have a lon dimension", var_name),
        })?;

        // Get the data array
        let data = state.get_variable_checked(&var_name)?;

        // Get coordinates
        let lat_coords = state.get_coordinate_checked("lat")?;
        let lon_coords = state.get_coordinate_checked("lon")?;

        // Check if coordinates are within bounds
        if lon < *lon_coords.first().unwrap() || lon > *lon_coords.last().unwrap() {
            return Err(RossbyError::InvalidCoordinates {
                message: format!(
                    "Longitude {} is outside the range [{}, {}]",
                    lon,
                    lon_coords.first().unwrap(),
                    lon_coords.last().unwrap()
                ),
            });
        }

        if lat < *lat_coords.first().unwrap() || lat > *lat_coords.last().unwrap() {
            return Err(RossbyError::InvalidCoordinates {
                message: format!(
                    "Latitude {} is outside the range [{}, {}]",
                    lat,
                    lat_coords.first().unwrap(),
                    lat_coords.last().unwrap()
                ),
            });
        }

        // Find fractional indices
        let lon_idx = crate::interpolation::common::coord_to_index(lon, lon_coords)?;
        let lat_idx = crate::interpolation::common::coord_to_index(lat, lat_coords)?;

        // Set up the indices based on dimensionality
        let mut indices = vec![0.0; data.ndim()];
        indices[lon_dim_idx] = lon_idx;
        indices[lat_dim_idx] = lat_idx;

        // Set time index if present
        if let Some(idx) = time_dim_idx {
            indices[idx] = time_index as f64;
        }

        // Get the raw data as a slice
        let data_slice = data.as_slice().ok_or_else(|| RossbyError::DataNotFound {
            message: format!(
                "Cannot access data for variable {} as contiguous slice",
                var_name
            ),
        })?;

        // Interpolate the value
        let value = interpolator.interpolate(data_slice, data.shape(), &indices)?;

        // Add to results
        values.insert(
            var_name,
            serde_json::Value::Number(serde_json::Number::from_f64(value as f64).unwrap()),
        );
    }

    Ok(PointResponse { values })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::{AttributeValue, Dimension, Metadata, Variable};
    use ndarray::{Array, IxDyn};
    use std::collections::HashMap;

    // Helper function to create a test AppState
    fn create_test_state() -> Arc<AppState> {
        // Create a simple 2D grid with known values
        // Data is a 2x3 grid (lat x lon) with values 1-6
        let data_array =
            Array::from_shape_vec(IxDyn(&[2, 3]), vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();

        // Create metadata
        let mut dimensions = HashMap::new();
        dimensions.insert(
            "lat".to_string(),
            Dimension {
                name: "lat".to_string(),
                size: 2,
                is_unlimited: false,
            },
        );
        dimensions.insert(
            "lon".to_string(),
            Dimension {
                name: "lon".to_string(),
                size: 3,
                is_unlimited: false,
            },
        );

        // Create variable metadata
        let mut variables = HashMap::new();
        let mut var_attributes = HashMap::new();
        var_attributes.insert(
            "units".to_string(),
            AttributeValue::Text("degrees_C".to_string()),
        );

        variables.insert(
            "temperature".to_string(),
            Variable {
                name: "temperature".to_string(),
                dimensions: vec!["lat".to_string(), "lon".to_string()],
                shape: vec![2, 3],
                attributes: var_attributes,
                dtype: "f32".to_string(),
            },
        );

        // Create coordinate values
        let mut coordinates = HashMap::new();
        coordinates.insert("lat".to_string(), vec![10.0, 20.0]);
        coordinates.insert("lon".to_string(), vec![100.0, 110.0, 120.0]);

        // Create metadata
        let metadata = Metadata {
            global_attributes: HashMap::new(),
            dimensions,
            variables,
            coordinates,
        };

        // Create data map
        let mut data = HashMap::new();
        data.insert("temperature".to_string(), data_array);

        // Create config
        let config = Config::default();

        // Create AppState
        Arc::new(AppState::new(config, metadata, data))
    }

    #[test]
    fn test_point_query_success() {
        let state = create_test_state();

        // Query a point at exact grid location (10.0, 100.0) - should be exactly 1.0
        let params = PointQuery {
            lon: 100.0,
            lat: 10.0,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("nearest".to_string()),
        };

        let result = process_point_query(state.clone(), params).unwrap();
        let value = result.values.get("temperature").unwrap().as_f64().unwrap();
        assert_eq!(value, 1.0);

        // Query a point in between grid points with bilinear interpolation
        let params = PointQuery {
            lon: 105.0, // halfway between 100.0 and 110.0
            lat: 15.0,  // halfway between 10.0 and 20.0
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("bilinear".to_string()),
        };

        let result = process_point_query(state.clone(), params).unwrap();
        let value = result.values.get("temperature").unwrap().as_f64().unwrap();
        // Expected value: linear interpolation between 1.0, 2.0, 4.0, 5.0
        assert!((value - 3.0).abs() < 1e-5);
    }

    #[test]
    fn test_multiple_variables() {
        // For this test, we would need a more complex test state with multiple variables
        // For now, we'll just test the error case when an invalid variable is requested
        let state = create_test_state();

        let params = PointQuery {
            lon: 100.0,
            lat: 10.0,
            time_index: None,
            vars: "temperature,humidity".to_string(), // humidity doesn't exist
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::VariableNotFound { name }) = result {
            assert_eq!(name, "humidity");
        } else {
            panic!("Expected VariableNotFound error");
        }
    }

    #[test]
    fn test_out_of_bounds() {
        let state = create_test_state();

        // Test out of bounds longitude
        let params = PointQuery {
            lon: 130.0, // outside the range of [100.0, 120.0]
            lat: 10.0,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::InvalidCoordinates { .. }) = result {
            // Expected error
        } else {
            panic!("Expected InvalidCoordinates error");
        }

        // Test out of bounds latitude
        let params = PointQuery {
            lon: 100.0,
            lat: 30.0, // outside the range of [10.0, 20.0]
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::InvalidCoordinates { .. }) = result {
            // Expected error
        } else {
            panic!("Expected InvalidCoordinates error");
        }
    }

    #[test]
    fn test_invalid_interpolation() {
        let state = create_test_state();

        let params = PointQuery {
            lon: 100.0,
            lat: 10.0,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("invalid_method".to_string()),
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::InvalidParameter { param, .. }) = result {
            assert_eq!(param, "interpolation");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }

    #[test]
    fn test_empty_vars() {
        let state = create_test_state();

        let params = PointQuery {
            lon: 100.0,
            lat: 10.0,
            time_index: None,
            vars: "".to_string(), // Empty variable list
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::InvalidParameter { param, .. }) = result {
            assert_eq!(param, "vars");
        } else {
            panic!("Expected InvalidParameter error");
        }
    }
}
