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

    matches!(var.vartype(), 
        VariableType::Basic(BasicType::Byte)
        | VariableType::Basic(BasicType::Char)
        | VariableType::Basic(BasicType::Short)
        | VariableType::Basic(BasicType::Int)
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
        },
    }
}

/// Extract coordinate values from a coordinate variable
fn extract_coordinate_values(var: &NetCDFVariable) -> Result<Vec<f64>> {
    use netcdf::types::{BasicType, VariableType};

    match var.vartype() {
        VariableType::Basic(BasicType::Byte) => {
            let values: Vec<i8> = var.get_values::<i8, _>(&[] as &[netcdf::Extent])?;
            Ok(values.into_iter().map(|v| v as f64).collect())
        }
        VariableType::Basic(BasicType::Short) => {
            let values: Vec<i16> = var.get_values::<i16, _>(&[] as &[netcdf::Extent])?;
            Ok(values.into_iter().map(|v| v as f64).collect())
        }
        VariableType::Basic(BasicType::Int) => {
            let values: Vec<i32> = var.get_values::<i32, _>(&[] as &[netcdf::Extent])?;
            Ok(values.into_iter().map(|v| v as f64).collect())
        }
        VariableType::Basic(BasicType::Float) => {
            let values: Vec<f32> = var.get_values::<f32, _>(&[] as &[netcdf::Extent])?;
            Ok(values.into_iter().map(|v| v as f64).collect())
        }
        VariableType::Basic(BasicType::Double) => {
            let values: Vec<f64> = var.get_values::<f64, _>(&[] as &[netcdf::Extent])?;
            Ok(values)
        }
        _ => {
            // For unsupported types, create a sequence of indices
            let indices: Vec<f64> = (0..var.dimensions()[0].len()).map(|i| i as f64).collect();
            warn!(
                "Unsupported coordinate variable type: {:?}, using indices instead",
                var.vartype()
            );
            Ok(indices)
        }
    }
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

/// Convert a NetCDF variable to an ndarray Array<f32, IxDyn>
fn convert_variable_to_array(var: &NetCDFVariable, shape: &[usize]) -> Result<Array<f32, IxDyn>> {
    use netcdf::types::{BasicType, VariableType};

    // Create the shape for the ndarray
    let dim = Dim(shape.to_vec());

    match var.vartype() {
        VariableType::Basic(BasicType::Byte) => {
            let data: Vec<i8> = var.get_values::<i8, _>(&[] as &[netcdf::Extent])?;
            let array = Array::from_shape_vec(dim, data.into_iter().map(|v| v as f32).collect())?;
            Ok(array)
        }
        VariableType::Basic(BasicType::Short) => {
            let data: Vec<i16> = var.get_values::<i16, _>(&[] as &[netcdf::Extent])?;
            let array = Array::from_shape_vec(dim, data.into_iter().map(|v| v as f32).collect())?;
            Ok(array)
        }
        VariableType::Basic(BasicType::Int) => {
            let data: Vec<i32> = var.get_values::<i32, _>(&[] as &[netcdf::Extent])?;
            let array = Array::from_shape_vec(dim, data.into_iter().map(|v| v as f32).collect())?;
            Ok(array)
        }
        VariableType::Basic(BasicType::Float) => {
            let data: Vec<f32> = var.get_values::<f32, _>(&[] as &[netcdf::Extent])?;
            let array = Array::from_shape_vec(dim, data)?;
            Ok(array)
        }
        VariableType::Basic(BasicType::Double) => {
            let data: Vec<f64> = var.get_values::<f64, _>(&[] as &[netcdf::Extent])?;
            let array = Array::from_shape_vec(dim, data.into_iter().map(|v| v as f32).collect())?;
            Ok(array)
        }
        _ => Err(RossbyError::NetCdf {
            message: format!("Unsupported variable type: {:?}", var.vartype()),
        }),
    }
}

/// Create a test NetCDF file with sample data for testing
#[cfg(test)]
fn create_test_netcdf_file(path: &Path) -> Result<()> {
    use netcdf::types::BasicType;

    // Create a new NetCDF file
    let mut file = netcdf::create(path)?;

    // Add dimensions
    let lon_dim = file.add_dimension("lon", 4)?;
    let lat_dim = file.add_dimension("lat", 3)?;
    let time_dim = file.add_unlimited_dimension("time")?;

    // Add variables
    let mut lon_var = file.add_variable::<f64>("lon", &[&lon_dim])?;
    let mut lat_var = file.add_variable::<f64>("lat", &[&lat_dim])?;
    let mut time_var = file.add_variable::<f64>("time", &[&time_dim])?;
    let mut temp_var = file.add_variable::<f32>("temperature", &[&time_dim, &lat_dim, &lon_dim])?;

    // Add attributes
    file.add_attribute("title", "Rossby Test File")?;
    file.add_attribute("source", "test")?;

    lon_var.add_attribute("units", "degrees_east")?;
    lat_var.add_attribute("units", "degrees_north")?;
    time_var.add_attribute("units", "days since 2000-01-01")?;

    temp_var.add_attribute("units", "K")?;
    temp_var.add_attribute("long_name", "Temperature")?;

    // Write data
    lon_var.put_values(&[0.0, 1.0, 2.0, 3.0], None, None)?;
    lat_var.put_values(&[0.0, 1.0, 2.0], None, None)?;
    time_var.put_values(&[0.0, 1.0], None, None)?;

    // Create temperature data (2 time steps, 3 lat, 4 lon = 24 values)
    let temp_data: Vec<f32> = (0..24).map(|i| i as f32).collect();
    temp_var.put_values(&temp_data, None, None)?;

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
    use tempfile::tempdir;

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

        // Verify the metadata
        assert!(metadata.global_attributes.contains_key("title"));
        assert!(metadata.dimensions.contains_key("lon"));
        assert!(metadata.dimensions.contains_key("lat"));
        assert!(metadata.dimensions.contains_key("time"));
        assert!(metadata.variables.contains_key("temperature"));
        assert!(metadata.coordinates.contains_key("lon"));

        // Check specific values
        assert_eq!(metadata.dimensions["lon"].size, 4);
        assert_eq!(metadata.dimensions["lat"].size, 3);
        assert_eq!(metadata.dimensions["time"].size, 2);
        assert_eq!(metadata.variables["temperature"].dimensions.len(), 3);

        // Check coordinates
        assert_eq!(metadata.coordinates["lon"], vec![0.0, 1.0, 2.0, 3.0]);
        assert_eq!(metadata.coordinates["lat"], vec![0.0, 1.0, 2.0]);
        assert_eq!(metadata.coordinates["time"], vec![0.0, 1.0]);

        // Verify the data
        assert!(data.contains_key("temperature"));
        let temp_data = &data["temperature"];
        assert_eq!(temp_data.shape(), &[2, 3, 4]);

        // Check the first few values
        assert_eq!(temp_data[[0, 0, 0]], 0.0);
        assert_eq!(temp_data[[0, 0, 1]], 1.0);
        assert_eq!(temp_data[[0, 0, 2]], 2.0);

        Ok(())
    }

    #[test]
    fn test_attribute_conversion() -> Result<()> {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");

        // Create a test NetCDF file
        create_test_netcdf_file(&file_path)?;

        // Load the file
        let (metadata, _) = load_netcdf_file(&file_path)?;

        // Check global attributes
        match &metadata.global_attributes["title"] {
            AttributeValue::Text(text) => assert_eq!(text, "Rossby Test File"),
            _ => panic!("Expected Text attribute"),
        }

        // Check variable attributes
        match &metadata.variables["temperature"].attributes["units"] {
            AttributeValue::Text(text) => assert_eq!(text, "K"),
            _ => panic!("Expected Text attribute"),
        }

        match &metadata.variables["temperature"].attributes["long_name"] {
            AttributeValue::Text(text) => assert_eq!(text, "Temperature"),
            _ => panic!("Expected Text attribute"),
        }

        Ok(())
    }

    #[test]
    fn test_validation() -> Result<()> {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");

        // Create a test NetCDF file
        create_test_netcdf_file(&file_path)?;

        // Load the file
        let (metadata, data) = load_netcdf_file(&file_path)?;

        // Validation should pass
        assert!(validate_netcdf_data(&metadata, &data).is_ok());

        Ok(())
    }
}
