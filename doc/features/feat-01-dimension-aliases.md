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
curl "http://127.0.0.1:9000/point?lon=139.76&lat=35.68&vars=t2m"
```

**2. By Prefixed Canonical Name (Advanced Method)**

For scripting, automation, or to guarantee clarity, users can reference `rossby`'s internal canonical dimension names. To prevent any ambiguity with user-data names, these canonical parameters **must be prefixed with an underscore (`_`)**. This creates a protected namespace for `rossby`'s system parameters.

```bash
# This call is equivalent to the one above, using the protected canonical names
curl "http://127.0.0.1:9000/point?_longitude=139.76&_latitude=35.68&vars=t2m"
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

## Implementation

### Configuration

The dimension aliases feature is implemented through a configuration option in the `DataConfig` struct:

```rust
/// Data processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    // ...other fields...
    
    /// Dimension aliases mapping canonical names to file-specific names
    /// For example: {"latitude": "lat", "longitude": "lon", "time": "t"}
    #[serde(default)]
    pub dimension_aliases: HashMap<String, String>,
}
```

This allows users to specify a mapping from canonical dimension names to the specific names used in their NetCDF files. The aliases can be provided in a JSON configuration file:

```json
{
  "data": {
    "dimension_aliases": {
      "latitude": "y",
      "longitude": "x",
      "time": "t"
    }
  }
}
```

### State Management

The `AppState` struct maintains a reverse mapping from canonical names to file-specific names to efficiently resolve dimensions:

```rust
/// The main application state shared across all handlers
#[derive(Debug, Clone)]
pub struct AppState {
    // ...other fields...
    
    /// Reverse dimension aliases mapping (canonical name -> file-specific name)
    dimension_aliases_reverse: HashMap<String, String>,
}
```

When the `AppState` is created, it initializes this reverse mapping:

```rust
pub fn new(
    config: Config,
    metadata: Metadata,
    data: HashMap<String, Array<f32, IxDyn>>,
) -> Self {
    // Build the reverse dimension aliases mapping
    let mut dimension_aliases_reverse = HashMap::new();
    for (canonical, file_specific) in &config.data.dimension_aliases {
        dimension_aliases_reverse.insert(canonical.clone(), file_specific.clone());
    }

    Self {
        config,
        metadata,
        data,
        dimension_aliases_reverse,
    }
}
```

### Dimension Resolution

The `AppState` provides a `resolve_dimension` method that resolves dimension names using the following strategy:

1. First, check if the name is a direct file-specific dimension name
2. Next, check if it's a prefixed canonical name (starting with '_')
3. Finally, check if it's an unprefixed canonical name from config aliases

```rust
pub fn resolve_dimension<'a>(&'a self, name: &'a str) -> Result<&'a str> {
    // Case 1: Check if the name is a direct file-specific dimension name
    if self.metadata.dimensions.contains_key(name) {
        return Ok(name);
    }

    // Case 2: Check if it's a prefixed canonical name (starting with "_")
    if name.starts_with('_') {
        let canonical = &name[1..]; // Remove the "_" prefix
        if let Some(file_specific) = self.dimension_aliases_reverse.get(canonical) {
            // Make sure the file-specific name actually exists
            if self.metadata.dimensions.contains_key(file_specific) {
                return Ok(file_specific);
            }
        }
    }

    // Case 3: Check if it's an unprefixed canonical name from config aliases
    if let Some(file_specific) = self.dimension_aliases_reverse.get(name) {
        if self.metadata.dimensions.contains_key(file_specific) {
            return Ok(file_specific);
        }
    }

    // Couldn't resolve the dimension name
    Err(RossbyError::DimensionNotFound {
        name: name.to_string(),
        available: self.metadata.dimensions.keys().cloned().collect(),
        aliases: self.dimension_aliases_reverse.clone(),
    })
}
```

### API Integration

The `point_handler` supports both methods of specifying coordinates:

1. Using the file-specific dimension names directly:
   ```
   /point?lon=139.76&lat=35.68&vars=t2m
   ```

2. Using prefixed canonical names:
   ```
   /point?_longitude=139.76&_latitude=35.68&vars=t2m
   ```

This is implemented through the `PointQuery` struct:

```rust
#[derive(Debug, Deserialize)]
pub struct PointQuery {
    /// Longitude coordinate (file-specific name)
    #[serde(default)]
    pub lon: Option<f64>,
    /// Latitude coordinate (file-specific name)
    #[serde(default)]
    pub lat: Option<f64>,

    /// Longitude coordinate (canonical name with underscore prefix)
    #[serde(rename = "_longitude", default)]
    pub _longitude: Option<f64>,
    /// Latitude coordinate (canonical name with underscore prefix)
    #[serde(rename = "_latitude", default)]
    pub _latitude: Option<f64>,
    
    // ...other fields...
}
```

The handler prioritizes file-specific names but falls back to prefixed canonical names:

```rust
// Get longitude coordinate - try direct file-specific name first, then prefixed canonical name
let lon = match (params.lon, params._longitude) {
    (Some(value), _) => value,
    (None, Some(value)) => value,
    (None, None) => {
        return Err(RossbyError::InvalidParameter {
            param: "longitude".to_string(),
            message: "Missing longitude coordinate...".to_string(),
        })
    }
};
```

### Error Handling

When a dimension cannot be resolved, a detailed error message is provided:

```rust
#[error("Dimension not found: {name}. Available dimensions: {available:?}. If using a canonical name, try using it with an underscore prefix (e.g., '_latitude') or set up dimension_aliases in config.")]
DimensionNotFound {
    name: String,
    available: Vec<String>,
    aliases: std::collections::HashMap<String, String>,
}
```

This error includes the dimension name that couldn't be resolved, a list of available dimensions, and the current aliases configuration to help users troubleshoot the issue.

## Testing

### Unit Tests

We've added unit tests for the dimension aliases feature:

1. Testing direct resolution of file-specific names
2. Testing resolution of prefixed canonical names
3. Testing resolution of unprefixed canonical names (from config)
4. Testing error handling for non-existent dimensions

Example test for dimension aliases in `point_handler`:

```rust
#[test]
fn test_dimension_aliases() {
    // Test with prefixed canonical names
    let state = create_test_state();

    // Use _longitude and _latitude instead of lon and lat
    let params = PointQuery {
        lon: None,
        lat: None,
        _longitude: Some(100.0),
        _latitude: Some(10.0),
        time_index: None,
        vars: "temperature".to_string(),
        interpolation: Some("nearest".to_string()),
    };

    let result = process_point_query(state.clone(), params);
    assert!(result.is_ok());
    let value = result.unwrap().values.get("temperature").unwrap().as_f64().unwrap();
    assert_eq!(value, 1.0);
}
```

### Integration Tests

We've also added integration tests to verify the end-to-end functionality:

1. Testing API requests using file-specific names
2. Testing API requests using prefixed canonical names
3. Testing error responses for invalid dimensions

These tests ensure that the dimension aliases feature works correctly in a real server environment.

## Conclusion

The dimension aliases feature makes `rossby` more flexible and user-friendly by allowing it to work with a wide variety of NetCDF files without requiring preprocessing. The implementation is robust, well-tested, and provides clear error messages to help users troubleshoot any issues.
