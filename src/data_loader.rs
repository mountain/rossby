//! NetCDF data loading functionality.
//!
//! This module handles reading NetCDF files and loading them into memory.

use ndarray::{Array, IxDyn};
use std::collections::HashMap;
use std::path::Path;

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
    // TODO: Implement NetCDF loading
    // This is a placeholder that will be implemented in Phase 4

    // Check if the file exists
    if !path.exists() {
        return Err(RossbyError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path.display()),
        )));
    }

    // In Phase 4, we'll replace this with actual NetCDF loading code
    // For now, create placeholder metadata and data structures
    let metadata = create_placeholder_metadata(path);
    let data = create_placeholder_data(&metadata);

    Ok((metadata, data))
}

/// Create placeholder metadata for testing
fn create_placeholder_metadata(path: &Path) -> Metadata {
    // This will be replaced with actual loading code in Phase 4
    // For now, just create some basic metadata for development

    let mut global_attributes = HashMap::new();
    global_attributes.insert(
        "title".to_string(),
        AttributeValue::Text("Rossby Development File".to_string()),
    );
    global_attributes.insert(
        "source".to_string(),
        AttributeValue::Text(format!("File: {}", path.display())),
    );

    let mut dimensions = HashMap::new();
    dimensions.insert(
        "lon".to_string(),
        Dimension {
            name: "lon".to_string(),
            size: 180,
            is_unlimited: false,
        },
    );
    dimensions.insert(
        "lat".to_string(),
        Dimension {
            name: "lat".to_string(),
            size: 90,
            is_unlimited: false,
        },
    );
    dimensions.insert(
        "time".to_string(),
        Dimension {
            name: "time".to_string(),
            size: 10,
            is_unlimited: true,
        },
    );

    let mut variables = HashMap::new();
    let mut temp_attrs = HashMap::new();
    temp_attrs.insert("units".to_string(), AttributeValue::Text("K".to_string()));
    temp_attrs.insert(
        "long_name".to_string(),
        AttributeValue::Text("Temperature".to_string()),
    );

    variables.insert(
        "temperature".to_string(),
        Variable {
            name: "temperature".to_string(),
            dimensions: vec!["time".to_string(), "lat".to_string(), "lon".to_string()],
            shape: vec![10, 90, 180],
            attributes: temp_attrs,
            dtype: "f32".to_string(),
        },
    );

    let mut coordinates = HashMap::new();
    coordinates.insert("lon".to_string(), (-180..180).map(|i| i as f64).collect());
    coordinates.insert("lat".to_string(), (-45..45).map(|i| i as f64).collect());
    coordinates.insert("time".to_string(), (0..10).map(|i| i as f64).collect());

    Metadata {
        global_attributes,
        dimensions,
        variables,
        coordinates,
    }
}

/// Create placeholder data arrays for testing
fn create_placeholder_data(metadata: &Metadata) -> HashMap<String, Array<f32, IxDyn>> {
    // This will be replaced with actual loading code in Phase 4
    let mut data = HashMap::new();

    // In Phase 4, this will load real data from the NetCDF file
    // For now, just create some empty arrays with the right dimensions
    for (name, var) in &metadata.variables {
        // Create a shape vector from the variable's dimensions
        let shape: Vec<_> = var.shape.to_vec();

        // Create an empty array with the right shape
        // In Phase 4, this will be filled with actual data from the NetCDF file
        let array = Array::<f32, _>::zeros(shape);

        data.insert(name.clone(), array);
    }

    data
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
    use std::fs::File;
    use std::io::Write;
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
    fn test_placeholder_metadata() {
        // Create a temporary file for testing
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");
        File::create(&file_path)
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let metadata = create_placeholder_metadata(&file_path);

        // Verify the placeholder metadata
        assert!(metadata.global_attributes.contains_key("title"));
        assert!(metadata.dimensions.contains_key("lon"));
        assert!(metadata.dimensions.contains_key("lat"));
        assert!(metadata.dimensions.contains_key("time"));
        assert!(metadata.variables.contains_key("temperature"));
        assert!(metadata.coordinates.contains_key("lon"));

        // Check specific values
        assert_eq!(metadata.dimensions["lon"].size, 180);
        assert_eq!(metadata.dimensions["lat"].size, 90);
        assert_eq!(metadata.variables["temperature"].dimensions.len(), 3);
    }

    #[test]
    fn test_placeholder_data() {
        // Create placeholder metadata
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");
        File::create(&file_path)
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let metadata = create_placeholder_metadata(&file_path);
        let data = create_placeholder_data(&metadata);

        // Verify the data
        assert!(data.contains_key("temperature"));
        let temp_data = &data["temperature"];
        assert_eq!(temp_data.shape(), &[10, 90, 180]);
    }

    #[test]
    fn test_validation() {
        // Create valid metadata and data
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.nc");
        File::create(&file_path)
            .unwrap()
            .write_all(b"test")
            .unwrap();

        let metadata = create_placeholder_metadata(&file_path);
        let data = create_placeholder_data(&metadata);

        // Validation should pass
        assert!(validate_netcdf_data(&metadata, &data).is_ok());
    }
}
