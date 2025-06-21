# Rossby: System Design Document (Revision 2)

* Version: Revision 2
* Date: 2025-06-20

## 1. Introduction & Motivation

`rossby` is born from the need to bridge the gap between static, file-based scientific datasets (specifically NetCDF) and the world of dynamic, interactive web services. Traditional methods involve complex ETL (Extract, Transform, Load) pipelines to load data into a database. `rossby` challenges this by proposing a simpler, faster paradigm: **adapt to the data where it lives**. It treats the file itself as the database and serves it directly from memory.

The target user is a scientist, engineer, or developer who has a NetCDF file and needs to query or visualize it via an API *now*, without administrative overhead or data modification.

## 2. Guiding Principles

- **Respect for Data:** Simplifying Service Design to Data Governance: This is the highest-level principle guiding `rossby`. The tool is built on the belief that the user's data is the ultimate authority. The service should not impose its schema, require preprocessing, or force data into a new format. By creating an adaptive, "non-invasive" service, the complex engineering challenge of "how to serve this data" is transformed into a much simpler, more powerful strategic question: "how should we organize our data?" The responsibility shifts from complex ETL and software integration to clean data governance (e.g., partitioning files by year or region). rossby's goal is to make serving scientific data a problem of good data organization, not a problem of software engineering.

- **Immutability (WORM):** The source NetCDF file is the single source of truth. It is loaded at startup and is **never modified** during the server's lifetime. This dramatically simplifies the design by eliminating the need for transactions, locks, or complex state management.

- **In-Memory First:** To achieve the lowest possible query latency, the entire data tensor is loaded into RAM. The architecture prioritizes vertical scaling (using machines with large RAM) to enable this.

- **Non-Invasive & Adaptive (The Core Philosophy):** The tool must adapt to the data, not the other way around. `rossby` is designed to be a "guest" in the user's data ecosystem. It does not require preprocessing, renaming, or reformatting of source files. Through features like **[Dimension Aliasing](features/feat-01-dimension-aliases.md)**, it learns the "dialect" of the source data, respecting its authority and structure. This philosophy directly supports data governance by preserving data lineage and empowering data owners.

- **Stateless & Horizontally Scalable API Layer:** The API server logic is stateless. All the "state" is the immutable data block loaded at startup. This design, combined with an API that queries by **[physical value](features/feat-02-querying-by-physical-value-and-raw-index.md)**, makes `rossby` instances inherently scalable. Multiple instances can be run behind a load balancer, with a routing layer using physical values as natural **partition keys** to direct traffic to the correct instance.

## 3. Architecture

### 3.1. Single-Instance Model

The system consists of a single, self-contained Rust binary. Its internal architecture is as follows:

```
+-------------------------------------------------------------+
|                     The `rossby` Process                      |
|                                                             |
|  +---------------------+      +--------------------------+  |
|  | CLI / Config Parser |----->|   Data Loading Service   |  |
|  | (clap)              |      |   (netcdf crate)         |  |
|  +---------------------+      +-----------+--------------+  |
|                                           | (on startup)    |
|  +-------------------------------------+  | (incl. aliases) |
|  |           Axum Web Server           |  |                 |
|  | (Tokio async runtime)               |  v                 |
|  |                                     | +----------------+ |
|  |  - /metadata endpoint               | | In-Memory      | |
|  |  - /point (Value/Index Query)       | | Data Store     | |
|  |  - /image (Data-to-Image Renderer)  | | (Arc<AppState> | |
|  |                                     | | with ndarray)  | |
|  +-------------------------------------+ +----------------+ |
|       ^                                   ^                 |
|       | HTTP Requests                     | Internal Access |
|       +-----------------------------------+-----------------+
|                                                             |
+-------------------------------------------------------------+
```

### 3.2. Distributed Deployment Model (Clustering)

The API design directly enables a horizontally scaled, distributed system for serving massive datasets partitioned across many files.

```
                  +--------------------------------+
User Request ---> |   Gateway / API Router         |
(e.g., ?time=2024..)| (Data-Aware Routing Logic)     |
                  +---------------+----------------+
                                  |
                                  | (Route based on physical value in query)
                                  |
        +-------------------------+-------------------------+
        |                         |                         |
+-------+--------+       +--------+-------+       +--------+-------+
| rossby Server 1|       | rossby Server 2|       | rossby Server 3|
| (serves 2023)  |       | (serves 2024)  |       | (serves ...)   |
+----------------+       +----------------+       +----------------+
| data_2023.nc   |       | data_2024.nc   |       | data_...  .nc  |
+----------------+       +----------------+       +----------------+
```
In this model, each `rossby` instance registers its metadata (including the physical value ranges of its dimensions) with the Gateway. The Gateway uses this information to route incoming queries to the appropriate instance, creating a unified, scalable data fabric from a simple collection of files.

## 4. Component Deep Dive

### 4.1. CLI & Configuration
- **Library:** `clap` crate for parsing CLI arguments, environment variables, and a JSON config file.
- **Key Configuration:**
    - `file_path`: Path to the source NetCDF file.
    - `dimension_aliases`: A mapping from canonical dimension names (`time`, `latitude`) to file-specific names (`t`, `lat`), enabling `rossby` to adapt to non-standard data.

### 4.2. Data Ingestion & In-Memory Representation
- On startup, `rossby` performs a one-time read of the file via the `netcdf` crate.
- It ingests all metadata and variable data into `ndarray` objects.
- The `dimension_aliases` from the config are processed and stored for fast lookups.
- All of this is wrapped in an `Arc<AppState>` for safe, read-only, lock-free sharing across all concurrent web server threads.

### 4.3. The API Layer (Axum)

#### API Design Philosophy: A Tiered Namespace

The API uses a clear, prefix-based naming convention to create a self-documenting and robust interface.

1.  **User-Friendly Layer (No Prefix):** e.g., `?lat=35.68&t=...`
    - Designed for humans and simple scripts. Uses the file's actual dimension names (or aliases). Queries by physical value.
2.  **Canonical Layer (`_` prefix):** e.g., `?_latitude=35.68&_time=...`
    - Designed for robust system integration. Uses `rossby`'s internal canonical names, ensuring scripts work regardless of the source file's naming conventions.
3.  **Expert/Index Layer (`__` prefix):** e.g., `?__latitude_index=15`
    - Designed for low-level access and debugging. The double underscore acts as a "warning" that the parameter is tied to the internal data structure and may be brittle.

#### Endpoint Implementation Details

- **`GET /metadata`**: Returns the **original** metadata from the file, ensuring the API response is a faithful representation of the source data.

- **`GET /point`**: The primary data extraction endpoint.
    1.  Receives query parameters representing spatial, temporal, or other dimensions.
    2.  Resolves dimension coordinates using the tiered namespace logic, with a clear precedence: raw index (`__`) > physical value (no prefix or `_` prefix).
    3.  For spatial dimensions (`latitude`, `longitude`), it performs **bilinear interpolation** to provide values for off-grid points.
    4.  For all other dimensions, it finds the **exact match** for the requested physical value or uses the specified raw index to select a data slice.
    5.  Returns a JSON object with the final values.

- **`GET /image`**: A simple, powerful data-to-image renderer.
    1.  **Purpose:** To generate a quick "heatmap" visualization of a 2D data slice, not to create a full-featured geographical map.
    2.  Receives parameters to define a data slice (e.g., `var`, `time`, `level`) and rendering options (`bbox`, `colormap`, `width`, `height`).
    3.  Uses the same value/index resolution logic as `/point` to select the data.
    4.  Normalizes the 2D data slice and maps it to a color gradient.
    5.  Returns the resulting image buffer (e.g., PNG) with the correct `Content-Type`.

- **`GET /heartbeat`**: Provides a JSON object with server status, uptime, and memory usage. In a clustered environment, this endpoint is used by the Gateway for health checks. The metadata in this response includes the physical value ranges of the loaded data, enabling data-aware routing.

## 5. Data Flow Walkthroughs

### Startup Flow
1.  User executes `rossby --config my_config.json`.
2.  `clap` parses all configuration sources.
3.  The `netcdf::open()` function is called on the specified file.
4.  Metadata is parsed. The `dimension_aliases` are used to create an efficient, canonical-to-file-specific name mapping in `AppState`.
5.  All variable data is read into `ndarray` objects.
6.  The `AppState` is wrapped in `Arc` and passed to the Axum router.
7.  The server starts, ready to accept requests (and potentially register with a Gateway).

### Point Query Flow (Example)
1.  Request arrives: `GET /point?_latitude=35.5&t=1672531200&vars=t2m`
2.  Axum's `Query` extractor deserializes parameters.
3.  The handler resolves the `time` dimension. It finds the parameter `t` and looks up its physical value `1672531200` in the time coordinate array, finding the corresponding index (e.g., index `5`).
4.  The handler resolves the `latitude` dimension. It sees the `_latitude` canonical parameter and uses its value `35.5` for interpolation calculations.
5.  It uses the resolved time index (`5`) to slice the `t2m` `ndarray`, getting a 2D spatial grid.
6.  It performs bilinear interpolation on this 2D grid using the latitude and longitude values.
7.  The result is serialized to JSON and returned.

## 6. Future Work

- **Advanced Querying:**
    - Support for **nearest-neighbor** lookups for physical values that don't have an exact match.
    - Support for **linear interpolation** along non-spatial dimensions (e.g., time, level).
- **Expanded Format Support:** Add readers for GRIB2, Zarr, or TileDB.
- **Optional Visualization Enhancements:** While not a core feature, consider optional parameters for the `/image` endpoint such as pole enhancement for better visualization of polar regions.
- **Authentication & Caching:** Implement an optional API key layer and an LRU caching mechanism for hybrid memory/disk access models to handle datasets larger than RAM.
