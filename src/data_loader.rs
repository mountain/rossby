//! NetCDF data loading functionality.
//!
//! This module handles reading NetCDF files and loading them into memory.
//! It converts NetCDF variables and metadata into a format that can be efficiently
//! accessed by the application.

use ndarray::{Array, Dim, IxDyn};
use netcdf::{self, Attribute, Variable as NetCDFVariable};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::error::{Result, RossbyError};
use crate::state::{AppState, AttributeValue, Dimension, Metadata, Variable};

/// Type alias for the NetCDF loading result to simplify the complex return type
pub type LoadResult = Result<(Metadata, HashMap<String, Array<f32, IxDyn>>)>;

/// Load a NetCDF file into memory and create the application state
pub fn load_netcdf(path: &Path, config: Config) -> Result<AppState> {
    // Load the NetCDF data and metadata
    let (metadata, data) = load_netcdf_file(path)?;

    // Validate the loaded data
    validate_netcdf_data(&metadata, &data)?;

    // Create the application state directly
    let app_state = AppState {
        config,
        metadata,
        data,
    };

    Ok(app_state)
}

/// Load a NetCDF file into memory, returning metadata and data
fn load_netcdf_file(path: &Path) -> LoadResult {
    // Check if the file exists
    if !path.exists() {
        return Err(RossbyError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        )));
    }

    // Open the NetCDF file
    let file = match netcdf::open(path) {
        Ok(f) => f,
        Err(e) => {
            return Err(RossbyError::NetCdf {
                message: format!("Failed to open NetCDF file: {}", e),
            });
        }
    };

    info!("Opened NetCDF file: {}", path.display());
    let variables_count = file.variables().count();
    let dimensions_count = file.dimensions().count();
    debug!("File has {} variables", variables_count);
    debug!("File has {} dimensions", dimensions_count);

    // Extract file metadata
    let metadata = extract_metadata(&file)?;

    // Extract data from variables
    let data = extract_data(&file, &metadata)?;

    Ok((metadata, data))
}

/// Extract metadata from the NetCDF file
fn extract_metadata(file: &netcdf::File) -> Result<Metadata> {
    // Extract global attributes
    let mut global_attributes = HashMap::new();
    for attr in file.attributes() {
        let value = convert_attribute(&attr)?;
        global_attributes.insert(attr.name().to_string(), value);
    }

    // Extract dimensions
    let mut dimensions = HashMap::new();
    for dim in file.dimensions() {
        let dimension = Dimension {
            name: dim.name().to_string(),
            size: dim.len(),
            is_unlimited: dim.is_unlimited(),
        };
        dimensions.insert(dim.name().to_string(), dimension);
    }

    // Extract variables and their metadata
    let mut variables = HashMap::new();
    let mut coordinates = HashMap::new();

    for var in file.variables() {
        // Skip variables we can't handle (non-numeric types)
        if !is_supported_variable(&var) {
            warn!("Skipping unsupported variable: {}", var.name());
            continue;
        }

        // Extract variable dimensions
        let var_dims: Vec<String> = var
            .dimensions()
            .iter()
            .map(|dim| dim.name().to_string())
            .collect();

        // Extract variable shape
        let var_shape: Vec<usize> = var_dims
            .iter()
            .map(|name| file.dimension(name).unwrap().len())
            .collect();

        // Extract variable attributes
        let mut var_attrs = HashMap::new();
        for attr in var.attributes() {
            let value = convert_attribute(&attr)?;
            var_attrs.insert(attr.name().to_string(), value);
        }

        // Create variable metadata
        let variable = Variable {
            name: var.name().to_string(),
            dimensions: var_dims,
            shape: var_shape,
            attributes: var_attrs,
            dtype: format!("{:?}", var.vartype()),
        };

        variables.insert(var.name().to_string(), variable);

        // If this is a coordinate variable (name matches a dimension),
        // extract the coordinate values
        if file.dimension(&var.name()).is_some() {
            let coord_values = extract_coordinate_values(&var)?;
            coordinates.insert(var.name().to_string(), coord_values);
        }
    }

    // Check for missing coordinate variables and create them if needed
    for dim_name in dimensions.keys() {
        if !coordinates.contains_key(dim_name) {
            // Create a default coordinate (0-based indices)
            let dim_size = dimensions[dim_name].size;
            let coord_values: Vec<f64> = (0..dim_size).map(|i| i as f64).collect();
            coordinates.insert(dim_name.to_string(), coord_values);

            warn!("Created default coordinates for dimension: {}", dim_name);
        }
    }

    Ok(Metadata {
        global_attributes,
        dimensions,
        variables,
        coordinates,
    })
}

/// Check if a variable has a supported type that we can work with
fn is_supported_variable(var: &NetCDFVariable) -> bool {
    use netcdf::types::{BasicType, VariableType};

    matches!(
        var.vartype(),
        VariableType::Basic(BasicType::Byte)
            | VariableType::Basic(BasicType::Char)
            | VariableType::Basic(BasicType::Short)
            | VariableType::Basic(BasicType::Int)
            | VariableType::Basic(BasicType::Int64)
            | VariableType::Basic(BasicType::Float)
            | VariableType::Basic(BasicType::Double)
    )
}

/// Convert a NetCDF attribute to our AttributeValue enum
fn convert_attribute(attr: &Attribute) -> Result<AttributeValue> {
    use netcdf::AttributeValue as NcAttributeValue;

    // The new API returns an AttributeValue enum directly
    let value = attr.value()?;

    match value {
        // String types
        NcAttributeValue::Str(s) => Ok(AttributeValue::Text(s)),

        // Numeric types - store as f64 for simplicity
        NcAttributeValue::Uchar(v) => Ok(AttributeValue::Number(v as f64)),
        NcAttributeValue::Schar(v) => Ok(AttributeValue::Number(v as f64)),
        NcAttributeValue::Short(v) => Ok(AttributeValue::Number(v as f64)),
        NcAttributeValue::Int(v) => Ok(AttributeValue::Number(v as f64)),
        NcAttributeValue::Float(v) => Ok(AttributeValue::Number(v as f64)),
        NcAttributeValue::Double(v) => Ok(AttributeValue::Number(v)),

        // For array types, the netcdf crate now returns a Vec<T>, but we need to check the API
        // to see what the exact variants are
        _ => {
            // Convert any other types to a text representation for now
            Ok(AttributeValue::Text(format!("{:?}", value)))
        }
    }
}

/// Extract coordinate values from a coordinate variable - reading one value at a time
fn extract_coordinate_values(var: &NetCDFVariable) -> Result<Vec<f64>> {
    use netcdf::types::{BasicType, VariableType};

    // Get the dimension size
    let dim_size = var.dimensions()[0].len();
    let mut values = Vec::with_capacity(dim_size);

    // Read each value individually based on the variable type
    match var.vartype() {
        VariableType::Basic(BasicType::Byte) => {
            for i in 0..dim_size {
                let index = [i]; // Use a fixed-size array instead of Vec
                let value: i8 = var.get_value(index)?;
                values.push(value as f64);
            }
        }
        VariableType::Basic(BasicType::Short) => {
            for i in 0..dim_size {
                let index = [i];
                let value: i16 = var.get_value(index)?;
                values.push(value as f64);
            }
        }
        VariableType::Basic(BasicType::Int) => {
            for i in 0..dim_size {
                let index = [i];
                let value: i32 = var.get_value(index)?;
                values.push(value as f64);
            }
        }
        VariableType::Basic(BasicType::Int64) => {
            for i in 0..dim_size {
                let index = [i];
                let value: i64 = var.get_value(index)?;
                values.push(value as f64);
            }
        }
        VariableType::Basic(BasicType::Float) => {
            for i in 0..dim_size {
                let index = [i];
                let value: f32 = var.get_value(index)?;
                values.push(value as f64);
            }
        }
        VariableType::Basic(BasicType::Double) => {
            for i in 0..dim_size {
                let index = [i];
                let value: f64 = var.get_value(index)?;
                values.push(value);
            }
        }
        _ => {
            // For unsupported types, create a sequence of indices
            for i in 0..dim_size {
                values.push(i as f64);
            }
            warn!(
                "Unsupported coordinate variable type: {:?}, using indices instead",
                var.vartype()
            );
        }
    }

    Ok(values)
}

/// Extract data from the NetCDF variables
fn extract_data(
    file: &netcdf::File,
    metadata: &Metadata,
) -> Result<HashMap<String, Array<f32, IxDyn>>> {
    let mut data = HashMap::new();

    for var_name in metadata.variables.keys() {
        if let Some(var) = file.variable(var_name) {
            // Only process variables we can handle
            if !is_supported_variable(&var) {
                continue;
            }

            // Get the variable's shape
            let shape = &metadata.variables[var_name].shape;

            // Convert the data to f32 array
            let array = convert_variable_to_array(&var, shape)?;
            data.insert(var_name.clone(), array);
        }
    }

    Ok(data)
}

/// Convert a NetCDF variable to an ndarray Array<f32, IxDyn> - reading one value at a time
fn convert_variable_to_array(var: &NetCDFVariable, shape: &[usize]) -> Result<Array<f32, IxDyn>> {
    use netcdf::types::{BasicType, VariableType};

    // Create the shape for the ndarray
    let dim = Dim(shape.to_vec());

    // Total number of elements
    let total_elements = shape.iter().product();
    let mut data = Vec::with_capacity(total_elements);

    // Multi-dimensional array indices
    let mut indices = vec![0; shape.len()];

    // Read each value individually based on the variable type
    match var.vartype() {
        VariableType::Basic(BasicType::Byte) => {
            // Use a fixed-size array to hold the indices
            let mut index_array = [0; 10]; // Most NetCDF files won't have more than 10 dimensions

            for i in 0..total_elements {
                // Convert flat index to multi-dimensional indices
                compute_indices(&mut indices, i, shape);

                // Copy from vec to array (only up to shape.len() elements)
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: i8 = var.get_value(&index_array[..shape.len()])?;
                data.push(value as f32);
            }
        }
        VariableType::Basic(BasicType::Short) => {
            let mut index_array = [0; 10];

            for i in 0..total_elements {
                compute_indices(&mut indices, i, shape);
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: i16 = var.get_value(&index_array[..shape.len()])?;
                data.push(value as f32);
            }
        }
        VariableType::Basic(BasicType::Int) => {
            let mut index_array = [0; 10];

            for i in 0..total_elements {
                compute_indices(&mut indices, i, shape);
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: i32 = var.get_value(&index_array[..shape.len()])?;
                data.push(value as f32);
            }
        }
        VariableType::Basic(BasicType::Int64) => {
            let mut index_array = [0; 10];

            for i in 0..total_elements {
                compute_indices(&mut indices, i, shape);
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: i64 = var.get_value(&index_array[..shape.len()])?;
                data.push(value as f32);
            }
        }
        VariableType::Basic(BasicType::Float) => {
            let mut index_array = [0; 10];

            for i in 0..total_elements {
                compute_indices(&mut indices, i, shape);
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: f32 = var.get_value(&index_array[..shape.len()])?;
                data.push(value);
            }
        }
        VariableType::Basic(BasicType::Double) => {
            let mut index_array = [0; 10];

            for i in 0..total_elements {
                compute_indices(&mut indices, i, shape);
                index_array[..shape.len()].copy_from_slice(&indices[..shape.len()]);

                let value: f64 = var.get_value(&index_array[..shape.len()])?;
                data.push(value as f32);
            }
        }
        _ => {
            return Err(RossbyError::NetCdf {
                message: format!("Unsupported variable type: {:?}", var.vartype()),
            })
        }
    }

    // Create the ndarray from the collected data
    let array = Array::from_shape_vec(dim, data)?;
    Ok(array)
}

/// Helper function to convert a flat index to multi-dimensional indices
fn compute_indices(indices: &mut [usize], flat_index: usize, shape: &[usize]) {
    let mut remaining = flat_index;
    for (i, &dim_size) in shape.iter().enumerate().rev() {
        indices[i] = remaining % dim_size;
        remaining /= dim_size;
    }
}

/// Create a super simplified test NetCDF file - focusing only on making valid data
#[cfg(test)]
fn create_test_netcdf_file(path: &Path) -> Result<()> {
    use std::fs;

    // Create a very basic netCDF file with the minimal structure required for tests
    let mut file = netcdf::create(path)?;

    // Add global attributes
    file.add_attribute("title", "Rossby Test File")?;
    file.add_attribute("source", "test")?;

    // First, add all dimensions
    let lon_size = 2;
    let lat_size = 2;
    let time_size = 2;

    file.add_dimension("lon", lon_size)?;
    file.add_dimension("lat", lat_size)?;
    file.add_dimension("time", time_size)?;

    // Then create coordinate variables one at a time
    {
        // Define and write lon coordinate - one value at a time
        let mut lon_var = file.add_variable::<f64>("lon", &["lon"])?;
        lon_var.put_attribute("units", "degrees_east")?;
        lon_var.put_value(0.0, &[0])?;
        lon_var.put_value(1.0, &[1])?;
    }

    {
        // Define and write lat coordinate - one value at a time
        let mut lat_var = file.add_variable::<f64>("lat", &["lat"])?;
        lat_var.put_attribute("units", "degrees_north")?;
        lat_var.put_value(0.0, &[0])?;
        lat_var.put_value(1.0, &[1])?;
    }

    {
        // Define and write time coordinate - one value at a time
        let mut time_var = file.add_variable::<f64>("time", &["time"])?;
        time_var.put_attribute("units", "days since 2000-01-01")?;
        time_var.put_value(0.0, &[0])?;
        time_var.put_value(1.0, &[1])?;
    }

    {
        // Define and write temperature data - one value at a time
        let mut temp_var = file.add_variable::<f32>("temperature", &["time", "lat", "lon"])?;
        temp_var.put_attribute("units", "K")?;
        temp_var.put_attribute("long_name", "Temperature")?;

        // Write 2x2x2 array one value at a time
        for t in 0..time_size {
            for y in 0..lat_size {
                for x in 0..lon_size {
                    let value = (t * lat_size * lon_size + y * lon_size + x) as f32;
                    // Write to position [t, y, x]
                    temp_var.put_value(value, &[t, y, x])?;
                }
            }
        }
    }

    // Sync to ensure all data is written
    file.sync()?;

    // Verify the file was created correctly
    let file_verify = netcdf::open(path)?;
    println!("TEST FILE CREATED with dimensions:");
    for dim in file_verify.dimensions() {
        println!("  Dimension '{}' has size {}", dim.name(), dim.len());
    }

    // Print variable information to help debug
    println!("TEST FILE VARIABLES:");
    for var in file_verify.variables() {
        println!(
            "  Variable '{}' dimensions: {:?}",
            var.name(),
            var.dimensions()
        );
        if let Ok(values) = var.get_values::<f32, _>(&[] as &[netcdf::Extent]) {
            println!("    Values (as f32): {:?}", values);
        } else if let Ok(values) = var.get_values::<f64, _>(&[] as &[netcdf::Extent]) {
            println!("    Values (as f64): {:?}", values);
        }
    }

    Ok(())
}

/// Validate the loaded NetCDF data for consistency
fn validate_netcdf_data(
    metadata: &Metadata,
    data: &HashMap<String, Array<f32, IxDyn>>,
) -> Result<()> {
    // Check if we have any variables
    if metadata.variables.is_empty() {
        return Err(RossbyError::DataNotFound {
            message: "No variables found in NetCDF file".to_string(),
        });
    }

    // Check if dimensions match variables
    for (var_name, var) in &metadata.variables {
        // Check that the variable has dimensions
        if var.dimensions.is_empty() {
            return Err(RossbyError::DataNotFound {
                message: format!("Variable {} has no dimensions", var_name),
            });
        }

        // Check that all dimensions exist
        for dim_name in &var.dimensions {
            if !metadata.dimensions.contains_key(dim_name) {
                return Err(RossbyError::DataNotFound {
                    message: format!(
                        "Variable {} references non-existent dimension {}",
                        var_name, dim_name
                    ),
                });
            }
        }

        // Check that the data array exists and has the right shape
        if let Some(array) = data.get(var_name) {
            let shape = array.shape();

            // Check that the number of dimensions match
            if shape.len() != var.dimensions.len() {
                return Err(RossbyError::DataNotFound {
                    message: format!(
                        "Variable {} has inconsistent dimensions: metadata has {}, data has {}",
                        var_name,
                        var.dimensions.len(),
                        shape.len()
                    ),
                });
            }

            // Check that each dimension size matches
            for (i, dim_name) in var.dimensions.iter().enumerate() {
                let expected_size = metadata.dimensions[dim_name].size;
                if shape[i] != expected_size {
                    return Err(RossbyError::DataNotFound {
                        message: format!(
                            "Variable {} dimension {} has inconsistent size: expected {}, got {}",
                            var_name, dim_name, expected_size, shape[i]
                        ),
                    });
                }
            }
        } else {
            return Err(RossbyError::DataNotFound {
                message: format!("Data array for variable {} not found", var_name),
            });
        }
    }

    // Check for coordinate variables
    for dim_name in metadata.dimensions.keys() {
        if !metadata.coordinates.contains_key(dim_name) {
            return Err(RossbyError::DataNotFound {
                message: format!("Coordinate values for dimension {} not found", dim_name),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Test loading a real climate data file
    #[test]
    fn test_real_climate_data() -> Result<()> {
        let file_path = Path::new("tests/fixtures/2m_temperature_1982_5.625deg.nc");
        if !file_path.exists() {
            println!("Skipping test_real_climate_data as fixture file is not present");
            return Ok(());
        }

        println!("Loading real climate data from: {}", file_path.display());

        // Load the file
        let (metadata, data) = load_netcdf_file(file_path)?;

        // Verify dimensions
        assert!(metadata.dimensions.contains_key("time"));
        assert!(metadata.dimensions.contains_key("lat"));
        assert!(metadata.dimensions.contains_key("lon"));

        assert_eq!(metadata.dimensions["time"].size, 53);
        assert_eq!(metadata.dimensions["lat"].size, 32);
        assert_eq!(metadata.dimensions["lon"].size, 64);

        // Verify variables
        assert!(metadata.variables.contains_key("t2m"));
        assert!(metadata.variables.contains_key("lat"));
        assert!(metadata.variables.contains_key("lon"));
        assert!(metadata.variables.contains_key("time"));

        // Verify coordinates
        assert!(metadata.coordinates.contains_key("lat"));
        assert!(metadata.coordinates.contains_key("lon"));
        assert!(metadata.coordinates.contains_key("time"));

        assert_eq!(metadata.coordinates["lat"].len(), 32);
        assert_eq!(metadata.coordinates["lon"].len(), 64);
        assert_eq!(metadata.coordinates["time"].len(), 53);

        // Check some specific coordinate values
        assert_eq!(metadata.coordinates["lat"][0], -87.1875);
        assert_eq!(metadata.coordinates["lon"][0], 0.0);

        // Verify the data arrays
        assert!(data.contains_key("t2m"));
        assert!(data.contains_key("lat"));
        assert!(data.contains_key("lon"));
        assert!(data.contains_key("time"));

        // Check the temperature data array
        let t2m_data = &data["t2m"];
        assert_eq!(t2m_data.shape(), &[53, 32, 64]);

        // Verify the first temperature value (approximately)
        let first_value = t2m_data[[0, 0, 0]];
        let expected_value = 251.48; // From the inspection result 251.47910694200166
        assert!(
            (first_value - expected_value).abs() < 0.01,
            "First value {} should be close to expected {}",
            first_value,
            expected_value
        );

        println!("Real climate data loaded and verified successfully");

        Ok(())
    }

    // Extremely minimal test to understand how the netcdf API works
    #[test]
    fn test_basic_netcdf() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory for the test file
        let dir = tempdir()?;
        let file_path = dir.path().join("minimal_test.nc");

        println!("Creating a minimal NetCDF file at: {}", file_path.display());

        // Create the file
        let mut file = netcdf::create(&file_path)?;

        // Add dimension
        println!("Adding dimension 'x' with size 2");
        let _x_dim = file.add_dimension("x", 2)?;

        // Add variable
        println!("Adding variable 'data' with dimension 'x'");
        let mut var = file.add_variable::<f32>("data", &["x"])?;

        // Try to add data in several different ways until one works

        println!("METHOD 1: Using empty extents array");
        let data = vec![1.0f32, 2.0f32];
        match var.put_values(&data, &[] as &[netcdf::Extent]) {
            Ok(_) => println!("SUCCESS: Method 1 worked"),
            Err(e) => println!("FAILED: Method 1 error: {}", e),
        }

        println!("METHOD 3: Writing one value at a time");
        match var.put_value(1.0f32, &[0]) {
            Ok(_) => println!("SUCCESS: Method 3a worked (first value)"),
            Err(e) => println!("FAILED: Method 3a error: {}", e),
        }

        match var.put_value(2.0f32, &[1]) {
            Ok(_) => println!("SUCCESS: Method 3b worked (second value)"),
            Err(e) => println!("FAILED: Method 3b error: {}", e),
        }

        // Save file
        println!("Syncing file");
        file.sync()?;

        // Read the file back
        println!("\nReading file back");
        let file = netcdf::open(&file_path)?;

        // Check dimensions
        println!("Checking dimensions:");
        for dim in file.dimensions() {
            println!("  Dimension '{}' size: {}", dim.name(), dim.len());
        }

        // Check variables
        println!("Checking variables:");
        for var in file.variables() {
            println!(
                "  Variable '{}' dimensions: {:?}",
                var.name(),
                var.dimensions()
            );

            // Try to read values
            match var.get_values::<f32, _>(&[] as &[netcdf::Extent]) {
                Ok(values) => println!("  Values: {:?}", values),
                Err(e) => println!("  Error reading values: {}", e),
            }
        }

        Ok(())
    }

    #[test]
    fn test_file_not_found() {
        let result = load_netcdf_file(Path::new("/nonexistent/file.nc"));
        assert!(result.is_err());
        match result.unwrap_err() {
            RossbyError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            _ => panic!("Expected IO error"),
        }
    }

    #[test]
    fn test_netcdf_loading() -> Result<()> {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");

        // Create a test NetCDF file
        create_test_netcdf_file(&file_path)?;

        // Load the file
        let (metadata, data) = load_netcdf_file(&file_path)?;

        // Simplified verification based on our new test file structure
        assert!(metadata.global_attributes.contains_key("title"));
        assert!(metadata.dimensions.contains_key("lon"));
        assert!(metadata.dimensions.contains_key("lat"));
        assert!(metadata.dimensions.contains_key("time"));
        assert!(metadata.variables.contains_key("temperature"));
        assert!(metadata.coordinates.contains_key("lon"));

        // Check specific values with the smaller dimensions
        assert_eq!(metadata.dimensions["lon"].size, 2);
        assert_eq!(metadata.dimensions["lat"].size, 2);
        assert_eq!(metadata.dimensions["time"].size, 2);
        assert_eq!(metadata.variables["temperature"].dimensions.len(), 3);

        // Check coordinates
        assert_eq!(metadata.coordinates["lon"], vec![0.0, 1.0]);
        assert_eq!(metadata.coordinates["lat"], vec![0.0, 1.0]);
        assert_eq!(metadata.coordinates["time"], vec![0.0, 1.0]);

        // Verify the data
        assert!(data.contains_key("temperature"));
        let temp_data = &data["temperature"];
        assert_eq!(temp_data.shape(), &[2, 2, 2]);

        // Check the first few values
        assert_eq!(temp_data[[0, 0, 0]], 0.0);
        assert_eq!(temp_data[[0, 0, 1]], 1.0);
        assert_eq!(temp_data[[0, 1, 0]], 2.0);

        Ok(())
    }

    #[test]
    fn test_attribute_conversion() -> Result<()> {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_attr.nc");

        // Create a test NetCDF file with debugging output
        println!("Creating test NetCDF file for attribute conversion test");
        create_test_netcdf_file(&file_path)?;
        println!("Test file created successfully");

        // Load the file with debugging
        println!("Loading NetCDF file for attribute test");
        let (metadata, _) = load_netcdf_file(&file_path)?;
        println!("File loaded successfully");

        // Debugging output
        println!("Global attributes: {:?}", metadata.global_attributes.keys());
        for (k, v) in &metadata.global_attributes {
            println!("  Global attribute '{}': {:?}", k, v);
        }

        println!("Variables: {:?}", metadata.variables.keys());
        for (name, var) in &metadata.variables {
            println!(
                "  Variable '{}' attributes: {:?}",
                name,
                var.attributes.keys()
            );
        }

        // Check global attributes
        match &metadata.global_attributes["title"] {
            AttributeValue::Text(text) => {
                println!("Title attribute value: {}", text);
                assert_eq!(text, "Rossby Test File");
            }
            _ => panic!("Expected Text attribute"),
        }

        // Check variable attributes
        match &metadata.variables["temperature"].attributes["units"] {
            AttributeValue::Text(text) => {
                println!("Temperature units attribute value: {}", text);
                assert_eq!(text, "K");
            }
            _ => panic!("Expected Text attribute"),
        }

        match &metadata.variables["temperature"].attributes["long_name"] {
            AttributeValue::Text(text) => {
                println!("Temperature long_name attribute value: {}", text);
                assert_eq!(text, "Temperature");
            }
            _ => panic!("Expected Text attribute"),
        }

        Ok(())
    }

    #[test]
    fn test_validation() -> Result<()> {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_valid.nc");

        // Create a test NetCDF file with debugging output
        println!("Creating test NetCDF file for validation test");
        create_test_netcdf_file(&file_path)?;
        println!("Test file created successfully");

        // Load the file with debugging
        println!("Loading NetCDF file for validation test");
        let (metadata, data) = load_netcdf_file(&file_path)?;
        println!("File loaded successfully");

        // Print debugging information
        println!("Metadata dimensions: {:?}", metadata.dimensions.keys());
        println!("Metadata variables: {:?}", metadata.variables.keys());
        println!("Metadata coordinates: {:?}", metadata.coordinates.keys());
        println!("Data variables: {:?}", data.keys());

        // Validation should pass
        println!("Running validation...");
        let validation_result = validate_netcdf_data(&metadata, &data);
        if let Err(e) = &validation_result {
            println!("Validation failed: {:?}", e);
        } else {
            println!("Validation passed");
        }

        assert!(validation_result.is_ok());

        Ok(())
    }
}
