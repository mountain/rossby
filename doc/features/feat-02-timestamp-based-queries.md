# Feature Request: Enable Temporal Interpolation with Timestamp-based Queries

## 1. Problem Statement

Currently, querying data at a specific time requires using the `time_index` parameter. This approach has two main drawbacks from a data governance and API design perspective:

1.  **Brittle API Contract:** `time_index` is an internal implementation detail (the array index). If the underlying NetCDF file is updated or regenerated with a different time resolution or start date, all client integrations that rely on hardcoded or calculated indices will break.
2.  **Limited Scientific Utility:** Data often represents continuous phenomena. Forcing users to query only at discrete, pre-defined time steps prevents them from analyzing or visualizing the state of the system at arbitrary points in time.

This feature request proposes enhancing `rossby` to treat time as a continuous dimension, similar to how it currently treats space, by enabling queries based on a physical timestamp and performing on-the-fly temporal interpolation.

## 2. Proposed Solution

We propose the introduction of a new `time` parameter for the `/point` and `/image` endpoints. This parameter would accept a standard timestamp format (e.g., Unix timestamp).

When a request with the `time` parameter is received, `rossby` will:
1.  Identify the two time steps in the dataset that bracket the requested timestamp.
2.  Perform a linear interpolation in the time dimension to compute the data for that exact moment.
3.  For `/image` requests, implement a high-performance LRU cache to store the generated images, as rendering can be computationally expensive.

The existing `time_index` parameter will be retained for backward compatibility. The server will prioritize the `time` parameter if both are present.

## 3. Detailed Design

### 3.1 API Changes

- **Add `time` parameter:** Introduce an optional `time` query parameter (e.g., integer Unix timestamp) to both `GET /point` and `GET /image`.
- **Backward Compatibility:** If `time` is not provided, the API will fall back to using the existing `time_index` parameter.

**Example Usage:**
```bash
# Get an interpolated point value for a specific timestamp
curl "[http://127.0.0.1:8000/point?lon=...&lat=...&vars=t2m&time=1672531200](http://127.0.0.1:8000/point?lon=...&lat=...&vars=t2m&time=1672531200)"

# Get an interpolated image for a specific timestamp
curl "[http://127.0.0.1:8000/image?var=t2m&bbox=...&time=1672531200](http://127.0.0.1:8000/image?var=t2m&bbox=...&time=1672531200)" -o interpolated_image.png
```

### 3.2 Interpolation Logic

The key to an efficient implementation is to perform interpolation in **data-space**, not image-space.

**For `GET /image`:**
1.  Identify the two time slices (`t1`, `t2`) that bracket the requested `time`.
2.  Extract the corresponding 2D `ndarray` data slices for the requested `bbox` at both `t1` and `t2` (`data_slice_t1`, `data_slice_t2`).
3.  Create a new 2D array, `interpolated_data_slice`, by performing a linear interpolation between `data_slice_t1` and `data_slice_t2` for each grid cell.
4.  Pass this **single** `interpolated_data_slice` to the rendering engine to generate the final image. The rendering pipeline (coloring, resampling, drawing overlays) is only executed once.

**For `GET /point`:**
1.  Identify time slices `t1` and `t2`.
2.  Perform 2D spatial interpolation (bilinear) at `t1` to get `value1`.
3.  Perform 2D spatial interpolation (bilinear) at `t2` to get `value2`.
4.  Perform 1D linear interpolation between `value1` and `value2` to get the final result. This constitutes a full trilinear interpolation.

### 3.3 Performance Optimization: LRU Cache for Images

To handle the computational cost of the rendering step for the `/image` endpoint, a configurable LRU (Least Recently Used) cache for the final image binaries is essential.

- The cache should be implemented in-memory and its max size (e.g., `cache-size-mb`) should be a configurable server option.
- When an image request is received, the server will first compute the cache key and check the cache.
- On a cache hit, the binary image is served directly.
- On a cache miss, the server will generate the image via the interpolation and rendering pipeline, store the result in the cache, and then return it to the user.

### 3.4 Canonical Cache Key Design

To maximize cache efficiency, the cache key must be **canonical**, meaning equivalent-but-different requests map to the same key. We will adopt a sorted key-value string strategy.

**Key Generation Logic:**
1.  **Normalize Coordinates:** The user-provided `bbox` will be normalized based on the `center` parameter to a canonical coordinate system (e.g., longitude in `[-180, 180]`). The `center` parameter itself will be **omitted** from the key. The result is a `normalized_bbox` string.
2.  **Collect Parameters:** Gather all parameters that define the final image, including default values.
3.  **Sort and Concatenate:** Sort the parameters alphabetically by key and concatenate them into a single string.

**Parameters to be included in the key:**
- `colormap`
- `coastlines`
- `enhance_poles`
- `format`
- `grid`
- `height`
- `normalized_bbox` (replaces `bbox` and `center`)
- `resampling`
- `time`
- `var`
- `width`
- `wrap_longitude`

**Example Key:**
```
"coastlines=true&colormap=viridis&...&normalized_bbox=10,20,30,40&time=1672531200&var=t2m&width=800&..."
```
This design ensures that requests that are geographically identical but use different `center` parameters will resolve to the same cache key, increasing the cache hit rate.

## 4. Benefits

- **Robust API Contract:** Decouples the API from the data's internal structure, making it stable against changes in the source file.
- **Enhanced Scientific Accuracy & Usability:** Allows users to query and visualize data for any point in time, aligning the tool's behavior with the continuous nature of the phenomena it represents.
- **Improved Interoperability:** Using standard timestamps makes it trivial to integrate `rossby` with other time-series datasets and tools.
- **Elegant & Consistent Design:** Treats the time dimension as a first-class continuous coordinate, consistent with its handling of spatial dimensions.
- **High Performance:** The combination of data-space interpolation and a canonical LRU cache ensures that this powerful new feature remains fast and scalable.
