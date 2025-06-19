# Feature: Enable Querying by Physical Value and Raw Index

**Labels:** `enhancement`, `feature`, `api-design`, `high-priority`

## 1. Problem Statement

The current `rossby` API primarily relies on `time_index` to select a temporal slice of the data. This approach has significant drawbacks:

1.  **Brittle API Contract:** The `*_index` parameters are internal implementation details. If the underlying NetCDF file is updated or regenerated (e.g., with a different starting date or resolution), all client integrations that rely on hardcoded or calculated indices will break.
2.  **Limited & Unintuitive Querying:** There is no direct way to query for a specific physical coordinate value. A user cannot ask, "Give me the data for the timestamp `2023-01-01T00:00:00Z`," "for longitude `90.0` degrees," or "at the `850` hPa pressure level." They are forced to first determine the integer index corresponding to that value, which is cumbersome.

This feature request proposes to resolve these issues by introducing a robust, value-based querying mechanism and establishing a consistent pattern for low-level index access.

## 2. Proposed Solution

This feature introduces two new, distinct methods for querying any dimension (e.g., `time`, `latitude`, `longitude`, `level`), establishing a clear and consistent API paradigm.

### 2.1. Method 1: Querying by Physical Value (Exact Match)

This will be the primary and recommended way to query for data.

* **Mechanism:** Users will use the dimension's name (or its alias as defined in the `dimension_aliases` configuration) as a query parameter, providing a specific physical value.
* **Behavior:** `rossby` will perform an **exact match** lookup on the coordinate data for that dimension. If a matching value is found, the corresponding data slice is returned. If not, an appropriate error is returned. This method **does not** perform interpolation.
* **Example:** Assuming `dimension_aliases` is `{"time": "t", "longitude": "lon", "level": "plev"}`:
    ```bash
    # Request data for a specific timestamp, longitude, and pressure level by their physical values
    curl "[http://127.0.0.1:8000/point?lat=...&lon=90.0&plev=850&t=1672531200](http://127.0.0.1:8000/point?lat=...&lon=90.0&plev=850&t=1672531200)"
    ```

### 2.2. Method 2: Querying by Raw Index (Expert Use)

This method provides a consistent way for low-level, index-based access while clearly signaling its nature.

* **Mechanism:** Use a double-underscore (`__`) prefix, followed by the canonical dimension name and the `_index` suffix.
* **Rationale:** The `__` prefix acts as a namespace for internal or expert-level parameters. It visually communicates that this is a low-level operation tied to the data's internal structure, discouraging its use in high-level applications and preventing the creation of brittle integrations.
* **Example:**
    ```bash
    # Request data for the 0th time step, 32nd latitude point, and 5th level by their raw indices
    curl "[http://127.0.0.1:8000/point?lon=...&__time_index=0&__latitude_index=32&__level_index=5](http://127.0.0.1:8000/point?lon=...&__time_index=0&__latitude_index=32&__level_index=5)"
    ```

### 2.3. Deprecation

* The existing `time_index` parameter will be **deprecated**.
* It will be replaced by `__time_index` to align with the new, consistent naming convention.

## 3. Benefits

This is a foundational improvement for `rossby` with immediate, significant benefits:

1.  **Robust API:** By enabling queries based on physical values, the API is decoupled from the internal data structure. Client integrations will be far more stable and resilient to changes in the source data files.
2.  **Intuitive User Experience:** Querying by real-world values (`lon=90.0`, `plev=850`, `t=...`) is far more natural and user-friendly than working with abstract integer indices.
3.  **Clear & Consistent Design:** The `__` prefix convention creates a clear, logical, and extensible paradigm. It cleanly separates high-level, value-based queries from low-level, index-based access.
4.  **Lays the Groundwork:** This feature establishes the essential API foundation upon which future capabilities, such as interpolation (e.g., `_time={timevalue}`), can be built.

## 4. Implementation

### 4.1. API Changes

The point query endpoint now supports the following parameter patterns:

1. **Direct Physical Value**
   * Specify a physical coordinate value directly: `lon=135.0`, `lat=42.5`, `time=1672531200`
   * This is the preferred, user-friendly way to query

2. **Canonical Physical Value**
   * Prefixed canonical name with underscore: `_longitude=135.0`, `_latitude=42.5`, `_time=1672531200`
   * Useful when the file-specific names differ from standard conventions

3. **Raw Index Access**
   * Double-underscore prefix with canonical name and _index suffix: `__longitude_index=10`, `__latitude_index=5`, `__time_index=0`
   * For expert use when exact control over indices is needed

4. **Legacy Parameters** (Deprecated)
   * The `time_index` parameter is maintained for backward compatibility but marked as deprecated
   * Users will see a warning in the logs when using this parameter

### 4.2. Error Handling

Several new error types have been added to handle the extended querying capabilities:

1. **`PhysicalValueNotFound`**
   * Returned when a specific physical value doesn't exactly match any coordinate in the array
   * Includes the dimension name, requested value, and available values

2. **`IndexOutOfBounds`**
   * Returned when a raw index is outside the valid range for a dimension
   * Includes the parameter name, provided value, and maximum allowed index

3. **`InvalidParameter`**
   * Used for general parameter validation errors
   * Provides detailed messages about the expected format and options

### 4.3. Implementation Details

The implementation follows these key principles:

1. **Clear Preference Order**
   * When multiple parameter forms are provided, the order of precedence is:
     1. Raw indices (most specific)
     2. File-specific physical values
     3. Canonical physical values
     4. Default values (e.g., time_index=0)

2. **Exact Matching for Physical Values**
   * Physical values must match exactly (within floating-point epsilon)
   * This is distinct from interpolation, which occurs after coordinates are resolved

3. **Backward Compatibility**
   * Legacy parameters continue to work but trigger deprecation warnings
   * New functionality is additive and doesn't break existing client code

## 5. Testing

Comprehensive tests have been added to verify all aspects of the feature:

1. **Physical Value Queries**
   * Test exact matches for various dimensions
   * Test error cases for values not present in coordinate arrays

2. **Raw Index Queries**
   * Test valid index access
   * Test out-of-bounds error handling

3. **Mixed Parameter Testing**
   * Test combinations of different parameter types
   * Verify precedence rules are correctly applied

4. **Dimension Aliases**
   * Test that aliases defined in configuration work correctly
   * Test canonical name resolution

5. **Backward Compatibility**
   * Verify deprecated parameters still function as expected
   * Check warning messages are generated appropriately

## 6. Usage Examples

### Example 1: Basic Physical Value Query

```bash
# Get temperature at specific lon/lat coordinates
curl "http://127.0.0.1:8000/point?lon=135.25&lat=35.68&vars=t2m"
# Response: {"t2m": 288.45}
```

### Example 2: Using Canonical Names

```bash
# Get multiple variables using canonical dimension names
curl "http://127.0.0.1:8000/point?_longitude=135.25&_latitude=35.68&vars=t2m,humidity"
# Response: {"t2m": 288.45, "humidity": 85.2}
```

### Example 3: Raw Index Access

```bash
# Expert usage with direct indices
curl "http://127.0.0.1:8000/point?__longitude_index=32&__latitude_index=15&vars=t2m"
# Response: {"t2m": 290.1}
```

### Example 4: Specific Time Value

```bash
# Query for a specific timestamp (Unix timestamp or ISO string depending on file)
curl "http://127.0.0.1:8000/point?lon=135.25&lat=35.68&time=1672531200&vars=t2m"
# Response: {"t2m": 278.9}
```

### Example 5: Mixed Parameter Types

```bash
# Mixing parameter styles (raw indices with physical values)
curl "http://127.0.0.1:8000/point?lon=135.25&__latitude_index=15&__time_index=0&vars=t2m"
# Response: {"t2m": 285.3}
```

## 7. Conclusion

This feature significantly enhances the usability and robustness of the `rossby` API. By providing multiple, clearly defined ways to query data, it accommodates both casual users who prefer intuitive physical values and expert users who need precise control through indices. The consistent naming convention with visual distinctions between parameter types creates a self-documenting API that helps users understand the implications of their parameter choices.
