# Feature: Data Access with Apache Arrow

**Labels:** `enhancement`, `feature`, `api-design`, `performance`, `mle`, `high-priority`

## 1. Problem Statement

While `rossby` currently excels at providing interactive single-point queries (`/point`) and 2D data visualizations (`/image`), it lacks a core capability: the ability to **efficiently extract large-scale, multi-dimensional structured data subsets**.

For programmatic use cases like scientific computing and Machine Learning (ML) model training, which require large volumes of raw numerical data, using JSON for data transfer introduces significant serialization/deserialization overhead and network payload, failing to meet high-performance requirements. To position `rossby` as a bridge between raw scientific data and modern data science workflows, a solution designed for high-throughput data transfer is needed.

## 2. Proposed Solution

We propose the introduction of a new endpoint: `GET /data`.

The core mission of this endpoint is to stream user-defined, N-dimensional data hyperslabs directly in the **Apache Arrow** format. Apache Arrow is an in-memory columnar data format designed for high-performance analytics, serving as the lingua franca for the modern data science (Pandas, Polars, DuckDB) and machine learning (PyTorch, TensorFlow) ecosystems.

This feature will provide users with a near-zero-copy, analytics-ready data stream, enabling them to seamlessly integrate `rossby` into their data processing and MLOps workflows. Key features include:

* Flexible subsetting of data hyperslabs via physical value or index ranges.
* The ability for users to specify the output data's dimension layout to match the requirements of downstream computational frameworks.
* Adoption of a "raw structured" data model to prioritize maximum server throughput and network efficiency.

## 3. API Endpoint Specification

### `GET /data`

#### Query Parameters

* `vars=<variables>`
    * **Description**: (Required) A comma-separated list of one or more variables to extract.
    * **Example**: `vars=t2m,u10`

* **Dimension Selectors**: For each dimension in the data (e.g., `time`, `latitude`, `level`), the user can constrain it using one of the following methods. If a dimension is not constrained, its entire range is selected by default.
    * `<dim_name>=<value>`: Select a single slice by physical value.
        * **Example**: `level=850`
    * `<dim_name>_range=<start_value>,<end_value>`: Select a closed interval range by physical values.
        * **Example**: `time_range=1672531200,1675209600`
    * `__<canonical_name>_index=<index>`: Select a single slice by its raw index.
        * **Example**: `__time_index=0`
    * `__<canonical_name>_index_range=<start_index>,<end_index>`: Select a closed interval range by raw indices.
        * **Example**: `__latitude_index_range=10,50`

* `layout=<dimension_order>`
    * **Description**: (Optional) A comma-separated string specifying the desired dimension order for the output N-dimensional data array.
    * **Default**: If omitted, the native dimension order from the source NetCDF file is used.
    * **Example**: `layout=time,longitude,latitude`

#### Response

* **`Content-Type`**: `application/vnd.apache.arrow.stream`
* **Body**: A streaming binary representation of an Apache Arrow **Table**, with a schema defined as follows:
    * **Coordinate Columns**: For each dimension included in the query, a corresponding 1D column containing its coordinate values (e.g., a `time` column, a `latitude` column).
    * **Variable Columns**: For each variable requested in `vars`, a corresponding column (e.g., a `t2m` column).
        * The data in this column is a **flattened 1D array** containing all the numerical values from the selected data hyperslab.
        * The **metadata** map of this column's field **must** contain the following two keys to allow for client-side reconstruction of the N-dimensional array:
            * `shape`: A JSON string array representing the data's shape. Example: `"[10, 90, 45]"`.
            * `dimensions`: A JSON string array listing the dimension names in their specified order, matching the `layout` parameter. Example: `"['time', 'longitude', 'latitude']"`.

## 4. Core Concepts & Design Rationale

* **Raw Structured Data**: We chose not to reshape the data into a "Tidy Data" format on the server. This decision maximizes server-side processing speed and throughput while minimizing the network payload size. It offloads the final data reconstruction task to client libraries (e.g., `xarray`), which are highly optimized for this and know the user's ultimate data structure requirements best.
* **User-Defined Layout**: The `layout` parameter is a critical feature. Different computational frameworks have different expectations for the dimension order of input tensors. Providing this functionality eliminates the need for users to perform error-prone `permute`/`transpose` operations on the client side, significantly improving the ease of integration with ML/DL frameworks.

## 5. Usage Examples

**Example 1: Get a 2D spatial slice for a single time step**
```bash
# Get t2m data for all lat/lon at a specific timestamp
curl "http://127.0.0.1:8000/data?vars=t2m&time=1672531200" -o data.arrow
```

**Example 2: Get a 3D data cube over a spatio-temporal range**
```bash
# Get u10 and v10 variables for all levels within a specified time and lat/lon range
curl "http://127.0.0.1:8000/data?vars=u10,v10&time_range=...&lat_range=30,40&lon_range=130,140" -o data.arrow
```

**Example 3: Get data with a custom layout for PyTorch**
```bash
# Request a specific dimension layout to match PyTorch's convolution input requirements
curl "http://127.0.0.1:8000/data?vars=t2m&level=500&layout=time,latitude,longitude" -o data.arrow
```

## 6. Safeguards and Considerations

To prevent clients from requesting excessively large datasets that could exhaust server memory or saturate the network, `rossby` **must** implement a configurable **request size limit**.

* **Mechanism**: Before processing a request, the server will calculate the total number of data points to be returned based on the selected dimension ranges (`total_points = num_times * num_lats * num_lons ...`).
* **Behavior**: If the calculated number of points exceeds the configured threshold (e.g., `10,000,000`), the server should reject the request and return a `413 Payload Too Large` or `400 Bad Request` status code with a clear, explanatory error message advising the user to narrow their query.

## 7. Benefits

* **Extreme Performance**: Achieves high-throughput data transfer far exceeding JSON by using the binary Arrow format.
* **Seamless Ecosystem Integration**: Becomes a first-class citizen in the modern data science and ML ecosystem, with data consumable by Pandas, PyTorch, and other tools.
* **Enables MLOps Workflows**: Functions as a high-performance online feature server, providing data directly for model training and inference.
* **Flexibility and Control**: Provides users with fine-grained control to extract the exact data subset and dimension layout they need via a rich set of query parameters.