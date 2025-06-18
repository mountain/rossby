//! Application state management for rossby.
//!
//! This module defines the shared state that is passed to all handlers,
//! containing the loaded NetCDF data and metadata.

use ndarray::{Array, IxDyn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;
use crate::error::{Result, RossbyError};

/// Metadata about a NetCDF dimension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    /// Name of the dimension
    pub name: String,
    /// Size of the dimension
    pub size: usize,
    /// Whether this dimension is unlimited
    pub is_unlimited: bool,
}

/// Metadata about a NetCDF variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Name of the variable
    pub name: String,
    /// Dimensions of the variable
    pub dimensions: Vec<String>,
    /// Shape of the variable (dimension sizes)
    pub shape: Vec<usize>,
    /// Variable attributes
    pub attributes: HashMap<String, AttributeValue>,
    /// Data type as string
    pub dtype: String,
}

/// Possible attribute values in NetCDF
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// String attribute
    Text(String),
    /// Numeric attribute (stored as f64 for simplicity)
    Number(f64),
    /// Array of numbers
    NumberArray(Vec<f64>),
}

/// Complete metadata for a NetCDF file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// File-level attributes
    pub global_attributes: HashMap<String, AttributeValue>,
    /// Dimensions in the file
    pub dimensions: HashMap<String, Dimension>,
    /// Variables in the file
    pub variables: HashMap<String, Variable>,
    /// Coordinate variables (subset of variables that match dimension names)
    pub coordinates: HashMap<String, Vec<f64>>,
}

/// The main application state shared across all handlers
#[derive(Debug, Clone)]
pub struct AppState {
    /// Configuration
    pub config: Config,
    /// File metadata
    pub metadata: Metadata,
    /// Loaded data arrays
    pub data: HashMap<String, Array<f32, IxDyn>>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        config: Config,
        metadata: Metadata,
        data: HashMap<String, Array<f32, IxDyn>>,
    ) -> Self {
        Self {
            config,
            metadata,
            data,
        }
    }
    
    /// Create a new AppState wrapped in an Arc for shared ownership
    pub fn new_shared(
        config: Config,
        metadata: Metadata,
        data: HashMap<String, Array<f32, IxDyn>>,
    ) -> Arc<Self> {
        Arc::new(Self::new(config, metadata, data))
    }

        /// Get a variable's data array
    pub fn get_variable(&self, name: &str) -> Option<&Array<f32, IxDyn>> {
        self.data.get(name)
    }

    /// Get a variable's data array with error handling
    pub fn get_variable_checked(&self, name: &str) -> Result<&Array<f32, IxDyn>> {
        self.data.get(name).ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable not found: {}", name),
        })
    }

    /// Get coordinate values for a dimension
    pub fn get_coordinate(&self, name: &str) -> Option<&Vec<f64>> {
        self.metadata.coordinates.get(name)
    }

    /// Get coordinate values for a dimension with error handling
    pub fn get_coordinate_checked(&self, name: &str) -> Result<&Vec<f64>> {
        self.metadata.coordinates.get(name).ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Coordinate not found: {}", name),
        })
    }

    /// Get variable metadata
    pub fn get_variable_metadata(&self, name: &str) -> Option<&Variable> {
        self.metadata.variables.get(name)
    }
    
    /// Get variable metadata with error handling
    pub fn get_variable_metadata_checked(&self, name: &str) -> Result<&Variable> {
        self.metadata.variables.get(name).ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable metadata not found: {}", name),
        })
    }
    
    /// Check if a variable exists
    pub fn has_variable(&self, name: &str) -> bool {
        self.metadata.variables.contains_key(name)
    }
    
    /// Find the index of a coordinate value within its array
    /// Returns the nearest index if exact match is not found
    pub fn find_coordinate_index(&self, dim_name: &str, value: f64) -> Result<usize> {
        let coords = self.get_coordinate_checked(dim_name)?;
        
        // Early return for empty coordinates (shouldn't happen in valid files)
        if coords.is_empty() {
            return Err(RossbyError::DataNotFound {
                message: format!("Coordinate {} is empty", dim_name),
            });
        }
        
        // Check if the value is out of bounds
        if value < coords[0] || value > coords[coords.len() - 1] {
            return Err(RossbyError::InvalidCoordinates {
                message: format!(
                    "Coordinate value {} is outside the range of {} ({} to {})",
                    value, dim_name, coords[0], coords[coords.len() - 1]
                ),
            });
        }
        
        // Find the index of the closest coordinate
        let mut closest_idx = 0;
        let mut min_diff = f64::MAX;
        
        for (i, &coord) in coords.iter().enumerate() {
            let diff = (coord - value).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_idx = i;
            }
        }
        
        Ok(closest_idx)
    }
    
    /// Get the variable dimensions
    pub fn get_variable_dimensions(&self, var_name: &str) -> Result<Vec<String>> {
        let var_meta = self.get_variable_metadata_checked(var_name)?;
        Ok(var_meta.dimensions.clone())
    }
    
    /// Validate that the application state is consistent and ready for use
    pub fn validate(&self) -> Result<()> {
        // Ensure we have at least one variable
        if self.metadata.variables.is_empty() {
            return Err(RossbyError::DataNotFound {
                message: "No variables found in the NetCDF file".to_string(),
            });
        }
        
        // Validate that all referenced dimensions exist
        for (var_name, var) in &self.metadata.variables {
            for dim_name in &var.dimensions {
                if !self.metadata.dimensions.contains_key(dim_name) {
                    return Err(RossbyError::DataNotFound {
                        message: format!(
                            "Variable {} references non-existent dimension {}",
                            var_name, dim_name
                        ),
                    });
                }
            }
        }
        
        // Validate that the data arrays match their metadata shape
        for (var_name, var) in &self.metadata.variables {
            if let Some(data) = self.data.get(var_name) {
                let shape = data.shape();
                if shape.len() != var.shape.len() {
                    return Err(RossbyError::DataNotFound {
                        message: format!(
                            "Variable {} has inconsistent dimensions between metadata ({:?}) and data ({:?})",
                            var_name, var.shape, shape
                        ),
                    });
                }
                
                for (i, &dim_size) in var.shape.iter().enumerate() {
                    if shape[i] != dim_size {
                        return Err(RossbyError::DataNotFound {
                            message: format!(
                                "Variable {} has inconsistent dimension size at index {}: metadata={}, data={}",
                                var_name, i, dim_size, shape[i]
                            ),
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_value_serialization() {
        let text = AttributeValue::Text("test".to_string());
        let json = serde_json::to_string(&text).unwrap();
        assert_eq!(json, r#""test""#);

        let number = AttributeValue::Number(42.0);
        let json = serde_json::to_string(&number).unwrap();
        assert_eq!(json, "42.0");

        let array = AttributeValue::NumberArray(vec![1.0, 2.0, 3.0]);
        let json = serde_json::to_string(&array).unwrap();
        assert_eq!(json, "[1.0,2.0,3.0]");
    }

    #[test]
    fn test_metadata_structure() {
        let mut metadata = Metadata {
            global_attributes: HashMap::new(),
            dimensions: HashMap::new(),
            variables: HashMap::new(),
            coordinates: HashMap::new(),
        };

        metadata.dimensions.insert(
            "time".to_string(),
            Dimension {
                name: "time".to_string(),
                size: 10,
                is_unlimited: true,
            },
        );

        assert_eq!(metadata.dimensions.get("time").unwrap().size, 10);
        assert!(metadata.dimensions.get("time").unwrap().is_unlimited);
    }
}
