# Feature: Unified Data Access Endpoint (`/data`)

**Labels:** `enhancement`, `feature`, `api-design`, `performance`, `mle`, `web-api`, `high-priority`

## 1\. Problem Statement

While `rossby` excels at providing interactive single-point queries (`/point`) and 2D data visualizations (`/image`), its ability to serve as a bridge to other applications hinges on its data extraction capabilities. Two primary, distinct use cases have emerged:

1.  **High-Performance Analytics**: For scientific computing and Machine Learning (ML), users need to efficiently extract large-scale, multi-dimensional data subsets into environments like Pandas or PyTorch. Using JSON for this purpose introduces significant performance overhead.

2.  **Web & Lightweight Integration**: For web frontends (like `rossby-vis`) and simple scripts, a direct, human-readable, and browser-friendly format like JSON is essential. These clients should not be required to implement complex binary protocol readers or handle low-level data unpacking (e.g., `scale_factor`, `add_offset`).

A unified data endpoint is needed to serve both use cases without compromising the principles of either, **especially when dealing with data volumes that exceed the server's available memory.**

## 2\. Proposed Solution

We propose to enhance the `GET /data` endpoint by introducing a `format` query parameter. This will allow the endpoint to serve data in two distinct modes, catering to different consumer needs while maintaining a single, clean API interface.

The cornerstone of this feature for large datasets is its **streaming-first architecture**. All large data responses are sent to the client piece-by-piece using **HTTP's `Transfer-Encoding: chunked` (分块传输编码)**. This ensures constant, low memory usage on the server, regardless of the requested data size, preventing memory exhaustion.

* `format=arrow` (Default): For high-performance use cases, the endpoint will stream user-defined, N-dimensional data hyperslabs directly in the **Apache Arrow** format. This provides a near-zero-copy, analytics-ready stream for the modern data science ecosystem.

* `format=json`: For web frontends and general-purpose clients, the endpoint will return a well-structured JSON object. In this mode, `rossby` will perform all necessary data processing on the server-side, including **unpacking data** (applying `scale_factor` and `add_offset`) and **handling missing values** (converting `_FillValue` to `null`), delivering data that is immediately ready for consumption.

## 3\. API Endpoint Specification

### `GET /data`

#### Query Parameters

* `vars=<variables>`

    * **Description**: (Required) A comma-separated list of one or more variables to extract.
    * **Example**: `vars=t2m,u10`

* **Dimension Selectors**: For each dimension in the data (e.g., `time`, `latitude`, `level`), the user can constrain it using one of the following methods. If a dimension is not constrained, its entire range is selected by default.

    * `<dim_name>=<value>`: Select a single slice by physical value. (e.g., `level=850`)
    * `<dim_name>_range=<start_value>,<end_value>`: Select a closed interval range by physical values. (e.g., `time_range=1672531200,1675209600`)
    * `__<canonical_name>_index=<index>`: Select a single slice by its raw index. (e.g., `__time_index=0`)
    * `__<canonical_name>_index_range=<start_index>,<end_index>`: Select a closed interval range by raw indices. (e.g., `__latitude_index_range=10,50`)

* `layout=<dimension_order>`

    * **Description**: (Optional) A comma-separated string specifying the desired dimension order for the output N-dimensional data array.
    * **Default**: If omitted, the native dimension order from the source NetCDF file is used.
    * **Example**: `layout=time,longitude,latitude`

* `format=<output_format>`

    * **Description**: (Optional) Specifies the output data format.
    * **Values**: `arrow`, `json`.
    * **Default**: `arrow`.
    * **Example**: `format=json`

#### Response

The response body and `Content-Type` header depend on the `format` parameter.

***Note: For any request that results in a large data payload, the response body will be streamed using `Transfer-Encoding: chunked` to ensure service stability.***

##### **Response for `format=arrow`**

* **`Content-Type`**: `application/vnd.apache.arrow.stream`
* **Body**: A streaming binary representation of an Apache Arrow **Table**.
    * **Coordinate Columns**: For each dimension, a 1D column with its coordinate values.
    * **Variable Columns**: For each variable, a **flattened 1D column** of numerical data. The column's metadata **must** contain `shape` and `dimensions` keys to allow for client-side reconstruction of the N-dimensional array.

##### **Response for `format=json`**

* **`Content-Type`**: `application/json`

* **Body**: A JSON object with the following structure:

  ```json
  {
    "metadata": {
      "query": { /* The query parameters used for this request */ },
      "shape": [1, 1, 721, 1440],
      "dimensions": ["time", "level", "latitude", "longitude"],
      "variables": {
        "u10": {
          "units": "m s**-1",
          "long_name": "10 metre U wind component"
        }
      }
    },
    "data": {
      "u10": [ /* A 1D flattened array of unpacked numerical values and nulls */ ]
    }
  }
  ```

    * `metadata`: An object containing contextual information about the returned data.
    * `data`: An object where each key is a requested variable name, and the value is a **1D flattened array** containing the fully processed numerical data. Missing values are represented as JSON `null`.

## 4\. Core Concepts & Design Rationale

* **Raw Structured Data (`format=arrow`)**: We offload the final data reconstruction to client libraries (`xarray`, `pandas`) which are highly optimized for this. This maximizes server throughput and network efficiency for high-performance workflows.
* **Web-Friendly Processed Data (`format=json`)**: We intentionally centralize the data processing (unpacking, null conversion) on the server. This simplifies client-side logic, especially for web browsers, adhering to the principle of making APIs easy to consume.
* **User-Defined Layout**: The `layout` parameter remains a critical feature for both formats, allowing users to receive data in the precise dimensional order required by their tools (e.g., ML frameworks), thus avoiding error-prone `transpose` operations.
* **Streaming & Constant Memory Usage**: **This is a fundamental principle of the `/data` endpoint. The server is designed to handle datasets far larger than its available RAM. It achieves this by never materializing the full response body in memory. For both Arrow and JSON formats, data is processed and written to the network stream in small chunks or batches. This ensures constant, predictable memory usage and is fundamental to the service's stability and scalability.**

## 5\. Usage Examples

**Example 1: Get data as Arrow for data science (Default)**

```bash
# Get t2m data for all lat/lon at a specific timestamp
curl "http://127.0.0.1:8000/data?vars=t2m&time=1672531200" -o data.arrow
```

**Example 2: Get data as JSON for a web frontend**

```bash
# Get t2m data and pipe it to jq for pretty-printing
curl "http://127.0.0.1:8000/data?vars=t2m&time=1672531200&format=json" | jq .
```

**Example 3: Get data with a custom layout for PyTorch**

```bash
curl "http://127.0.0.1:8000/data?vars=t2m&level=500&layout=time,latitude,longitude&format=arrow" -o data.arrow
```

## 6\. Safeguards and Considerations

To prevent clients from requesting excessively large datasets that could exhaust server resources, `rossby` **must** implement a configurable **request size limit**. This limit is based on the total number of data points requested and applies regardless of the output `format`. It serves as a guardrail against abusive or accidental queries, **while valid large requests within this limit are handled efficiently via chunked-encoding streaming.** If the limit is exceeded, the server will return a `413 Payload Too Large` error.

## 7\. Benefits

* **Extreme Performance (`format=arrow`)**: Achieves high-throughput data transfer for data science workflows.
* **Seamless Ecosystem Integration (`format=arrow`)**: Data is directly consumable by Pandas, Polars, PyTorch, etc.
* **Enhanced Web Interoperability (`format=json`)**: Provides a simple, standard format for web applications and lightweight clients.
* **Simplified Client Implementation (`format=json`)**: Eliminates the need for clients to handle scientific data format complexities like unpacking.
* **Unified and Flexible API**: A single, powerful endpoint (`/data`) serves a wider range of applications through one simple parameter.
* **Scalability and Stability**: **Thanks to its streaming architecture and chunked transfer encoding, the service can handle large data requests without risking memory exhaustion, making it reliable for production use.**

## 8\. Implementation

### 8.1. Dependencies

The implementation will rely on `arrow` crates for `format=arrow` and `serde_json` for `format=json`.

```toml
[dependencies]
# ...
arrow = "26.0.0"
# ...
serde_json = "1.0"
```

### 8.2. Query Parameter Handling

The `DataQuery` struct will be updated to include the `format` parameter.

```rust
#[derive(Debug, Deserialize)]
pub struct DataQuery {
    pub vars: String,
    pub layout: Option<String>,
    #[serde(default)]
    pub format: Option<String>, // New field
    #[serde(flatten)]
    pub dynamic_params: HashMap<String, String>,
}
```

### 8.3. Handler Implementation

The `data_handler` will determine the format and delegate to a specific processing function that returns a **streamable body**.

```rust
pub async fn data_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DataQuery>,
) -> Response {
    // ...
    let output_format = params.format.as_deref().unwrap_or("arrow");

    match output_format {
        "arrow" => stream_as_arrow(state, params).await,
        "json" => stream_as_json(state, params).await,
        _ => (StatusCode::BAD_REQUEST, "Unsupported format").into_response(),
    }
}

// These functions will return a type that Axum can stream,
// for example, a `Body` created from a `futures::stream::Stream`.
async fn stream_as_arrow(...) -> Response { ... }
async fn stream_as_json(...) -> Response { ... }
```

### 8.4. Streaming Implementation & Chunked Encoding

**This section details the core mechanism for handling large data payloads with constant memory usage.** The Axum web framework natively supports returning response bodies from `Stream` types, which automatically enables `Transfer-Encoding: chunked`.

#### 8.4.1. JSON Streaming

A custom stream will be constructed to generate the JSON response piece-by-piece.

1.  **Initial Chunk**: The stream's first item will be the JSON prefix, e.g., `{"metadata":{...},"data":{"t2m":[`
2.  **Data Chunks**: The implementation will iterate over the source `ndarray`. For each small group of data points, it will perform unpacking, convert them to a string segment (e.g., `"15.4,15.6,null,"`), and yield this string as the next item in the stream.
3.  **Final Chunk**: After all data is processed, the stream's final item will be the JSON suffix, e.g., `]}}`.

This process ensures that the full JSON data array is never held in memory as a single string.

#### 8.4.2. Arrow Streaming

The `arrow-rs` library's `StreamWriter` is designed for this purpose.

1.  **Schema First**: The Arrow schema is written to the output stream first.
2.  **Batched Records**: The data from the source `ndarray` is processed in manageable batches (e.g., 10,000 rows at a time). Each batch is converted into an Arrow `RecordBatch`.
3.  **Stream Writing**: Each `RecordBatch` is written to the `StreamWriter`, which serializes it and sends it immediately down the network stream.
4.  **Finalization**: The `StreamWriter` writes an end-of-stream indicator.

This guarantees that memory usage is proportional to the size of a single batch, not the entire dataset.

## 9\. Testing

### 9.1. Unit Tests

Unit tests will be expanded to cover the JSON output format.

* `test_json_response_generation()`: Verify correct structure, data unpacking, and null handling.

### 9.2. Integration Tests

The integration test suite for the `/data` endpoint will be updated.

* Add a test case for `?format=json`.
* Assert the `Content-Type` is `application/json`.
* Parse the JSON response and verify its structure and a few sample data points.
* Ensure that a request for an unsupported format returns `400 Bad Request`.
* Add a test for a very large request (within the configured limits) to ensure the response is streamed correctly and the server memory usage remains low. This can be verified by observing that the response headers include `Transfer-Encoding: chunked`.

## 10\. Conclusion

By enhancing the `/data` endpoint with a `format` parameter and building it on a **foundation of streaming and chunked transfer encoding**, `rossby` evolves into a significantly more versatile and robust tool. It elegantly serves both high-performance data science and interoperable web services, with the architectural assurance that it can handle large-scale data requests safely and efficiently.
