# Feature: Support for Dimension Aliases

**Labels:** `enhancement`, `feature`, `data-governance`, `configuration`

### Is your feature request related to a problem? Please describe.

Currently, `rossby` brilliantly serves NetCDF files by inferring schema directly from the source file. This works perfectly when the dimension names follow common conventions (e.g., `time`, `latitude`, `longitude`).

However, scientific datasets from various sources often use non-standard or abbreviated names for core spatio-temporal dimensions. For example:
- **Time:** `t`, `TIME`, `time_step`
- **Latitude:** `lat`, `lats`, `y`
- **Longitude:** `lon`, `lons`, `x`
- **Vertical Level:** `lev`, `level`, `plev`

Without a way to map these non-standard names to `rossby`'s internal understanding of spatio-temporal axes, users are forced to manually preprocess their NetCDF files. This creates friction, is time-consuming, and goes against the project's core principle of serving data files directly and without modification.

### Our solution

We propose introducing a new configuration option called `dimension_aliases`. This feature would allow users to provide a simple mapping from `rossby`'s internal, canonical dimension names to the custom names found in their NetCDF file.

This mapping would be provided in the server configuration file (`server.json`):

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 9000
  },
  "data": {
    "file_path": "/path/to/my_weather_data.nc",
    "dimension_aliases": {
      "time": "t",
      "latitude": "lat",
      "longitude": "lon"
    }
  }
}
```

**How it would work:**

1.  **On Startup:** `rossby` reads the `dimension_aliases` map and understands that, for the loaded file, the dimension named `t` should be treated as the time axis, `lat` as the latitude axis, and so on.
2.  **API Handler Logic:** The core logic for endpoints like `/point` and `/image` would use this mapping to correctly interpret the data axes.
3.  **Seamless User Experience:** The key requirement is that the public-facing API remains unchanged for the user. They should continue to use their file's native dimension names in query parameters.

For example, with the configuration above, the following API call should work exactly as expected:

```bash
# User queries using their file's original dimension names: "lon" and "lat"
curl "[http://127.0.0.1:9000/point?lon=139.76&lat=35.68&vars=t2m](http://127.0.0.1:9000/point?lon=139.76&lat=35.68&vars=t2m)"
```

For maximum flexibility, the API should ideally accept **both** the canonical name and the alias in queries:
```bash
# This should also work and produce the same result
curl "[http://127.0.0.1:9000/point?longitude=139.76&latitude=35.68&vars=t2m](http://127.0.0.1:9000/point?longitude=139.76&latitude=35.68&vars=t2m)"
```

4.  **Metadata Endpoint:** The `GET /metadata` endpoint should continue to return the **original** dimension names (`t`, `lat`, `lon`) as found in the source file, to avoid confusing the user who is only familiar with their own data's schema.

### Alternatives considered

1.  **Hard-coding Common Aliases:** We could build a predefined list of common aliases into `rossby` (e.g., automatically recognize `lat` as `latitude`). This is not flexible enough to cover all real-world cases and is less explicit than a user-defined mapping.
2.  **Status Quo (Manual Preprocessing):** Requiring users to rename dimensions in their NetCDF files using external tools (like `nco` or `xarray`). This is the exact workflow friction `rossby` aims to eliminate.

The proposed `dimension_aliases` feature is superior because it is explicit, flexible, and requires no modification of the source data.

### Additional context

Implementing this feature would significantly enhance `rossby` by:
-   **Improving Robustness:** The internal logic can be built upon a stable, canonical set of dimensions.
-   **Increasing Compatibility:** Drastically broadens the range of "out-of-the-box" compatible NetCDF files.
-   **Enhancing User Experience:** Removes a major hurdle for users with non-standard datasets, truly fulfilling the promise of a zero-ETL, instant data server.

This feature provides an elegant bridge between the need for internal consistency and the reality of external data diversity, strengthening `rossby`'s position as a practical tool for the scientific community.
