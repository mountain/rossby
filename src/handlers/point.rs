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
use std::time::Instant;
use tracing::{debug, info, warn};

use crate::error::RossbyError;
use crate::logging::{generate_request_id, log_request_error};
use crate::state::AppState;

/// Query parameters for point endpoint
#[derive(Debug, Deserialize, Clone)]
pub struct PointQuery {
    // File-specific physical values
    /// Longitude coordinate (file-specific name)
    #[serde(default)]
    pub lon: Option<f64>,
    /// Latitude coordinate (file-specific name)
    #[serde(default)]
    pub lat: Option<f64>,
    /// Time value (file-specific name)
    #[serde(default)]
    pub time: Option<f64>,

    // Canonical physical values with underscore prefix
    /// Longitude coordinate (canonical name with underscore prefix)
    #[serde(rename = "_longitude", default)]
    pub _longitude: Option<f64>,
    /// Latitude coordinate (canonical name with underscore prefix)
    #[serde(rename = "_latitude", default)]
    pub _latitude: Option<f64>,
    /// Time value (canonical name with underscore prefix)
    #[serde(rename = "_time", default)]
    pub _time: Option<f64>,

    // Raw indices with double-underscore prefix
    /// Longitude index (canonical name with double-underscore prefix)
    #[serde(rename = "__longitude_index", default)]
    pub __longitude_index: Option<usize>,
    /// Latitude index (canonical name with double-underscore prefix)
    #[serde(rename = "__latitude_index", default)]
    pub __latitude_index: Option<usize>,
    /// Time index (canonical name with double-underscore prefix)
    #[serde(rename = "__time_index", default)]
    pub __time_index: Option<usize>,

    // Deprecated parameters
    /// Time index (0-based) - DEPRECATED, use __time_index instead
    #[serde(default)]
    pub time_index: Option<usize>,

    // Other parameters
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
    let request_id = generate_request_id();
    let start_time = Instant::now();

    // Log request parameters
    debug!(
        endpoint = "/point",
        request_id = %request_id,
        lon = ?params.lon,
        lat = ?params.lat,
        time = ?params.time,
        time_index = ?params.time_index,
        vars = %params.vars,
        interpolation = ?params.interpolation,
        "Processing point query"
    );

    match process_point_query(state, params.clone()) {
        Ok(response) => {
            // Log successful request
            let duration = start_time.elapsed();
            info!(
                endpoint = "/point",
                request_id = %request_id,
                duration_us = duration.as_micros() as u64,
                "Point query successful"
            );

            Json(response).into_response()
        }
        Err(error) => {
            // Log error
            log_request_error(
                &error,
                "/point",
                &request_id,
                Some(&format!("vars={}", params.vars)),
            );

            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": error.to_string(),
                    "request_id": request_id
                })),
            )
                .into_response()
        }
    }
}

/// Process a point query
fn process_point_query(
    state: Arc<AppState>,
    params: PointQuery,
) -> Result<PointResponse, RossbyError> {
    // Setup indices
    #[allow(unused_assignments)]
    let mut longitude_idx: Option<usize> = None;
    #[allow(unused_assignments)]
    let mut latitude_idx: Option<usize> = None;
    #[allow(unused_assignments)]
    let mut time_idx: Option<usize> = None;

    // Get longitude using raw index or physical value
    #[allow(unused_assignments)]
    let mut lon_value = None;
    if let Some(idx) = params.__longitude_index {
        // Use raw index directly
        let lon_coords = state
            .get_coordinate_checked("lon")
            .or_else(|_| state.get_coordinate_checked("_longitude"))
            .or_else(|_| state.get_coordinate_checked("longitude"))?;

        // Check if index is in bounds
        if idx >= lon_coords.len() {
            return Err(RossbyError::IndexOutOfBounds {
                param: "__longitude_index".to_string(),
                value: idx.to_string(),
                max: lon_coords.len() - 1,
            });
        }

        longitude_idx = Some(idx);
        lon_value = Some(lon_coords[idx]);
    } else {
        // Get longitude coordinate - try direct file-specific name first, then prefixed canonical name
        let lon = match (params.lon, params._longitude) {
            (Some(value), _) => value,
            (None, Some(value)) => value,
            (None, None) => {
                return Err(RossbyError::InvalidParameter {
                    param: "longitude".to_string(),
                    message: "Missing longitude coordinate. Provide either file-specific name (e.g., 'lon'), canonical name with underscore prefix (e.g., '_longitude'), or raw index with double-underscore prefix (e.g., '__longitude_index')".to_string(),
                })
            }
        };
        lon_value = Some(lon);
    }

    // Get latitude using raw index or physical value
    #[allow(unused_assignments)]
    let mut lat_value = None;
    if let Some(idx) = params.__latitude_index {
        // Use raw index directly
        let lat_coords = state
            .get_coordinate_checked("lat")
            .or_else(|_| state.get_coordinate_checked("_latitude"))
            .or_else(|_| state.get_coordinate_checked("latitude"))?;

        // Check if index is in bounds
        if idx >= lat_coords.len() {
            return Err(RossbyError::IndexOutOfBounds {
                param: "__latitude_index".to_string(),
                value: idx.to_string(),
                max: lat_coords.len() - 1,
            });
        }

        latitude_idx = Some(idx);
        lat_value = Some(lat_coords[idx]);
    } else {
        // Get latitude coordinate - try direct file-specific name first, then prefixed canonical name
        let lat = match (params.lat, params._latitude) {
            (Some(value), _) => value,
            (None, Some(value)) => value,
            (None, None) => {
                return Err(RossbyError::InvalidParameter {
                    param: "latitude".to_string(),
                    message: "Missing latitude coordinate. Provide either file-specific name (e.g., 'lat'), canonical name with underscore prefix (e.g., '_latitude'), or raw index with double-underscore prefix (e.g., '__latitude_index')".to_string(),
                })
            }
        };

        lat_value = Some(lat);
    }

    // Get time using raw index or physical value
    if let Some(idx) = params.__time_index {
        // Use raw index directly
        if idx >= state.time_dim_size() {
            return Err(RossbyError::IndexOutOfBounds {
                param: "__time_index".to_string(),
                value: idx.to_string(),
                max: state.time_dim_size() - 1,
            });
        }

        time_idx = Some(idx);
    } else if let Some(idx) = params.time_index {
        // Use deprecated time_index parameter (with warning)
        warn!(
            param = "time_index",
            deprecated_since = "0.1.0",
            replacement = "__time_index",
            "The 'time_index' parameter is deprecated. Please use '__time_index' instead."
        );

        if idx >= state.time_dim_size() {
            return Err(RossbyError::IndexOutOfBounds {
                param: "time_index".to_string(),
                value: idx.to_string(),
                max: state.time_dim_size() - 1,
            });
        }

        time_idx = Some(idx);
    } else if let Some(time_val) = params.time.or(params._time) {
        // Use physical time value - convert to index with exact match
        // Get time coordinates
        let _time_coords = state
            .get_coordinate_checked("time")
            .or_else(|_| state.get_coordinate_checked("_time"))?;

        // Find exact match for time value
        match state.find_coordinate_index_exact("time", time_val) {
            Ok(idx) => time_idx = Some(idx),
            Err(e) => return Err(e),
        }
    } else {
        // Default to time index 0
        time_idx = Some(0);
    }

    // Get time index (default to 0)
    let time_index = time_idx.unwrap_or(0);

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

        // Find dimension indices for lat, lon, and time with alias support
        let mut lat_dim_idx = None;
        let mut lon_dim_idx = None;
        let mut time_dim_idx = None;

        for (i, dim) in dimensions.iter().enumerate() {
            // Try to get the canonical name for this dimension
            let canonical = state.get_canonical_dimension_name(dim).unwrap_or(dim);

            if dim == "lat" || canonical == "latitude" {
                lat_dim_idx = Some(i);
            } else if dim == "lon" || canonical == "longitude" {
                lon_dim_idx = Some(i);
            } else if dim == "time" || canonical == "time" {
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

        // Get coordinates using dimension aliases
        let lon_coords = state
            .get_coordinate_checked("lon")
            .or_else(|_| state.get_coordinate_checked("_longitude"))
            .or_else(|_| state.get_coordinate_checked("longitude"))?;

        let lat_coords = state
            .get_coordinate_checked("lat")
            .or_else(|_| state.get_coordinate_checked("_latitude"))
            .or_else(|_| state.get_coordinate_checked("latitude"))?;

        // Resolve indices from physical values if necessary
        let lon_idx = if let Some(idx) = longitude_idx {
            idx as f64
        } else {
            // Check if coordinates are within bounds
            let lon = lon_value.unwrap();
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

            // Find fractional index
            crate::interpolation::common::coord_to_index(lon, lon_coords)?
        };

        let lat_idx = if let Some(idx) = latitude_idx {
            idx as f64
        } else {
            // Check if coordinates are within bounds
            let lat = lat_value.unwrap();
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

            // Find fractional index
            crate::interpolation::common::coord_to_index(lat, lat_coords)?
        };

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

    // Helper function to create a test AppState with dimension aliases
    fn create_test_state_with_aliases() -> Arc<AppState> {
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

        // Create config with dimension aliases
        let mut config = Config::default();
        let mut aliases = HashMap::new();
        aliases.insert("latitude".to_string(), "lat".to_string());
        aliases.insert("longitude".to_string(), "lon".to_string());
        config.data.dimension_aliases = aliases;

        // Create AppState
        Arc::new(AppState::new(config, metadata, data))
    }

    #[test]
    fn test_point_query_success() {
        let state = create_test_state();

        // Query a point at exact grid location (10.0, 100.0) - should be exactly 1.0
        let params = PointQuery {
            lon: Some(100.0),
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("nearest".to_string()),
        };

        let result = process_point_query(state.clone(), params).unwrap();
        let value = result.values.get("temperature").unwrap().as_f64().unwrap();
        assert_eq!(value, 1.0);

        // Query a point in between grid points with bilinear interpolation
        let params = PointQuery {
            lon: Some(105.0), // halfway between 100.0 and 110.0
            lat: Some(15.0),  // halfway between 10.0 and 20.0
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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
            lon: Some(100.0),
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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
            lon: Some(130.0), // outside the range of [100.0, 120.0]
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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
            lon: Some(100.0),
            lat: Some(30.0), // outside the range of [10.0, 20.0]
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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
            lon: Some(100.0),
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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
            lon: Some(100.0),
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
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

    #[test]
    fn test_dimension_aliases() {
        // Test with prefixed canonical names
        let state = create_test_state();

        // Use _longitude and _latitude instead of lon and lat
        let params = PointQuery {
            lon: None,
            lat: None,
            time: None,
            _longitude: Some(100.0),
            _latitude: Some(10.0),
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("nearest".to_string()),
        };

        let result = process_point_query(state.clone(), params);
        assert!(
            result.is_ok(),
            "Query with prefixed canonical names failed: {:?}",
            result
        );
        let value = result
            .unwrap()
            .values
            .get("temperature")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(value, 1.0);

        // Test with config-based aliases
        let state_with_aliases = create_test_state_with_aliases();

        // The AppState should now recognize 'longitude' and 'latitude' as aliases for 'lon' and 'lat'
        let params = PointQuery {
            lon: None,
            lat: None,
            time: None,
            _longitude: Some(120.0), // Using the right-most longitude to get value 6.0
            _latitude: Some(20.0),
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("nearest".to_string()),
        };

        let result = process_point_query(state_with_aliases.clone(), params);
        assert!(
            result.is_ok(),
            "Query with aliased names failed: {:?}",
            result
        );
        let value = result
            .unwrap()
            .values
            .get("temperature")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(value, 6.0); // Value at [1, 2] = bottom right corner (lat=20.0, lon=120.0)
    }

    #[test]
    fn test_raw_indices() {
        let state = create_test_state();

        // Query using raw indices
        let params = PointQuery {
            lon: None,
            lat: None,
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: Some(0), // First longitude (100.0)
            __latitude_index: Some(0),  // First latitude (10.0)
            __time_index: Some(0),      // First time index
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: Some("nearest".to_string()),
        };

        let result = process_point_query(state.clone(), params);
        assert!(
            result.is_ok(),
            "Query with raw indices failed: {:?}",
            result
        );
        let value = result
            .unwrap()
            .values
            .get("temperature")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(value, 1.0); // Should be the same as querying for (lon=100.0, lat=10.0)

        // Test index out of bounds
        let params = PointQuery {
            lon: None,
            lat: None,
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: Some(3), // Out of bounds (max is 2)
            __latitude_index: Some(0),
            __time_index: None,
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_err());

        if let Err(RossbyError::IndexOutOfBounds { param, value, max }) = result {
            assert_eq!(param, "__longitude_index");
            assert_eq!(value, "3");
            assert_eq!(max, 2);
        } else {
            panic!("Expected IndexOutOfBounds error");
        }
    }

    #[test]
    fn test_deprecated_time_index() {
        let state = create_test_state();

        // Query using deprecated time_index
        let params = PointQuery {
            lon: Some(100.0),
            lat: Some(10.0),
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: None,
            __time_index: None,
            time_index: Some(0), // Using deprecated parameter
            vars: "temperature".to_string(),
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_ok());
        let value = result
            .unwrap()
            .values
            .get("temperature")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(value, 1.0);
    }

    #[test]
    fn test_mixed_query_params() {
        let state = create_test_state();

        // Mix of physical values and raw indices
        let params = PointQuery {
            lon: Some(100.0), // Physical value
            lat: None,
            time: None,
            _longitude: None,
            _latitude: None,
            _time: None,
            __longitude_index: None,
            __latitude_index: Some(0), // Raw index
            __time_index: Some(0),     // Raw index
            time_index: None,
            vars: "temperature".to_string(),
            interpolation: None,
        };

        let result = process_point_query(state.clone(), params);
        assert!(result.is_ok());
        let value = result
            .unwrap()
            .values
            .get("temperature")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(value, 1.0);
    }
}
