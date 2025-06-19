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
