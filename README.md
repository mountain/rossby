# rossby: The Instant Spatio-Temporal Database

[![CI](https://github.com/your-username/rossby/actions/workflows/ci.yml/badge.svg)](https://github.com/your-username/rossby/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/rossby.svg)](https://crates.io/crates/rossby)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

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

## Quick Start

### 1. Installation

Ensure you have Rust installed. Then, install `rossby` using cargo:
```sh
cargo install rossby
````

### 2\. Get Sample Data

We'll use a sample weather forecast file for this demo.

```sh
# (This is a placeholder for a real sample data URL)
wget [https://example.com/path/to/sample_forecast.nc](https://example.com/path/to/sample_forecast.nc)
```

### 3\. Run `rossby`

Point `rossby` at your NetCDF file. It's that simple.

```sh
rossby sample_forecast.nc
```

You should see output indicating the server has started, probably on `127.0.0.1:8000`.

```
INFO  rossby > Loading metadata from 'sample_forecast.nc'...
INFO  rossby > Found variables: t2m, sst, u10, v10
INFO  rossby > Loading data into memory... (~5.2 GB)
INFO  rossby > Data loaded successfully.
INFO  axum::server > Listening on [http://127.0.0.1:8000](http://127.0.0.1:8000)
```

### 4\. Query the API

Open a new terminal and use `curl` to interact with your new, instant database.

**Get Metadata:** Discover what's in the file.

```sh
curl [http://127.0.0.1:8000/metadata](http://127.0.0.1:8000/metadata)
```

**Get a Point Forecast:** Get the interpolated 2-meter temperature (`t2m`) for a specific location at the first time step (`time_index=0`).

```sh
curl "[http://127.0.0.1:8000/point?lon=139.76&lat=35.68&time_index=0&vars=t2m](http://127.0.0.1:8000/point?lon=139.76&lat=35.68&time_index=0&vars=t2m)"
# Expected Response: {"t2m": 288.45}
```

**Get an Image:** Render an image of the `t2m` variable for a specific region and time.

```sh
curl "[http://127.0.0.1:8000/image?var=t2m&time_index=0&bbox=120,20,150,50](http://127.0.0.1:8000/image?var=t2m&time_index=0&bbox=120,20,150,50)" -o japan_temp.png
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
```

**JSON Configuration:**

You can specify a config file with the `--config` flag.
`rossby --config server.json`

**`server.json` example:**

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 9000,
    "workers": 8
  },
  "data": {
    "interpolation_method": "bilinear",
    "file_path": "/path/to/data.nc"
  }
}
```

## API Reference

- **`GET /metadata`**: Returns JSON describing all variables, dimensions, and attributes of the loaded file.
- **`GET /point`**: Returns interpolated values for one or more variables at a specific point in space-time.
- **`GET /image`**: Returns a PNG/JPEG image rendering of a variable over a specified region and time.

For full details, see the [design.md](https://www.google.com/search?q=design.md) document.

## Building from Source

```sh
git clone [https://github.com/your-username/rossby.git](https://github.com/your-username/rossby.git)
cd rossby
cargo build --release
./target/release/rossby --help
```

## Contributing

Contributions are welcome\! Please feel free to open an issue or submit a pull request.

## License

This project is licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.
