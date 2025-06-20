# Feature: Data Access with Apache Arrow

**Labels:** `enhancement`, `feature`, `api-design`, `performance`, `mle`, `high-priority`

## 1. Problem Statement

While `rossby` currently excels at providing interactive single-point queries (`/point`) and 2D data visualizations (`/image`), it lacks a core capability: the ability to **efficiently extract large-scale, multi-dimensional structured data subsets**.

For programmatic use cases like scientific computing and Machine Learning (ML) model training, which require large volumes of raw numerical data, using JSON for data transfer introduces significant serialization/deserialization overhead and network payload, failing to meet high-performance requirements. To position `rossby` as a bridge between raw scientific data and modern data science workflows, a solution designed for high-throughput data transfer is needed.

## 2. Proposed Solution

We propose the introduction of a new endpoint: `GET /data`.

The core mission of this endpoint is to stream user-defined, N-dimensional data hyperslabs directly in the **Apache Arrow** format. Apache Arrow is an in-memory columnar data format designed for high-performance analytics, serving as the lingua franca for the modern data science (Pandas, Polars, DuckDB) and machine learning (PyTorch, TensorFlow) ecosystems.

This feature will provide users with a near-zero-copy, analytics-ready data stream, enabling them to seamlessly integrate `rossby` into their data processing and MLOps workflows. Key features include:

* Flexible subsetting of data hyperslabs via physical value or index ranges.
* The ability for users to specify the output data's dimension layout to match the requirements of downstream computational frameworks.
* Adoption of a "raw structured" data model to prioritize maximum server throughput and network efficiency.

## 3. API Endpoint Specification

### `GET /data`

#### Query Parameters

* `vars=<variables>`
    * **Description**: (Required) A comma-separated list of one or more variables to extract.
    * **Example**: `vars=t2m,u10`

* **Dimension Selectors**: For each dimension in the data (e.g., `time`, `latitude`, `level`), the user can constrain it using one of the following methods. If a dimension is not constrained, its entire range is selected by default.
    * `<dim_name>=<value>`: Select a single slice by physical value.
        * **Example**: `level=850`
    * `<dim_name>_range=<start_value>,<end_value>`: Select a closed interval range by physical values.
        * **Example**: `time_range=1672531200,1675209600`
    * `__<canonical_name>_index=<index>`: Select a single slice by its raw index.
        * **Example**: `__time_index=0`
    * `__<canonical_name>_index_range=<start_index>,<end_index>`: Select a closed interval range by raw indices.
        * **Example**: `__latitude_index_range=10,50`

* `layout=<dimension_order>`
    * **Description**: (Optional) A comma-separated string specifying the desired dimension order for the output N-dimensional data array.
    * **Default**: If omitted, the native dimension order from the source NetCDF file is used.
    * **Example**: `layout=time,longitude,latitude`

#### Response

* **`Content-Type`**: `application/vnd.apache.arrow.stream`
* **Body**: A streaming binary representation of an Apache Arrow **Table**, with a schema defined as follows:
    * **Coordinate Columns**: For each dimension included in the query, a corresponding 1D column containing its coordinate values (e.g., a `time` column, a `latitude` column).
    * **Variable Columns**: For each variable requested in `vars`, a corresponding column (e.g., a `t2m` column).
        * The data in this column is a **flattened 1D array** containing all the numerical values from the selected data hyperslab.
        * The **metadata** map of this column's field **must** contain the following two keys to allow for client-side reconstruction of the N-dimensional array:
            * `shape`: A JSON string array representing the data's shape. Example: `"[10, 90, 45]"`.
            * `dimensions`: A JSON string array listing the dimension names in their specified order, matching the `layout` parameter. Example: `"['time', 'longitude', 'latitude']"`.

## 4. Core Concepts & Design Rationale

* **Raw Structured Data**: We chose not to reshape the data into a "Tidy Data" format on the server. This decision maximizes server-side processing speed and throughput while minimizing the network payload size. It offloads the final data reconstruction task to client libraries (e.g., `xarray`), which are highly optimized for this and know the user's ultimate data structure requirements best.
* **User-Defined Layout**: The `layout` parameter is a critical feature. Different computational frameworks have different expectations for the dimension order of input tensors. Providing this functionality eliminates the need for users to perform error-prone `permute`/`transpose` operations on the client side, significantly improving the ease of integration with ML/DL frameworks.

## 5. Usage Examples

**Example 1: Get a 2D spatial slice for a single time step**
```bash
# Get t2m data for all lat/lon at a specific timestamp
curl "http://127.0.0.1:8000/data?vars=t2m&time=1672531200" -o data.arrow
```

**Example 2: Get a 3D data cube over a spatio-temporal range**
```bash
# Get u10 and v10 variables for all levels within a specified time and lat/lon range
curl "http://127.0.0.1:8000/data?vars=u10,v10&time_range=...&lat_range=30,40&lon_range=130,140" -o data.arrow
```

**Example 3: Get data with a custom layout for PyTorch**
```bash
# Request a specific dimension layout to match PyTorch's convolution input requirements
curl "http://127.0.0.1:8000/data?vars=t2m&level=500&layout=time,latitude,longitude" -o data.arrow
```

## 6. Safeguards and Considerations

To prevent clients from requesting excessively large datasets that could exhaust server memory or saturate the network, `rossby` **must** implement a configurable **request size limit**.

* **Mechanism**: Before processing a request, the server will calculate the total number of data points to be returned based on the selected dimension ranges (`total_points = num_times * num_lats * num_lons ...`).
* **Behavior**: If the calculated number of points exceeds the configured threshold (e.g., `10,000,000`), the server should reject the request and return a `413 Payload Too Large` or `400 Bad Request` status code with a clear, explanatory error message advising the user to narrow their query.

## 7. Benefits

* **Extreme Performance**: Achieves high-throughput data transfer far exceeding JSON by using the binary Arrow format.
* **Seamless Ecosystem Integration**: Becomes a first-class citizen in the modern data science and ML ecosystem, with data consumable by Pandas, PyTorch, and other tools.
* **Enables MLOps Workflows**: Functions as a high-performance online feature server, providing data directly for model training and inference.
* **Flexibility and Control**: Provides users with fine-grained control to extract the exact data subset and dimension layout they need via a rich set of query parameters.

## 8. Implementation

### 8.1. Dependencies

The Apache Arrow integration requires adding the following dependencies to `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
arrow = "26.0.0"
arrow-array = "26.0.0"
arrow-schema = "26.0.0"
arrow-ipc = "26.0.0"
```

### 8.2. Query Parameter Handling

The endpoint will parse query parameters using a dedicated struct:

```rust
/// Query parameters for the data endpoint
#[derive(Debug, Deserialize)]
pub struct DataQuery {
    /// Comma-separated list of variables to extract
    pub vars: String,
    
    /// Optional layout specification (comma-separated dimension names)
    #[serde(default)]
    pub layout: Option<String>,
    
    /// Dynamic parameters - will be parsed separately
    #[serde(flatten)]
    pub dynamic_params: HashMap<String, String>,
}
```

Additional structs to handle the dimensional constraints:

```rust
/// Represents a dimension selection constraint
#[derive(Debug, Clone)]
pub enum DimensionSelector {
    /// Select a single slice by physical value
    SingleValue {
        dimension: String,
        value: f64,
    },
    /// Select a range by physical values (inclusive)
    ValueRange {
        dimension: String,
        start: f64,
        end: f64,
    },
    /// Select a single slice by raw index
    SingleIndex {
        dimension: String,
        index: usize,
    },
    /// Select a range by raw indices (inclusive)
    IndexRange {
        dimension: String,
        start: usize,
        end: usize,
    },
}

/// Parsed query information
pub struct ParsedDataQuery {
    /// List of variable names to extract
    pub variables: Vec<String>,
    
    /// Dimension constraints
    pub dimension_selectors: Vec<DimensionSelector>,
    
    /// Requested dimension order
    pub layout: Option<Vec<String>>,
}
```

### 8.3. Handler Implementation

The data handler will be implemented in `src/handlers/data.rs`:

```rust
/// Handle GET /data requests
pub async fn data_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DataQuery>,
) -> Response {
    let request_id = generate_request_id();
    let start_time = Instant::now();

    // Log request parameters
    debug!(
        endpoint = "/data",
        request_id = %request_id,
        vars = %params.vars,
        layout = ?params.layout,
        "Processing data query"
    );

    match process_data_query(state, params.clone()) {
        Ok(arrow_data) => {
            // Log successful request
            let duration = start_time.elapsed();
            info!(
                endpoint = "/data",
                request_id = %request_id,
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
        Err(error) => {
            // Log error
            log_request_error(
                &error,
                "/data",
                &request_id,
                Some(&format!("vars={}", params.vars)),
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
    }
}
```

### 8.4. Dimension Range Processing

The implementation will need to process dimension constraints:

```rust
/// Process dimension constraints from query parameters
fn process_dimension_constraints(
    state: &AppState,
    dynamic_params: &HashMap<String, String>,
) -> Result<Vec<DimensionSelector>, RossbyError> {
    let mut selectors = Vec::new();

    // Process each parameter to find dimension constraints
    for (key, value) in dynamic_params {
        // Handle single value selections (e.g., time=1672531200)
        if let Some(file_specific) = state.resolve_dimension(key).ok() {
            // Parse the value as a float
            let parsed_value = value.parse::<f64>().map_err(|_| {
                RossbyError::InvalidParameter {
                    param: key.clone(),
                    message: format!("Could not parse '{}' as a number", value),
                }
            })?;

            selectors.push(DimensionSelector::SingleValue {
                dimension: file_specific.to_string(),
                value: parsed_value,
            });
            continue;
        }

        // Handle range selections (e.g., time_range=1672531200,1675209600)
        if let Some(dim_name) = key.strip_suffix("_range") {
            if let Some(file_specific) = state.resolve_dimension(dim_name).ok() {
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

                let start = parts[0].trim().parse::<f64>().map_err(|_| {
                    RossbyError::InvalidParameter {
                        param: key.clone(),
                        message: format!("Could not parse start value '{}' as a number", parts[0]),
                    }
                })?;

                let end = parts[1].trim().parse::<f64>().map_err(|_| {
                    RossbyError::InvalidParameter {
                        param: key.clone(),
                        message: format!("Could not parse end value '{}' as a number", parts[1]),
                    }
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
        if let Some(dim_name) = key.strip_prefix("__").and_then(|s| s.strip_suffix("_index")) {
            if let Some(canonical) = state.get_canonical_dimension_name(dim_name) {
                if let Some(file_specific) = state.resolve_dimension(canonical).ok() {
                    // Parse as integer index
                    let index = value.parse::<usize>().map_err(|_| {
                        RossbyError::InvalidParameter {
                            param: key.clone(),
                            message: format!("Could not parse '{}' as an integer index", value),
                        }
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
                if let Some(file_specific) = state.resolve_dimension(canonical).ok() {
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
                            message: format!("Could not parse end index '{}' as an integer", parts[1]),
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
```

### 8.5. Arrow Table Construction

The core of the implementation converts the selected data slices to Arrow format:

```rust
/// Convert ndarray data to Arrow format
fn create_arrow_table(
    variables: &[String],
    data_arrays: &[&Array<f32, IxDyn>],
    dimension_names: &[String],
    coordinate_arrays: &[&Vec<f64>],
    layout: Option<&Vec<String>>,
) -> Result<Vec<u8>, RossbyError> {
    // Create schema
    let mut fields = Vec::new();
    
    // Add coordinate fields
    for (dim_name, coord_values) in dimension_names.iter().zip(coordinate_arrays.iter()) {
        fields.push(Field::new(
            dim_name,
            DataType::Float64,
            false,
        ));
    }
    
    // Add variable fields with metadata for reconstruction
    for (var_name, data_array) in variables.iter().zip(data_arrays.iter()) {
        // Calculate the shape and total elements
        let shape = data_array.shape();
        let total_elements = shape.iter().product::<usize>();
        
        // Create metadata for reconstruction
        let mut metadata = HashMap::new();
        
        // Add shape as JSON array
        metadata.insert(
            "shape".to_string(),
            serde_json::to_string(&shape).map_err(|e| {
                RossbyError::Conversion {
                    message: format!("Failed to serialize shape metadata: {}", e),
                }
            })?,
        );
        
        // Add dimension names based on requested layout or original order
        let dimension_order = layout.unwrap_or(&dimension_names.to_vec());
        metadata.insert(
            "dimensions".to_string(),
            serde_json::to_string(dimension_order).map_err(|e| {
                RossbyError::Conversion {
                    message: format!("Failed to serialize dimensions metadata: {}", e),
                }
            })?,
        );
        
        // Create field with metadata
        fields.push(Field::new_with_metadata(
            var_name,
            DataType::Float32,
            false,
            metadata,
        ));
    }
    
    // Create schema
    let schema = Arc::new(Schema::new(fields));
    
    // Create record batch
    let mut columns = Vec::new();
    
    // Add coordinate columns
    for coord_values in coordinate_arrays {
        let array = Float64Array::from(coord_values.clone());
        columns.push(Arc::new(array) as ArrayRef);
    }
    
    // Add variable data columns
    for data_array in data_arrays {
        // Flatten the ndarray to 1D
        let flat_data: Vec<f32> = data_array.iter().copied().collect();
        let array = Float32Array::from(flat_data);
        columns.push(Arc::new(array) as ArrayRef);
    }
    
    // Create record batch
    let batch = RecordBatch::try_new(schema.clone(), columns).map_err(|e| {
        RossbyError::Conversion {
            message: format!("Failed to create Arrow record batch: {}", e),
        }
    })?;
    
    // Serialize to IPC format
    let mut output = Vec::new();
    let mut writer = StreamWriter::try_new(&mut output, &schema).map_err(|e| {
        RossbyError::Conversion {
            message: format!("Failed to create Arrow IPC writer: {}", e),
        }
    })?;
    
    writer.write(&batch).map_err(|e| {
        RossbyError::Conversion {
            message: format!("Failed to write Arrow record batch: {}", e),
        }
    })?;
    
    writer.finish().map_err(|e| {
        RossbyError::Conversion {
            message: format!("Failed to finalize Arrow IPC stream: {}", e),
        }
    })?;
    
    Ok(output)
}
```

### 8.6. Error Handling Enhancements

A new error type would be added to `error.rs` to handle payload size limitations:

```rust
/// Payload too large error
#[error("Payload too large: {message}. Requested points: {requested}, maximum allowed: {max_allowed}")]
PayloadTooLarge {
    message: String,
    requested: usize,
    max_allowed: usize,
},
```

### 8.7. Server Configuration

Additional configuration options for the data endpoint:

```rust
/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    // Existing fields...
    
    /// Maximum number of data points allowed in a single data request
    #[serde(default = "default_max_data_points")]
    pub max_data_points: usize,
}

fn default_max_data_points() -> usize {
    10_000_000 // 10 million points default
}
```

## 9. Testing

### 9.1. Unit Tests

Unit tests will cover the parameter parsing and Arrow serialization logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dimension_selector_parsing() {
        // Create a test state
        let state = create_test_state();
        
        // Test various parameter combinations
        let mut params = HashMap::new();
        params.insert("time".to_string(), "1672531200".to_string());
        params.insert("lat_range".to_string(), "30.0,40.0".to_string());
        params.insert("__lon_index".to_string(), "5".to_string());
        
        let selectors = process_dimension_constraints(&state, &params).unwrap();
        
        // Verify the selectors
        assert_eq!(selectors.len(), 3);
        // Validate each selector type and values
        // ...
    }
    
    #[test]
    fn test_arrow_table_creation() {
        // Test with a simple 2D array
        // ...
    }
    
    #[test]
    fn test_dimension_layout_reordering() {
        // Test reordering dimensions based on layout parameter
        // ...
    }
    
    #[test]
    fn test_payload_size_limits() {
        // Test the payload size limitation logic
        // ...
    }
}
```

### 9.2. Integration Tests

Integration tests in `tests/integration_test.rs` will verify the endpoint behavior:

```rust
#[tokio::test]
async fn test_data_endpoint() {
    // Set up test server with sample data
    let app = create_test_app().await;
    
    // Test basic query
    let response = app
        .oneshot(
            Request::builder()
                .uri("/data?vars=t2m&time=1672531200")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.apache.arrow.stream"
    );
    
    // Verify the Arrow data
    // ...
    
    // Test query with multiple variables and dimension ranges
    // ...
    
    // Test payload too large error
    // ...
}
```

### 9.3. Benchmarks

Performance benchmarks will be added to measure throughput:

```rust
// In benches/data_endpoint.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_data_endpoint(c: &mut Criterion) {
    // Set up test data
    // ...
    
    c.bench_function("data_endpoint_small", |b| {
        b.iter(|| {
            // Process a small data request
            // ...
        })
    });
    
    c.bench_function("data_endpoint_large", |b| {
        b.iter(|| {
            // Process a large data request
            // ...
        })
    });
}

criterion_group!(benches, bench_data_endpoint);
criterion_main!(benches);
```

## 10. Conclusion

The `/data` endpoint with Apache Arrow integration significantly enhances Rossby's capabilities for data science and machine learning workflows. By providing efficient, structured data access, it bridges the gap between the raw scientific data stored in NetCDF files and modern analytics frameworks.

This implementation follows Rossby's design principles of adapting to the data, maintaining high performance, and providing a consistent, well-documented API. The Arrow format ensures maximum interoperability with downstream tools while minimizing data transfer overhead.
