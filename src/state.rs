//! Application state management for rossby.
//!
//! This module defines the shared state that is passed to all handlers,
//! containing the loaded NetCDF data and metadata.

use ndarray::{Array, IxDyn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;

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
#[derive(Debug)]
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
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            metadata,
            data,
        })
    }

    /// Get a variable's data array
    pub fn get_variable(&self, name: &str) -> Option<&Array<f32, IxDyn>> {
        self.data.get(name)
    }

    /// Get coordinate values for a dimension
    pub fn get_coordinate(&self, name: &str) -> Option<&Vec<f64>> {
        self.metadata.coordinates.get(name)
    }

    /// Get variable metadata
    pub fn get_variable_metadata(&self, name: &str) -> Option<&Variable> {
        self.metadata.variables.get(name)
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
