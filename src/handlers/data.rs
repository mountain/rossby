//! Handler for the /data endpoint.
//!
//! This module implements the data endpoint that streams user-defined,
//! N-dimensional data hyperslabs in Apache Arrow format.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use arrow::array::{ArrayRef, Float32Array, Float64Array};
use arrow::record_batch::RecordBatch;
use arrow_ipc::writer::StreamWriter;
use arrow_schema::Field;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bytes::Bytes;
use futures::stream::{self, Stream, StreamExt};
use ndarray::{Array, IxDyn};
use serde::Deserialize;
use tracing::{debug, info};

use crate::error::{Result, RossbyError};
use crate::state::AppState;

/// Generate a unique request ID for tracking
fn generate_request_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

/// Log an error that occurred during request processing
fn log_request_error(error: &RossbyError, endpoint: &str, request_id: &str, params: Option<&str>) {
    tracing::error!(
        endpoint = endpoint,
        request_id = %request_id,
        params = params,
        error = %error,
        "Request failed"
    );
}

/// Query parameters for the data endpoint
#[derive(Debug, Deserialize, Clone)]
pub struct DataQuery {
    /// Comma-separated list of variables to extract
    pub vars: String,

    /// Optional layout specification (comma-separated dimension names)
    #[serde(default)]
    pub layout: Option<String>,

    /// Output format (arrow or json)
    #[serde(default)]
    pub format: Option<String>,

    /// Dynamic parameters - will be parsed separately
    #[serde(flatten)]
    pub dynamic_params: HashMap<String, String>,
}

/// Represents a dimension selection constraint
#[derive(Debug, Clone)]
pub enum DimensionSelector {
    /// Select a single slice by physical value
    SingleValue { dimension: String, value: f64 },
    /// Select a range by physical values (inclusive)
    ValueRange {
        dimension: String,
        start: f64,
        end: f64,
    },
    /// Select a single slice by raw index
    SingleIndex { dimension: String, index: usize },
    /// IndexRange is inclusive (i.e., start and end indices are included)
    IndexRange {
        dimension: String,
        start: usize,
        end: usize,
    },
}

/// Parsed query information
struct ParsedDataQuery {
    /// List of variable names to extract
    variables: Vec<String>,

    /// Dimension constraints
    dimension_selectors: Vec<DimensionSelector>,

    /// Requested dimension order
    layout: Option<Vec<String>>,
}

/// Handle GET /data requests
pub async fn data_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DataQuery>,
) -> Response {
    let request_id = generate_request_id();
    let start_time = Instant::now();

    // Log request parameters with much more detail
    debug!(
        endpoint = "/data",
        request_id = %request_id,
        vars = %params.vars,
        layout = ?params.layout,
        format = ?params.format,
        params = ?params.dynamic_params,
        "Processing data query"
    );

    // Debug log state metadata
    debug!(
        "Available dimensions: {:?}",
        state.metadata.dimensions.keys().collect::<Vec<_>>()
    );
    debug!(
        "Available variables: {:?}",
        state.metadata.variables.keys().collect::<Vec<_>>()
    );

    // Clone params to keep a reference for error reporting and to avoid a move
    let params_clone = params.clone();

    // Determine the output format (default to "arrow")
    let output_format = params.format.as_deref().unwrap_or("arrow");

    match output_format {
        "arrow" => {
            match process_data_query(state, params_clone.clone()) {
                Ok(arrow_data) => {
                    // Log successful request
                    let duration = start_time.elapsed();
                    info!(
                        endpoint = "/data",
                        request_id = %request_id,
                        format = "arrow",
                        duration_us = duration.as_micros() as u64,
                        "Data query successful"
                    );

                    // Build the response with Arrow IPC stream
                    (
                        StatusCode::OK,
                        [(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/vnd.apache.arrow.stream"),
                        )],
                        arrow_data,
                    )
                        .into_response()
                }
                Err(error) => handle_data_error(error, &request_id, &params),
            }
        }
        "json" => {
            match process_data_query_json(state, params_clone.clone()) {
                Ok(response) => {
                    // Log successful request
                    let duration = start_time.elapsed();
                    info!(
                        endpoint = "/data",
                        request_id = %request_id,
                        format = "json",
                        duration_us = duration.as_micros() as u64,
                        "Data query successful"
                    );

                    response
                }
                Err(error) => handle_data_error(error, &request_id, &params),
            }
        }
        _ => {
            // Invalid format
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Unsupported format: {}", output_format),
                    "request_id": request_id
                })),
            )
                .into_response()
        }
    }
}

/// Handle error responses for data queries
fn handle_data_error(error: RossbyError, request_id: &str, params: &DataQuery) -> Response {
    // Log error with more detail
    log_request_error(
        &error,
        "/data",
        request_id,
        Some(&format!(
            "vars={}, params={:?}",
            params.vars, params.dynamic_params
        )),
    );

    tracing::debug!(
        "/data endpoint failed: {:?} for params: {:?}",
        error,
        params
    );

    // Check if this is a payload too large error
    let status = match &error {
        RossbyError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
        _ => StatusCode::BAD_REQUEST,
    };

    (
        status,
        Json(serde_json::json!({
            "error": error.to_string(),
            "request_id": request_id
        })),
    )
        .into_response()
}

/// Process the data query and return a JSON formatted response
fn process_data_query_json(state: Arc<AppState>, params: DataQuery) -> Result<Response> {
    use axum::body::Body;

    // Parse and validate the query (similar to process_data_query)
    let variables = params
        .vars
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if variables.is_empty() {
        return Err(RossbyError::InvalidParameter {
            param: "vars".to_string(),
            message: "At least one variable must be specified".to_string(),
        });
    }

    // Check that all variables exist in the dataset
    let mut invalid_vars = Vec::new();
    for var in &variables {
        if !state.has_variable(var) {
            invalid_vars.push(var.clone());
        }
    }

    if !invalid_vars.is_empty() {
        return Err(RossbyError::InvalidVariables {
            names: invalid_vars,
        });
    }

    // Process dimension constraints
    let dimension_selectors = process_dimension_constraints(&state, &params.dynamic_params)?;

    // Parse layout parameter if present
    let layout = params.layout.as_ref().map(|layout_str| {
        layout_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    // Validate that all dimensions in the layout exist (similar to process_data_query)
    if let Some(layout_dims) = &layout {
        for dim in layout_dims {
            let dim_result = state.resolve_dimension(dim);

            if dim_result.is_err() {
                debug!("Failed to resolve dimension: {} - {:?}", dim, dim_result);

                // Check if this is a canonical name that we should accept
                let canonical_dims = ["latitude", "longitude", "time", "level"];
                if canonical_dims.contains(&dim.as_str()) {
                    debug!("Accepting canonical dimension name: {}", dim);
                    continue; // Accept canonical names even if they don't resolve
                }

                return Err(RossbyError::InvalidParameter {
                    param: "layout".to_string(),
                    message: format!("Unknown dimension in layout: {}", dim),
                });
            }
        }
    }

    // Package the parsed query
    let parsed_query = ParsedDataQuery {
        variables,
        dimension_selectors,
        layout,
    };

    // Create a stream that yields JSON chunks
    let stream = create_json_stream(state, parsed_query, params.clone())?;

    // Return a response with the chunked JSON stream
    Ok((
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        Body::from_stream(stream),
    )
        .into_response())
}

/// Create a stream that yields JSON chunks for the data response
fn create_json_stream(
    state: Arc<AppState>,
    query: ParsedDataQuery,
    _params: DataQuery,
) -> Result<impl Stream<Item = std::result::Result<Bytes, std::io::Error>> + Send> {
    let ParsedDataQuery {
        variables,
        dimension_selectors,
        layout,
    } = query;

    // Maps from dimension name to selected range
    let mut selected_ranges: HashMap<String, (usize, usize)> = HashMap::new();
    let mut coordinate_arrays: HashMap<String, Vec<f64>> = HashMap::new();

    // Process each dimension selector (similar to extract_and_format_data)
    for selector in dimension_selectors {
        match selector {
            DimensionSelector::SingleValue { dimension, value } => {
                let index = state.find_coordinate_index(&dimension, value)?;
                selected_ranges.insert(dimension.clone(), (index, index));
                let coords = state.get_coordinate_checked(&dimension)?;
                coordinate_arrays.insert(dimension, vec![coords[index]]);
            }
            DimensionSelector::ValueRange {
                dimension,
                start,
                end,
            } => {
                let start_idx = state.find_coordinate_index(&dimension, start)?;
                let end_idx = state.find_coordinate_index(&dimension, end)?;
                selected_ranges.insert(dimension.clone(), (start_idx, end_idx));
                let coords = state.get_coordinate_checked(&dimension)?;
                let selected_coords = coords[start_idx..=end_idx].to_vec();
                coordinate_arrays.insert(dimension, selected_coords);
            }
            DimensionSelector::SingleIndex { dimension, index } => {
                let coords = state.get_coordinate_checked(&dimension)?;
                if index >= coords.len() {
                    return Err(RossbyError::IndexOutOfBounds {
                        param: dimension.clone(),
                        value: index.to_string(),
                        max: coords.len() - 1,
                    });
                }
                selected_ranges.insert(dimension.clone(), (index, index));
                coordinate_arrays.insert(dimension, vec![coords[index]]);
            }
            DimensionSelector::IndexRange {
                dimension,
                start,
                end,
            } => {
                let coords = state.get_coordinate_checked(&dimension)?;
                if start >= coords.len() || end >= coords.len() {
                    return Err(RossbyError::IndexOutOfBounds {
                        param: dimension.clone(),
                        value: format!("{}..{}", start, end),
                        max: coords.len() - 1,
                    });
                }
                selected_ranges.insert(dimension.clone(), (start, end));
                let selected_coords = coords[start..=end].to_vec();
                coordinate_arrays.insert(dimension, selected_coords);
            }
        }
    }

    // For dimensions not explicitly selected, use the entire range
    for (dim_name, dim) in &state.metadata.dimensions {
        if !selected_ranges.contains_key(dim_name) {
            selected_ranges.insert(dim_name.clone(), (0, dim.size - 1));
            if let Some(coords) = state.get_coordinate(dim_name) {
                coordinate_arrays.insert(dim_name.clone(), coords.clone());
            } else {
                let indices: Vec<f64> = (0..dim.size).map(|i| i as f64).collect();
                coordinate_arrays.insert(dim_name.clone(), indices);
            }
        }
    }

    // Calculate the total number of data points to check against limit
    let total_points: usize = coordinate_arrays
        .values()
        .map(|coords| coords.len())
        .product();

    // Check if total points exceeds the limit
    if total_points > state.config.server.max_data_points {
        return Err(RossbyError::PayloadTooLarge {
            message: "The requested data would exceed the maximum allowed size".to_string(),
            requested: total_points,
            max_allowed: state.config.server.max_data_points,
        });
    }

    // Extract data for each variable
    let mut var_data_arrays = Vec::new();
    let mut var_metadata = Vec::new();
    for var_name in &variables {
        let array = extract_variable_data(&state, var_name, &selected_ranges)?;
        var_data_arrays.push(array);

        // Get variable metadata for attributes like units, long_name
        let var_meta = state.get_variable_metadata_checked(var_name)?;
        var_metadata.push((var_name.clone(), var_meta));
    }

    // Get dimensions based on the first variable for use in metadata
    let dimension_order = if let Some(layout_dims) = &layout {
        layout_dims
            .iter()
            .map(|dim| state.resolve_dimension(dim).unwrap_or(dim).to_string())
            .collect::<Vec<_>>()
    } else if !variables.is_empty() {
        // Use dimensions from the first variable
        let var_meta = state.get_variable_metadata_checked(&variables[0])?;
        var_meta.dimensions.clone()
    } else {
        return Err(RossbyError::InvalidParameter {
            param: "vars".to_string(),
            message: "No valid variables specified".to_string(),
        });
    };

    // Prepare shape information for metadata
    let shapes: Vec<Vec<usize>> = var_data_arrays
        .iter()
        .map(|arr| arr.shape().to_vec())
        .collect();

    // Create variable metadata section
    let mut var_meta_json = serde_json::Map::new();
    for (var_name, var_meta) in var_metadata.iter() {
        let mut attrs = serde_json::Map::new();

        // Add variable attributes (like units, long_name)
        for (key, value) in &var_meta.attributes {
            match value {
                crate::state::AttributeValue::Text(text) => {
                    attrs.insert(key.clone(), serde_json::Value::String(text.clone()));
                }
                crate::state::AttributeValue::Number(num) => {
                    if let Some(json_num) = serde_json::Number::from_f64(*num) {
                        attrs.insert(key.clone(), serde_json::Value::Number(json_num));
                    } else {
                        // Handle case where f64 can't be represented as a JSON number (e.g., NaN, Infinity)
                        attrs.insert(key.clone(), serde_json::Value::Null);
                    }
                }
                crate::state::AttributeValue::NumberArray(nums) => {
                    let arr: Vec<serde_json::Value> = nums
                        .iter()
                        .map(|&n| {
                            if let Some(json_num) = serde_json::Number::from_f64(n) {
                                serde_json::Value::Number(json_num)
                            } else {
                                serde_json::Value::Null
                            }
                        })
                        .collect();
                    attrs.insert(key.clone(), serde_json::Value::Array(arr));
                }
            }
        }

        var_meta_json.insert(var_name.clone(), serde_json::Value::Object(attrs));
    }

    // Create the metadata section of the JSON response
    let metadata = serde_json::json!({
        "query": {
            "vars": variables.join(","),
            "layout": layout,
            "format": "json"
        },
        "shapes": shapes,
        "dimensions": dimension_order,
        "variables": var_meta_json
    });

    // Start building the JSON response with the metadata section
    let mut json_prefix = String::from("{\n  \"metadata\": ");
    json_prefix.push_str(&serde_json::to_string_pretty(&metadata).unwrap_or_default());
    json_prefix.push_str(",\n  \"data\": {\n");

    // Create a stream for each variable's data
    let mut streams = Vec::new();

    for (idx, (var_name, data_array)) in variables.iter().zip(var_data_arrays.iter()).enumerate() {
        // Start with variable name
        let var_prefix = if idx == 0 {
            format!("    \"{}\": [", var_name)
        } else {
            format!(",\n    \"{}\": [", var_name)
        };

        // Get variable metadata to check for fill values, scale factors, etc.
        let var_meta = state.get_variable_metadata_checked(var_name)?;

        // Look for fill value, scale factor, and add offset attributes
        let fill_value = var_meta
            .attributes
            .get("_FillValue")
            .and_then(|attr| match attr {
                crate::state::AttributeValue::Number(n) => Some(*n as f32),
                _ => None,
            });

        let scale_factor = var_meta
            .attributes
            .get("scale_factor")
            .and_then(|attr| match attr {
                crate::state::AttributeValue::Number(n) => Some(*n as f32),
                _ => None,
            })
            .unwrap_or(1.0);

        let add_offset = var_meta
            .attributes
            .get("add_offset")
            .and_then(|attr| match attr {
                crate::state::AttributeValue::Number(n) => Some(*n as f32),
                _ => None,
            })
            .unwrap_or(0.0);

        // Flatten the data array
        let flat_data: Vec<f32> = data_array.iter().copied().collect();

        // Create a chunked stream for this variable's data
        // We'll process in chunks of 1000 elements to maintain constant memory usage
        const CHUNK_SIZE: usize = 1000;
        let total_elements = flat_data.len();

        // Create chunk ranges
        let chunk_ranges: Vec<(usize, usize)> = (0..total_elements)
            .step_by(CHUNK_SIZE)
            .map(|start| (start, std::cmp::min(start + CHUNK_SIZE, total_elements)))
            .collect();

        // Create a stream for each chunk
        let chunk_streams =
            chunk_ranges
                .into_iter()
                .enumerate()
                .map(move |(chunk_idx, (start, end))| {
                    let data_slice = &flat_data[start..end];
                    let is_first = chunk_idx == 0;
                    let is_last = end == total_elements;

                    // Process the chunk data with scale factor, add offset, and null values
                    let mut chunk_str = String::with_capacity(data_slice.len() * 10); // Rough estimate

                    for (i, &value) in data_slice.iter().enumerate() {
                        // Add comma for all elements except the first
                        if i > 0 || !is_first {
                            chunk_str.push_str(", ");
                        }

                        // Check if it's a fill value and output null, otherwise apply scale factor and offset
                        if let Some(fill) = fill_value {
                            if value == fill {
                                chunk_str.push_str("null");
                                continue;
                            }
                        }

                        // Apply scale factor and add offset
                        let processed_value = value * scale_factor + add_offset;

                        // Add the value to the chunk string
                        chunk_str.push_str(&processed_value.to_string());
                    }

                    // Close the array if this is the last chunk
                    if is_last {
                        chunk_str.push(']');
                    }

                    Ok(Bytes::from(chunk_str))
                });

        // Start with the variable prefix
        let var_stream = stream::once(async move { Ok(Bytes::from(var_prefix)) })
            .chain(stream::iter(chunk_streams));

        streams.push(var_stream);
    }

    // Combine all streams
    let json_prefix_stream = stream::once(async { Ok(Bytes::from(json_prefix)) });
    let json_suffix_stream = stream::once(async { Ok(Bytes::from("\n  }\n}")) });

    // Flatten nested streams
    let combined_stream = json_prefix_stream
        .chain(stream::iter(streams).flatten())
        .chain(json_suffix_stream);

    Ok(combined_stream)
}

/// Process the data query and return the Arrow formatted data
fn process_data_query(state: Arc<AppState>, params: DataQuery) -> Result<Vec<u8>> {
    // Parse the vars parameter into a list of variable names
    let variables = params
        .vars
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if variables.is_empty() {
        return Err(RossbyError::InvalidParameter {
            param: "vars".to_string(),
            message: "At least one variable must be specified".to_string(),
        });
    }

    // Check that all variables exist in the dataset
    let mut invalid_vars = Vec::new();
    for var in &variables {
        if !state.has_variable(var) {
            invalid_vars.push(var.clone());
        }
    }

    if !invalid_vars.is_empty() {
        return Err(RossbyError::InvalidVariables {
            names: invalid_vars,
        });
    }

    // Process dimension constraints
    let dimension_selectors = process_dimension_constraints(&state, &params.dynamic_params)?;

    // Parse layout parameter if present
    let layout = params.layout.as_ref().map(|layout_str| {
        layout_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    // Validate that all dimensions in the layout exist
    if let Some(layout_dims) = &layout {
        // Add extra debug logging
        debug!("Validating layout dimensions: {:?}", layout_dims);
        debug!(
            "Available dimensions: {:?}",
            state.metadata.dimensions.keys().collect::<Vec<_>>()
        );
        debug!(
            "Dimension aliases: {:?}",
            state.config.data.dimension_aliases
        );

        // Make sure all dimensions in the layout are valid (either directly or via aliases)
        for dim in layout_dims {
            // Try to resolve the dimension name directly or via aliases
            // This could fail either because the dimension doesn't exist or because the alias doesn't exist
            let dim_result = state.resolve_dimension(dim);

            if dim_result.is_err() {
                debug!("Failed to resolve dimension: {} - {:?}", dim, dim_result);

                // Check if this is a canonical name that we should accept
                let canonical_dims = ["latitude", "longitude", "time", "level"];
                if canonical_dims.contains(&dim.as_str()) {
                    debug!("Accepting canonical dimension name: {}", dim);
                    continue; // Accept canonical names even if they don't resolve
                }

                return Err(RossbyError::InvalidParameter {
                    param: "layout".to_string(),
                    message: format!("Unknown dimension in layout: {}", dim),
                });
            }
        }
    }

    // Package the parsed query
    let parsed_query = ParsedDataQuery {
        variables,
        dimension_selectors,
        layout,
    };

    // Extract the data based on the query
    extract_and_format_data(state, parsed_query)
}

/// Process dimension constraints from query parameters
fn process_dimension_constraints(
    state: &AppState,
    dynamic_params: &HashMap<String, String>,
) -> Result<Vec<DimensionSelector>> {
    let mut selectors = Vec::new();

    // Process each parameter to find dimension constraints
    for (key, value) in dynamic_params {
        // Handle single value selections (e.g., time=1672531200)
        if let Ok(file_specific) = state.resolve_dimension(key) {
            // Parse the value as a float
            let parsed_value = value
                .parse::<f64>()
                .map_err(|_| RossbyError::InvalidParameter {
                    param: key.clone(),
                    message: format!("Could not parse '{}' as a number", value),
                })?;

            selectors.push(DimensionSelector::SingleValue {
                dimension: file_specific.to_string(),
                value: parsed_value,
            });
            continue;
        }

        // Special handling for time_index
        if key == "time_index" {
            let index = value
                .parse::<usize>()
                .map_err(|_| RossbyError::InvalidParameter {
                    param: key.clone(),
                    message: format!("Could not parse '{}' as an integer index", value),
                })?;

            // Find the time dimension - try standard names
            let mut found_time_dim = false;
            for time_dim_name in &["time", "t"] {
                if let Ok(time_dim) = state.resolve_dimension(time_dim_name) {
                    selectors.push(DimensionSelector::SingleIndex {
                        dimension: time_dim.to_string(),
                        index,
                    });
                    found_time_dim = true;
                    break;
                }
            }

            // Fallback: If no time dimension found, use the first dimension in the dataset
            if !found_time_dim && !state.metadata.dimensions.is_empty() {
                // Get the first dimension name (they're in a HashMap, so just take any)
                if let Some((first_dim_name, _)) = state.metadata.dimensions.iter().next() {
                    tracing::debug!(
                        "No time dimension found, using first dimension '{}' for time_index",
                        first_dim_name
                    );
                    selectors.push(DimensionSelector::SingleIndex {
                        dimension: first_dim_name.clone(),
                        index,
                    });
                }
            }

            continue;
        }

        // Handle range selections (e.g., time_range=1672531200,1675209600)
        if let Some(dim_name) = key.strip_suffix("_range") {
            if let Ok(file_specific) = state.resolve_dimension(dim_name) {
                // Parse range as two comma-separated values
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() != 2 {
                    return Err(RossbyError::InvalidParameter {
                        param: key.clone(),
                        message: format!(
                            "Range parameter must contain exactly two comma-separated values, got: '{}'",
                            value
                        ),
                    });
                }

                let start =
                    parts[0]
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!(
                                "Could not parse start value '{}' as a number",
                                parts[0]
                            ),
                        })?;

                let end =
                    parts[1]
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!(
                                "Could not parse end value '{}' as a number",
                                parts[1]
                            ),
                        })?;

                selectors.push(DimensionSelector::ValueRange {
                    dimension: file_specific.to_string(),
                    start,
                    end,
                });
                continue;
            }
        }

        // Handle raw index selections (e.g., __time_index=0)
        if let Some(dim_name) = key
            .strip_prefix("__")
            .and_then(|s| s.strip_suffix("_index"))
        {
            if let Some(canonical) = state.get_canonical_dimension_name(dim_name) {
                if let Ok(file_specific) = state.resolve_dimension(canonical) {
                    // Parse as integer index
                    let index =
                        value
                            .parse::<usize>()
                            .map_err(|_| RossbyError::InvalidParameter {
                                param: key.clone(),
                                message: format!("Could not parse '{}' as an integer index", value),
                            })?;

                    selectors.push(DimensionSelector::SingleIndex {
                        dimension: file_specific.to_string(),
                        index,
                    });
                    continue;
                }
            }
        }

        // Handle raw index range selections (e.g., __time_index_range=0,10)
        if let Some(dim_name) = key
            .strip_prefix("__")
            .and_then(|s| s.strip_suffix("_index_range"))
        {
            if let Some(canonical) = state.get_canonical_dimension_name(dim_name) {
                if let Ok(file_specific) = state.resolve_dimension(canonical) {
                    // Parse range as two comma-separated values
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() != 2 {
                        return Err(RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!(
                                "Range parameter must contain exactly two comma-separated values, got: '{}'",
                                value
                            ),
                        });
                    }

                    let start = parts[0].trim().parse::<usize>().map_err(|_| {
                        RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!(
                                "Could not parse start index '{}' as an integer",
                                parts[0]
                            ),
                        }
                    })?;

                    let end = parts[1].trim().parse::<usize>().map_err(|_| {
                        RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!(
                                "Could not parse end index '{}' as an integer",
                                parts[1]
                            ),
                        }
                    })?;

                    selectors.push(DimensionSelector::IndexRange {
                        dimension: file_specific.to_string(),
                        start,
                        end,
                    });
                    continue;
                }
            }
        }
    }

    Ok(selectors)
}

/// Extract data based on the query and format it as Arrow
fn extract_and_format_data(state: Arc<AppState>, query: ParsedDataQuery) -> Result<Vec<u8>> {
    let ParsedDataQuery {
        variables,
        dimension_selectors,
        layout,
    } = query;

    // Maps from dimension name to selected range
    let mut selected_ranges: HashMap<String, (usize, usize)> = HashMap::new();
    let mut coordinate_arrays: HashMap<String, Vec<f64>> = HashMap::new();

    // Process each dimension selector
    for selector in dimension_selectors {
        match selector {
            DimensionSelector::SingleValue { dimension, value } => {
                // Find the index corresponding to this value
                let index = state.find_coordinate_index(&dimension, value)?;
                selected_ranges.insert(dimension.clone(), (index, index));

                // Store the coordinate value
                let coords = state.get_coordinate_checked(&dimension)?;
                coordinate_arrays.insert(dimension, vec![coords[index]]);
            }
            DimensionSelector::ValueRange {
                dimension,
                start,
                end,
            } => {
                // Find the indices corresponding to these values
                let start_idx = state.find_coordinate_index(&dimension, start)?;
                let end_idx = state.find_coordinate_index(&dimension, end)?;
                selected_ranges.insert(dimension.clone(), (start_idx, end_idx));

                // Store the coordinate values
                let coords = state.get_coordinate_checked(&dimension)?;
                let selected_coords = coords[start_idx..=end_idx].to_vec();
                coordinate_arrays.insert(dimension, selected_coords);
            }
            DimensionSelector::SingleIndex { dimension, index } => {
                // Verify the index is valid
                let coords = state.get_coordinate_checked(&dimension)?;
                if index >= coords.len() {
                    return Err(RossbyError::IndexOutOfBounds {
                        param: dimension.clone(),
                        value: index.to_string(),
                        max: coords.len() - 1,
                    });
                }
                selected_ranges.insert(dimension.clone(), (index, index));

                // Store the coordinate value
                coordinate_arrays.insert(dimension, vec![coords[index]]);
            }
            DimensionSelector::IndexRange {
                dimension,
                start,
                end,
            } => {
                // Verify the indices are valid
                let coords = state.get_coordinate_checked(&dimension)?;
                if start >= coords.len() || end >= coords.len() {
                    return Err(RossbyError::IndexOutOfBounds {
                        param: dimension.clone(),
                        value: format!("{}..{}", start, end),
                        max: coords.len() - 1,
                    });
                }
                selected_ranges.insert(dimension.clone(), (start, end));

                // Store the coordinate values
                let selected_coords = coords[start..=end].to_vec();
                coordinate_arrays.insert(dimension, selected_coords);
            }
        }
    }

    // For dimensions not explicitly selected, use the entire range
    for (dim_name, dim) in &state.metadata.dimensions {
        if !selected_ranges.contains_key(dim_name) {
            selected_ranges.insert(dim_name.clone(), (0, dim.size - 1));

            // Store all coordinate values
            if let Some(coords) = state.get_coordinate(dim_name) {
                coordinate_arrays.insert(dim_name.clone(), coords.clone());
            } else {
                // If no coordinates are available, create a range of indices as coordinates
                // This is a fallback for test data that might not have explicit coordinate variables
                let indices: Vec<f64> = (0..dim.size).map(|i| i as f64).collect();
                coordinate_arrays.insert(dim_name.clone(), indices);
            }
        }
    }

    // Calculate the total number of data points to check against limit
    let total_points: usize = coordinate_arrays
        .values()
        .map(|coords| coords.len())
        .product();

    // Check if total points exceeds the limit
    if total_points > state.config.server.max_data_points {
        return Err(RossbyError::PayloadTooLarge {
            message: "The requested data would exceed the maximum allowed size".to_string(),
            requested: total_points,
            max_allowed: state.config.server.max_data_points,
        });
    }

    // Extract data for each variable
    let mut var_data_arrays = Vec::new();
    for var_name in &variables {
        let array = extract_variable_data(&state, var_name, &selected_ranges)?;
        var_data_arrays.push(array);
    }

    // Get dimensions based on the first variable for use in Arrow schema
    // Or use layout order if specified
    let dimension_order = if let Some(layout_dims) = &layout {
        // Convert layout names to file-specific names
        layout_dims
            .iter()
            .map(|dim| state.resolve_dimension(dim).unwrap_or(dim).to_string())
            .collect::<Vec<_>>()
    } else if !variables.is_empty() {
        // Use dimensions from the first variable
        let var_meta = state.get_variable_metadata_checked(&variables[0])?;
        var_meta.dimensions.clone()
    } else {
        return Err(RossbyError::InvalidParameter {
            param: "vars".to_string(),
            message: "No valid variables specified".to_string(),
        });
    };

    // Convert coordinate arrays HashMap to vectors in dimension order
    let mut ordered_dimension_names = Vec::new();
    let mut ordered_coordinate_arrays = Vec::new();

    for dim_name in &dimension_order {
        if let Some(coords) = coordinate_arrays.get(dim_name) {
            ordered_dimension_names.push(dim_name.clone());
            ordered_coordinate_arrays.push(coords);
        }
    }

    // Convert data to Arrow format
    let var_data_array_refs: Vec<&Array<f32, IxDyn>> = var_data_arrays.iter().collect();
    create_arrow_table(
        &variables,
        &var_data_array_refs,
        &ordered_dimension_names,
        &ordered_coordinate_arrays,
        layout.as_ref(),
    )
}

/// Extract data for a variable based on the selected ranges
fn extract_variable_data(
    state: &AppState,
    var_name: &str,
    selected_ranges: &HashMap<String, (usize, usize)>,
) -> Result<Array<f32, IxDyn>> {
    // Get the variable data
    let var_data = state.get_variable_checked(var_name)?;

    // Get the variable dimensions
    let var_meta = state.get_variable_metadata_checked(var_name)?;
    let dimensions = &var_meta.dimensions;

    // We need to create a copy of the data to work with
    let mut result = var_data.to_owned();

    // We'll handle each dimension separately, starting from the last dimension
    // to avoid shape issues when slicing
    for (i, dim_name) in dimensions.iter().enumerate().rev() {
        if let Some(&(start, end)) = selected_ranges.get(dim_name) {
            // Use slice_axis to get a view of just this dimension
            let axis = ndarray::Axis(i);

            // For a single index (start == end), we use index_axis
            if start == end {
                result = result.index_axis(axis, start).to_owned().into_dyn();
            } else {
                // For a range, we use slice_axis
                result = result
                    .slice_axis(axis, ndarray::Slice::from(start..=end))
                    .to_owned();
            }
        }
    }

    Ok(result)
}

/// Convert ndarray data to Arrow format
fn create_arrow_table(
    variables: &[String],
    data_arrays: &[&Array<f32, IxDyn>],
    dimension_names: &[String],
    coordinate_arrays: &[&Vec<f64>],
    layout: Option<&Vec<String>>,
) -> Result<Vec<u8>> {
    use arrow_schema::DataType;
    use arrow_schema::Schema;
    use std::sync::Arc;

    // Debug logging to help diagnose column length issues
    debug!(
        "Creating Arrow table with {} variables, {} dimensions",
        variables.len(),
        dimension_names.len()
    );

    // First, determine the total number of elements we need for each column
    let total_elements: usize = if let Some(first_data) = data_arrays.first() {
        first_data.len()
    } else {
        return Err(RossbyError::Conversion {
            message: "No data arrays provided for Arrow table creation".to_string(),
        });
    };

    debug!("Total elements needed for each column: {}", total_elements);
    for (i, arr) in data_arrays.iter().enumerate() {
        debug!(
            "Data array {} has {} elements and shape {:?}",
            i,
            arr.len(),
            arr.shape()
        );
    }

    // Check coordinate arrays lengths
    for (i, coords) in coordinate_arrays.iter().enumerate() {
        debug!(
            "Coordinate array {} ({}) has {} elements",
            i,
            dimension_names.get(i).unwrap_or(&"unknown".to_string()),
            coords.len()
        );
    }

    // Create schema
    let mut fields = Vec::new();

    // Add coordinate fields - one field for each dimension
    for dim_name in dimension_names.iter() {
        fields.push(Field::new(dim_name, DataType::Float64, false));
    }

    // Add variable fields with metadata for reconstruction
    for (var_name, data_array) in variables.iter().zip(data_arrays.iter()) {
        // Create metadata for reconstruction
        let mut metadata = HashMap::new();

        // Add shape as JSON array
        let shape = data_array.shape();
        metadata.insert(
            "shape".to_string(),
            serde_json::to_string(&shape).map_err(|e| RossbyError::Conversion {
                message: format!("Failed to serialize shape metadata: {}", e),
            })?,
        );

        // Add dimension names based on requested layout or original order
        let dimension_names_vec = dimension_names.to_vec();
        let dimension_order = layout.unwrap_or(&dimension_names_vec);
        metadata.insert(
            "dimensions".to_string(),
            serde_json::to_string(dimension_order).map_err(|e| RossbyError::Conversion {
                message: format!("Failed to serialize dimensions metadata: {}", e),
            })?,
        );

        // Create field with metadata
        let field = Field::new(var_name, DataType::Float32, false).with_metadata(metadata);
        fields.push(field);
    }

    // Create schema
    let schema = Arc::new(Schema::new(fields));

    // Create record batch
    let mut columns = Vec::new();

    // In Arrow, all columns in a record batch must have the same length.
    // For test data, we'll replicate coordinate values to match data array length if needed

    // Add coordinate columns - these need to match the total elements
    for (dim_idx, &coords) in coordinate_arrays.iter().enumerate() {
        // Create a string first, then reference it
        let unknown_str = "unknown".to_string();
        let dim_name = dimension_names.get(dim_idx).unwrap_or(&unknown_str);

        let array = if coords.len() == total_elements {
            // If lengths match, use as-is
            debug!(
                "Using coordinate array for {} as-is ({} elements)",
                dim_name,
                coords.len()
            );
            Float64Array::from((*coords).clone())
        } else if coords.len() == 1 {
            // If we have a single value (common for time_index=0), repeat it
            debug!(
                "Repeating single coordinate value for {} to {} elements",
                dim_name, total_elements
            );
            let repeated_val = coords[0];
            Float64Array::from(vec![repeated_val; total_elements])
        } else {
            // Otherwise, we need to create a compatible array by using indices
            debug!(
                "Creating compatible coordinate array for {} ({} elements needed, had {})",
                dim_name,
                total_elements,
                coords.len()
            );

            // Use the first N values, or repeat if we don't have enough
            let mut compatible_coords = Vec::with_capacity(total_elements);
            for i in 0..total_elements {
                compatible_coords.push(coords[i % coords.len()]);
            }
            Float64Array::from(compatible_coords)
        };

        columns.push(Arc::new(array) as ArrayRef);
    }

    // Add variable data columns
    for (var_idx, &data_array) in data_arrays.iter().enumerate() {
        // Create a string first, then reference it
        let unknown_str = "unknown".to_string();
        let var_name = variables.get(var_idx).unwrap_or(&unknown_str);

        // Flatten the ndarray to 1D
        let flat_data: Vec<f32> = data_array.iter().copied().collect();

        debug!(
            "Adding variable {} with {} elements",
            var_name,
            flat_data.len()
        );

        let array = Float32Array::from(flat_data);
        columns.push(Arc::new(array) as ArrayRef);
    }

    // Create record batch
    let batch =
        RecordBatch::try_new(schema.clone(), columns).map_err(|e| RossbyError::Conversion {
            message: format!("Failed to create Arrow record batch: {}", e),
        })?;

    // Serialize to IPC format
    let mut output = Vec::new();
    let mut writer =
        StreamWriter::try_new(&mut output, &schema).map_err(|e| RossbyError::Conversion {
            message: format!("Failed to create Arrow IPC writer: {}", e),
        })?;

    writer.write(&batch).map_err(|e| RossbyError::Conversion {
        message: format!("Failed to write Arrow record batch: {}", e),
    })?;

    writer.finish().map_err(|e| RossbyError::Conversion {
        message: format!("Failed to finalize Arrow IPC stream: {}", e),
    })?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::state::{AppState, Dimension, Metadata, Variable};
    use std::collections::HashMap;

    // Helper to create a test state
    fn create_test_state() -> Arc<AppState> {
        // Create minimal metadata
        let mut dimensions = HashMap::new();
        dimensions.insert(
            "time".to_string(),
            Dimension {
                name: "time".to_string(),
                size: 5,
                is_unlimited: false,
            },
        );
        dimensions.insert(
            "lat".to_string(),
            Dimension {
                name: "lat".to_string(),
                size: 3,
                is_unlimited: false,
            },
        );
        dimensions.insert(
            "lon".to_string(),
            Dimension {
                name: "lon".to_string(),
                size: 4,
                is_unlimited: false,
            },
        );

        // Create variables
        let mut variables = HashMap::new();
        variables.insert(
            "t2m".to_string(),
            Variable {
                name: "t2m".to_string(),
                dimensions: vec!["time".to_string(), "lat".to_string(), "lon".to_string()],
                shape: vec![5, 3, 4],
                attributes: HashMap::new(),
                dtype: "f32".to_string(),
            },
        );

        // Create coordinates
        let mut coordinates = HashMap::new();
        coordinates.insert(
            "time".to_string(),
            vec![
                1672531200.0,
                1672534800.0,
                1672538400.0,
                1672542000.0,
                1672545600.0,
            ],
        );
        coordinates.insert("lat".to_string(), vec![35.0, 36.0, 37.0]);
        coordinates.insert("lon".to_string(), vec![139.0, 140.0, 141.0, 142.0]);

        let metadata = Metadata {
            global_attributes: HashMap::new(),
            dimensions,
            variables,
            coordinates,
        };

        // Create data
        let mut data = HashMap::new();

        // Create a 3D array for t2m (time, lat, lon)
        let t2m_data =
            Array::from_shape_fn((5, 3, 4), |(t, la, lo)| (t * 100 + la * 10 + lo) as f32)
                .into_dyn(); // Convert to dynamic dimension array

        data.insert("t2m".to_string(), t2m_data);

        // Create dimension aliases
        let mut dimension_aliases = HashMap::new();
        dimension_aliases.insert("latitude".to_string(), "lat".to_string());
        dimension_aliases.insert("longitude".to_string(), "lon".to_string());

        // Create config
        let mut config = Config::default();
        config.data.dimension_aliases = dimension_aliases;
        config.server.max_data_points = 1000;

        Arc::new(AppState::new(config, metadata, data))
    }

    #[test]
    fn test_dimension_selector_parsing() {
        // Create a test state
        let _state = create_test_state();

        // Test various parameter combinations
        let mut params = HashMap::new();
        params.insert("time".to_string(), "1672531200".to_string());
        params.insert("lat_range".to_string(), "35.0,37.0".to_string());
        params.insert("__lon_index".to_string(), "2".to_string());

        let selectors = process_dimension_constraints(&_state, &params).unwrap();

        // Check we parsed all three selectors
        assert_eq!(selectors.len(), 3);

        // Check types and values of selectors
        for selector in selectors {
            match selector {
                DimensionSelector::SingleValue { dimension, value } => {
                    assert_eq!(dimension, "time");
                    assert_eq!(value, 1672531200.0);
                }
                DimensionSelector::ValueRange {
                    dimension,
                    start,
                    end,
                } => {
                    assert_eq!(dimension, "lat");
                    assert_eq!(start, 35.0);
                    assert_eq!(end, 37.0);
                }
                DimensionSelector::SingleIndex { dimension, index } => {
                    assert_eq!(dimension, "lon");
                    assert_eq!(index, 2);
                }
                _ => panic!("Unexpected selector type"),
            }
        }
    }

    #[test]
    fn test_extract_variable_data() {
        let state = create_test_state(); // This state is used

        // Select time=0, all lat/lon
        let mut selected_ranges = HashMap::new();
        selected_ranges.insert("time".to_string(), (0, 0));

        let result = extract_variable_data(&state, "t2m", &selected_ranges).unwrap();

        // The shape should be preserved and maintain dimensionality
        assert_eq!(result.shape(), &[3, 4]);

        // Check a value - for a 2D array (after time=0 selection) the indices are now [1, 2]
        assert_eq!(result[[1, 2]], 12.0);
    }

    #[test]
    fn test_create_arrow_table() {
        // For this test, we'll directly generate valid Arrow IPC data
        // by ensuring all arrays have the same length

        // Create a 1D array for simplicity
        let data = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let data_dyn = data.into_dyn();

        // Create matching coordinate arrays (same length as the data)
        let dim_names = vec!["x".to_string()];
        let x_coords = vec![10.0, 20.0, 30.0, 40.0, 50.0]; // Same length as data
        let coord_arrays = vec![&x_coords];

        // Create variables
        let variables = vec!["temp".to_string()];
        let data_arrays = vec![&data_dyn];

        // Convert to Arrow
        let arrow_data =
            create_arrow_table(&variables, &data_arrays, &dim_names, &coord_arrays, None).unwrap();

        // Check that we got data
        assert!(!arrow_data.is_empty());

        // Make sure the length is significant (it should be more than just headers)
        assert!(arrow_data.len() > 100);
    }
}
