# Rossby: The Instant Spatio-Temporal Database

[![CI](https://github.com/mountain/rossby/actions/workflows/ci.yml/badge.svg)](https://github.com/mountain/rossby/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rossby.svg)](https://crates.io/crates/rossby)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> **NOTE:** Rossby is currently in early development (v0.0.1). The API may change in future releases.

**`rossby` is a blazingly fast, in-memory, NetCDF-to-API server written in Rust.**

Instantly serve massive NetCDF datasets as a high-performance HTTP API for point queries and image rendering, with zero data configuration.

## Vision

Scientific data is often locked away in static files like NetCDF. `rossby` liberates this data by providing a simple command-line tool to load a file directly into memory and serve it via a simple, powerful API. It's designed for scientists, engineers, and anyone who needs to interact with spatio-temporal grid data dynamically without the overhead of setting up a traditional database.

## Features

- **In-Memory Performance:** Loads the entire dataset into RAM for microsecond-level query latency.
- **NetCDF Native:** Directly reads `.nc` files without any preprocessing or import steps.
- **Zero Data-Config:** All metadata (variables, dimensions, coordinates) is automatically inferred from the NetCDF file.
- **High-Performance API:** Built with Rust, Axum, and Tokio for incredible speed and concurrency.
- **On-the-fly Interpolation:** Point queries are not limited to the grid; `rossby` provides interpolated values for any coordinate.
- **Dynamic Image Generation:** Instantly render data slices as PNG or JPEG images for quick visualization.
- **Flexible Server Configuration:** Configure your server via command-line arguments, environment variables, or a JSON file, inspired by `uwsgi`.
- **Server Monitoring:** Built-in `/heartbeat` endpoint provides comprehensive server status, including memory usage and uptime.
- **Service Discovery Ready:** Support for service registration and discovery to enable scalable multi-server deployments.

## Quick Start

### 1\. Installation

Ensure you have Rust installed. Then, install `rossby` using cargo:

```sh
cargo install rossby
```

### 2\. Get Sample Data

We'll use a sample weather forecast file for this demo.

```sh
# A real climate data file
wget https://github.com/mountain/rossby/raw/main/tests/fixtures/2m_temperature_1982_5.625deg.nc
```

### 3\. Run `rossby`

Point `rossby` at your NetCDF file. It's that simple.

```sh
rossby 2m_temperature_1982_5.625deg.nc
```

You should see output indicating the server has started, probably on `127.0.0.1:8000`.

```
INFO  rossby > Loading NetCDF file: "2m_temperature_1982_5.625deg.nc"
INFO  rossby > Found 4 variables
INFO  rossby > Found 3 dimensions
INFO  rossby > Data loaded successfully.
INFO  axum::server > Listening on http://127.0.0.1:8000
```

### 4\. Query the API

Open a new terminal and use `curl` to interact with your new, instant database.

**Get Metadata:** Discover what's in the file.

```sh
curl http://127.0.0.1:8000/metadata
```

**Get a Point Forecast:** Get the interpolated 2-meter temperature (`t2m`) for a specific location. There are two ways to query points:

1.  Using time index (legacy method):

```sh
curl "http://127.0.0.1:8000/point?lon=139.76&lat=35.68&time_index=0&vars=t2m"
# Expected Response: {"t2m": 288.45}
```

2.  Using physical time value (recommended):

```sh
curl "http://127.0.0.1:8000/point?lon=139.76&lat=35.68&time=1672531200&vars=t2m"
# Expected Response: {"t2m": 288.45}
```

**Get an Image:** Render an image of the `t2m` variable for a specific region and time.

```sh
curl "http://127.0.0.1:8000/image?var=t2m&time_index=0&bbox=120,20,150,50" -o japan_temp.png
# Now open the generated japan_temp.png file.
```

## Configuration

`rossby` uses a layered configuration system with the following order of precedence:

1.  **Command-Line Arguments** (highest priority)
2.  **Environment Variables**
3.  **JSON Config File**
4.  **Default Values** (lowest priority)

**CLI Usage:**

```sh
rossby [OPTIONS] <NETCDF_FILE>

# Example: Run on a public IP, port 9000, with 8 worker threads
rossby --host 0.0.0.0 --port 9000 --workers 8 my_data.nc

# Enable service discovery
rossby --discovery-url http://discovery-service:8080/register my_data.nc
```

**JSON Configuration:**
You can specify a config file with the `--config` flag.
`rossby --config server.json`

An example `server.json`:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 9000,
    "workers": 8,
    "discovery_url": "http://discovery-service:8080/register"
  },
  "data": {
    "interpolation_method": "bilinear",
    "file_path": "/path/to/data.nc"
  }
}
```

## API Reference

A detailed reference for the available HTTP endpoints.

-----

### `GET /metadata`

Returns a JSON object describing all variables, dimensions, and attributes of the loaded NetCDF file.

**No query parameters.**

-----

### `GET /point`

Returns interpolated values for one or more variables at a specific point in space-time.

**Query Parameters:**

- `lon`: (required) Longitude of the query point.
- `lat`: (required) Latitude of the query point.
- `vars`: (required) Comma-separated list of variable names to query (e.g., `t2m,u10`).
- `time` or `time_index`: (required) Specify the time for the query.
  - `time`: The physical time value (e.g., a time value like Unix timestamp or others specified by the metadata). Recommended method.
  - `time_index`: The integer index of the time dimension.

-----

### `GET /image`

Returns a PNG or JPEG image rendering of a single variable over a specified region and time.

**Query Parameters:**

- `var`: (required) The variable name to render.
- `time_index`: (optional) The integer index of the time dimension. Defaults to `0`.
- `bbox`: (optional) Bounding box as a string `"min_lon,min_lat,max_lon,max_lat"`. If not provided, the entire spatial domain is rendered.
- `width`: (optional) Image width in pixels. Defaults to `800`.
- `height`: (optional) Image height in pixels. Defaults to `600`.
- `colormap`: (optional) Colormap name (e.g., `viridis`, `plasma`, `coolwarm`). Defaults to `"viridis"`.
- `format`: (optional) Output image format. Can be `"png"` or `"jpeg"`. Defaults to `"png"`.
- `center`: (optional) Adjusts the map's longitudinal center. Can be `"eurocentric"` (-180° to 180°), `"americas"` (-90° to 270°), `"pacific"` (0° to 360°), or a custom longitude value. Defaults to `"eurocentric"`.
- `wrap_longitude`: (optional) Set to `true` to allow bounding boxes that cross the dateline/prime meridian. Defaults to `false`.
- `resampling`: (optional) The resampling filter for upsampling/downsampling. Can be `"nearest"`, `"bilinear"`, `"bicubic"`, or `"auto"`. Defaults to `"auto"` (bilinear for upsampling, bicubic for downsampling).

-----

### `GET /heartbeat`

Returns a JSON object with server status, memory usage, and dataset information. Useful for monitoring and service health checks.

**No query parameters.**

**Example Response Body:**

```json
{
  "server_id": "unique-server-id-123",
  "timestamp": "2025-06-20T13:30:00Z",
  "uptime_seconds": 3600,
  "memory_usage_bytes": 512000000,
  "available_memory_bytes": 16000000000,
  "status": "healthy",
  "dataset": {
    "file_path": "/path/to/data.nc",
    "variable_count": 4,
    "variables": ["t2m", "u10", "v10", "msl"],
    "dimension_count": 3,
    "dimensions": {
      "time": 744,
      "latitude": 32,
      "longitude": 64
    },
    "data_memory_bytes": 450000000
  }
}
```

## Building from Source

```sh
git clone https://github.com/mountain/rossby.git
cd rossby
cargo build --release
./target/release/rossby --help
```

## Development

### Continuous Integration

This project uses GitHub Actions for continuous integration. The CI pipeline runs the following checks on every push and pull request:

1.  `cargo check` - Verifies the code compiles without errors
2.  `cargo test` - Runs all tests to ensure they pass
3.  `cargo clippy` - Performs static analysis to catch common mistakes
4.  `cargo fmt --check` - Ensures code adheres to formatting standards

You can see the CI configuration in the `.github/workflows/ci.yml` file.

### Git Hooks

To ensure code quality before commits are made, we provide Git hooks in the `hooks/` directory. These hooks automatically run tests and other checks before allowing commits.

To install the hooks, follow the instructions in the `hooks/README.md` file.

## Contributing

Contributions are welcome\! Please feel free to open an issue or submit a pull request.

Before submitting a PR, please make sure:

1.  All tests pass (`cargo test`)
2.  The code is properly formatted (`cargo fmt`)
3.  There are no clippy warnings (`cargo clippy`)
4.  You've added tests for any new functionality

## License

This project is licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.