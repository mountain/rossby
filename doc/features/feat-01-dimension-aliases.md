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

### Describe the solution you'd like

We propose introducing a new configuration option called `dimension_aliases`. This feature would allow users to provide a simple mapping from `rossby`'s internal, canonical dimension names to the custom names found in their NetCDF file.

This mapping would be provided in the server configuration file (`server.json`). Note that the canonical names in the configuration (`longitude`, `latitude`, etc.) are clean and do not have any prefix.

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

The API will provide two distinct and unambiguous ways to reference a dimension in a query:

**1. By User-Defined Alias (Primary Method)**

Users can—and typically should—use the exact dimension name from their source NetCDF file. This is the most direct and intuitive method.

```bash
# User queries using their file's original dimension names: "lon" and "lat"
curl "[http://127.0.0.1:9000/point?lon=139.76&lat=35.68&vars=t2m](http://127.0.0.1:9000/point?lon=139.76&lat=35.68&vars=t2m)"
```

**2. By Prefixed Canonical Name (Advanced Method)**

For scripting, automation, or to guarantee clarity, users can reference `rossby`'s internal canonical dimension names. To prevent any ambiguity with user-data names, these canonical parameters **must be prefixed with an underscore (`_`)**. This creates a protected namespace for `rossby`'s system parameters.

```bash
# This call is equivalent to the one above, using the protected canonical names
curl "[http://127.0.0.1:9000/point?_longitude=139.76&_latitude=35.68&vars=t2m](http://127.0.0.1:9000/point?_longitude=139.76&_latitude=35.68&vars=t2m)"
```

With this rule, an unprefixed canonical name (e.g., `?longitude=...`) would be interpreted as a literal search for a dimension named `longitude` in the source file, and would not be treated as a system parameter. This resolves all potential naming collisions.

**Metadata Endpoint (`GET /metadata`)**

The `/metadata` endpoint's behavior remains unchanged: it should continue to return the **original** dimension names (`t`, `lat`, `lon`) as found in the source file. This ensures the API response is always consistent with the user's view of their data.

### Describe alternatives you've considered

1.  **Hard-coding Common Aliases:** We could build a predefined list of common aliases into `rossby`. This is not flexible enough to cover all real-world cases and is less explicit than a user-defined mapping.
2.  **Status Quo (Manual Preprocessing):** Requiring users to rename dimensions in their NetCDF files using external tools. This is the exact workflow friction `rossby` aims to eliminate.

The proposed `dimension_aliases` feature with the `_` prefix convention is superior because it is explicit, flexible, robust, and requires no modification of the source data.

### Additional context

Implementing this feature would significantly enhance `rossby` by:
-   **Improving Robustness:** The internal logic can be built upon a stable, canonical set of dimensions, while the API remains flexible.
-   **Increasing Compatibility:** Drastically broadens the range of "out-of-the-box" compatible NetCDF files.
-   **Creating an Unambiguous & Future-Proof API:** The `_` prefix for canonical names creates a clear, reserved namespace. This prevents collisions with user data and establishes a solid pattern for any future system-level query parameters (e.g., `_projection`, `_format`).
-   **Enhancing User Experience:** Removes a major hurdle for users with non-standard datasets, truly fulfilling the promise of a zero-ETL, instant data server.
