# rossby: System Design Document

## 1. Introduction & Motivation

`rossby` is born from the need to bridge the gap between static, file-based scientific datasets (specifically NetCDF) and the world of dynamic, interactive web services. Traditional methods involve complex ETL pipelines to load data into a database. `rossby` challenges this by proposing a simpler, faster paradigm: treat the file itself as the database and serve it directly from memory.

The target user is a scientist, engineer, or developer who has a NetCDF file and needs to query or visualize it via an API *now*, without administrative overhead.

## 2. Guiding Principles

- **Immutability (WORM):** The core data is treated as Write-Once-Read-Many. The NetCDF file is loaded at startup and is never modified during the server's lifetime. This dramatically simplifies the design by eliminating the need for transactions, locks, or complex state management.
- **In-Memory First:** To achieve the lowest possible query latency, the entire data tensor is loaded into RAM. The architecture prioritizes vertical scaling (using machines with large RAM) to enable this.
- **Zero-Config for Data:** The data schema (variables, dimensions, coordinates, attributes) is sacred and is derived directly and solely from the source NetCDF file. The tool should adapt to the data, not the other way around.
- **Stateless & Scalable API Layer:** The API server logic is stateless. All the "state" is the immutable data block. This means the server can be horizontally scaled by running multiple identical instances behind a load balancer.

## 3. High-Level Architecture

The system consists of a single, self-contained Rust binary.

````

\+-------------------------------------------------------------+
|                     The `rossby` Process                      |
|                                                             |
|  +---------------------+      +--------------------------+  |
|  | CLI / Config Parser |-----\>|   Data Loading Service   |  |
|  | (clap)              |      |   (netcdf crate)         |  |
|  +---------------------+      +-------------+------------+  |
|                                             |               |
|  +---------------------------------------+  | (on startup)  |
|  |           Axum Web Server             |  |               |
|  | (Tokio async runtime)                 |  |               |
|  |                                       |  v               |
|  |  - /metadata endpoint                 |  +---------------+ |
|  |  - /point endpoint (Interpolation)    |  | In-Memory     | |
|  |  - /image endpoint (Rendering)        |  | Data Store    | |
|  |                                       |  | (Arc\<AppState\>| |
|  +---------------------------------------+  | with ndarray) | |
|       ^                                   |  +---------------+ |
|       | HTTP Requests                   |        ^           |
|       +---------------------------------+--------+-----------+
|                                                             |
\+-------------------------------------------------------------+

```
A static NetCDF file on disk is the single source of truth, loaded into the process at startup.

## 4. Component Deep Dive

### 4.1. CLI & Configuration

- **Library:** The `clap` crate is used to define and parse the CLI arguments. It provides a robust way to handle arguments, environment variables, and subcommand logic.
- **Loading Order:** Configuration is resolved in the following order of precedence:
    1. Command-line arguments (e.g., `--port 8080`)
    2. Environment variables (e.g., `rossby_SERVER_PORT=8080`)
    3. Values from a JSON config file specified via `--config`.
    4. Built-in default values.

### 4.2. Data Ingestion & In-Memory Representation

- **Reader:** The `netcdf` crate is used to open the `.nc` file.
- **Ingestion Process:** On startup, `rossby` performs a one-time, full read of the file.
    1. It first reads all metadata: dimensions, variables, and their attributes.
    2. It allocates a sufficiently large block of memory on the heap.
    3. It reads all the data for all variables into a single (or multiple) `ndarray::ArrayD<f32>` objects.
- **Shared State:** The metadata and the `ndarray` data are wrapped in an `AppState` struct. This struct is then wrapped in an `Arc` (Atomically Referenced Counter) to allow for safe, read-only sharing across all Axum handlers and Tokio's worker threads without needing locks.

### 4.3. The API Layer (Axum)

- **Framework:** Axum is chosen for its tight integration with the Tokio ecosystem, its high performance, and its simple, functional design.
- **Endpoint Implementation Details:**
    - **`GET /metadata`**: This handler simply accesses the `AppState` via the `State<Arc<AppState>>` extractor and serializes the metadata portion to JSON.
    - **`GET /point`**:
        1. Receives `lon`, `lat`, `time_index`, `vars` as query parameters.
        2. Accesses the shared `AppState`.
        3. Validates that all requested variable names in `vars` exist in the NetCDF file's metadata. If not, returns a 400 error.
        4. For each requested variable:
            a. Maps the incoming `lon`, `lat` coordinates to fractional grid indices based on the coordinate arrays read from the metadata.
            b. Identifies the 4 surrounding grid points (`(x, y)`, `(x+1, y)`, `(x, y+1)`, `(x+1, y+1)`).
            c. Slices the in-memory `ndarray` to retrieve the values at these 4 points for the given `time_index`.
            d. Performs a **bilinear interpolation** using the fractional parts of the grid indices as weights.
        5. Serializes the results into a JSON object.
    - **`GET /image`**:
        1. Receives `var`, `time_index`, `bbox`, `center`, `wrap_longitude`, `resampling`, etc. as query parameters.
        2. Validates that the requested `var` exists in the NetCDF file's metadata. If not, returns a 400 error.
        3. Checks if the variable is suitable for image rendering:
            a. Verifies the variable has at least two dimensions.
            b. Attempts to identify latitude and longitude dimensions by checking dimension names (e.g., "lat", "lon") and attributes of coordinate variables (e.g., `units` like "degrees_north", `standard_name` like "latitude").
            c. If the variable is not deemed a 2D spatial grid, returns a 400 error.
        4. Normalizes the bounding box coordinates based on the selected map centering:
           - Eurocentric view (-180° to 180°)
           - Americas-centered view (-90° to 270°)
           - Pacific-centered view (0° to 360°)
           - Custom center (specified longitude as center)
        5. Handles bounding boxes that cross the International Date Line or Prime Meridian when `wrap_longitude=true`.
        6. Slices the `ndarray` to get a 2D data array corresponding to the normalized `bbox`.
        7. Determines the data range (`min`, `max`) for color mapping.
        8. Creates a buffer using the `image` crate.
        9. Applies the selected resampling method (`nearest`, `bilinear`, `bicubic`, or `auto`) for data grid interpolation.
        10. Maps each interpolated value to a color using a colormap function.
        11. Encodes the buffer as a PNG or JPEG and returns the binary data with the correct `Content-Type` header.
    - **`GET /heartbeat`**:
        1. Returns a simple JSON response with server status information.
        2. Includes timestamp, uptime, server ID, and available memory.
        3. Can be used by external services for health monitoring and service discovery.

### 4.4. Service Discovery & Management

- **Service Registration:**
  - At startup, after the server is fully initialized and ready to handle requests, rossby can register itself with a central discovery service.
  - A configurable `discovery_url` parameter allows specifying where to send registration information.
  - Registration payload includes:
    - Server ID (unique identifier)
    - Host and port information
    - Base URL for accessing the API
    - Metadata URL (for dataset discovery)
    - Available dataset information (file name, variables, dimensions)
  
- **Service Management:**
  - The heartbeat endpoint serves as a health check mechanism for external monitoring.
  - The central discovery service can use the heartbeat to track:
    - Server health (up/down status)
    - Data availability (which datasets are available on which servers)
    - Load balancing information (for routing clients to appropriate servers)
  
- **Auto-scaling:**
  - With the discovery protocol in place, it becomes possible to:
    - Automatically scale rossby instances based on demand
    - Distribute different datasets across a pool of servers
    - Provide clients with a unified API for accessing all available data

## 5. Data Flow Walkthroughs

### Startup Flow

1.  User executes `rossby my_data.nc`.
2.  `clap` parses arguments.
3.  The `netcdf::open()` function is called.
4.  Metadata is parsed and stored in an `AppState` struct.
5.  A large `ndarray::ArrayD` is allocated.
6.  The `variables()` method from the `netcdf` crate is used to read all data into the `ndarray`.
7.  The `AppState` is wrapped in `Arc<AppState>`.
8.  The Axum router is initialized, with the `Arc<AppState>` provided as a shared state via `.with_state()`.
9.  The Axum server starts listening for HTTP requests.

### Point Query Flow

1.  An HTTP `GET /point?lon=...` request arrives.
2.  Tokio assigns the request to a worker thread.
3.  The Axum router matches the path and calls the `point_handler` function.
4.  The `State<Arc<AppState>>` extractor in the handler's signature provides a reference to the shared data, incrementing the `Arc`'s reference count.
5.  The handler performs the coordinate-to-index mapping and bilinear interpolation logic as described in section 4.3.
6.  The result is serialized to a JSON string.
7.  An Axum `Response` is created and returned. The `Arc` reference is dropped as the handler goes out of scope.

## 6. Future Work

- **Support for more file formats:** Add readers for GRIB2, Zarr, and TileDB to broaden the tool's applicability.
- **Advanced geographic features:** Implement map projections beyond simple longitude/latitude transformations.
- **Pluggable colormaps:** Allow users to define custom colormaps for the image endpoint.
- **Authentication:** Add an optional API key authentication layer.
- **Caching:** For extremely large files that don't fit in RAM, implement an LRU caching mechanism for frequently accessed data chunks, moving from a pure in-memory to a hybrid model.
- **Enhanced visualization options:** Add support for contour lines, vector overlays, and labeled features.
