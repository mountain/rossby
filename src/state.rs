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
        self.data
            .get(name)
            .ok_or_else(|| RossbyError::DataNotFound {
                message: format!("Variable not found: {}", name),
            })
    }

    /// Get coordinate values for a dimension
    pub fn get_coordinate(&self, name: &str) -> Option<&Vec<f64>> {
        self.metadata.coordinates.get(name)
    }

    /// Get coordinate values for a dimension with error handling
    pub fn get_coordinate_checked(&self, name: &str) -> Result<&Vec<f64>> {
        self.metadata
            .coordinates
            .get(name)
            .ok_or_else(|| RossbyError::DataNotFound {
                message: format!("Coordinate not found: {}", name),
            })
    }

    /// Get variable metadata
    pub fn get_variable_metadata(&self, name: &str) -> Option<&Variable> {
        self.metadata.variables.get(name)
    }

    /// Get variable metadata with error handling
    pub fn get_variable_metadata_checked(&self, name: &str) -> Result<&Variable> {
        self.metadata
            .variables
            .get(name)
            .ok_or_else(|| RossbyError::DataNotFound {
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
                    value,
                    dim_name,
                    coords[0],
                    coords[coords.len() - 1]
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

    /// Get the size of the time dimension
    pub fn time_dim_size(&self) -> usize {
        if let Some(dim) = self.metadata.dimensions.get("time") {
            dim.size
        } else {
            1 // Default to 1 if no time dimension
        }
    }

    /// Get the global lat/lon boundaries of the data
    pub fn get_lat_lon_bounds(&self) -> Result<(f32, f32, f32, f32)> {
        // Get lat and lon coordinate arrays
        let lon_coords = self.get_coordinate_checked("lon")?;
        let lat_coords = self.get_coordinate_checked("lat")?;

        if lon_coords.is_empty() || lat_coords.is_empty() {
            return Err(RossbyError::DataNotFound {
                message: "Latitude or longitude coordinates are empty".to_string(),
            });
        }

        // Find min/max values using iterators
        let min_lon = lon_coords
            .iter()
            .fold(f64::INFINITY, |min, &val| min.min(val)) as f32;
        let max_lon = lon_coords
            .iter()
            .fold(f64::NEG_INFINITY, |max, &val| max.max(val)) as f32;
        let min_lat = lat_coords
            .iter()
            .fold(f64::INFINITY, |min, &val| min.min(val)) as f32;
        let max_lat = lat_coords
            .iter()
            .fold(f64::NEG_INFINITY, |max, &val| max.max(val)) as f32;

        Ok((min_lon, min_lat, max_lon, max_lat))
    }

    /// Extract a 2D data slice for a variable at a given time and spatial bounds
    pub fn get_data_slice(
        &self,
        var_name: &str,
        time_index: usize,
        min_lon: f32,
        min_lat: f32,
        max_lon: f32,
        max_lat: f32,
    ) -> Result<Array<f32, ndarray::Ix2>> {
        // Get the variable data
        let var_data = self.get_variable_checked(var_name)?;

        // Get the variable dimensions
        let var_meta = self.get_variable_metadata_checked(var_name)?;
        let dimensions = &var_meta.dimensions;

        // Find the indices for lat, lon, and time in the dimensions
        let mut time_dim_idx = None;
        let mut lat_dim_idx = None;
        let mut lon_dim_idx = None;

        for (i, dim) in dimensions.iter().enumerate() {
            if dim == "time" {
                time_dim_idx = Some(i);
            } else if dim == "lat" {
                lat_dim_idx = Some(i);
            } else if dim == "lon" {
                lon_dim_idx = Some(i);
            }
        }

        // Ensure we have lat and lon dimensions
        let lat_dim_idx = lat_dim_idx.ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable {} does not have a lat dimension", var_name),
        })?;

        let lon_dim_idx = lon_dim_idx.ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable {} does not have a lon dimension", var_name),
        })?;

        // Get coordinate arrays
        let lon_coords = self.get_coordinate_checked("lon")?;
        let lat_coords = self.get_coordinate_checked("lat")?;

        // Find index ranges for the bounding box
        let min_lon_idx = lon_coords
            .iter()
            .position(|&lon| lon as f32 >= min_lon)
            .unwrap_or(0);

        let max_lon_idx = lon_coords
            .iter()
            .rposition(|&lon| lon as f32 <= max_lon)
            .unwrap_or(lon_coords.len() - 1);

        let min_lat_idx = lat_coords
            .iter()
            .position(|&lat| lat as f32 >= min_lat)
            .unwrap_or(0);

        let max_lat_idx = lat_coords
            .iter()
            .rposition(|&lat| lat as f32 <= max_lat)
            .unwrap_or(lat_coords.len() - 1);

        // Create a view into the data array based on the dimensions
        if let Some(time_dim_idx) = time_dim_idx {
            // Variable has a time dimension
            // Create the slice info directly with indexing operations
            if var_data.ndim() == 3 {
                // Most common case: [time, lat, lon]
                let slice = if time_dim_idx == 0 && lat_dim_idx == 1 && lon_dim_idx == 2 {
                    var_data.slice(ndarray::s![
                        time_index,
                        min_lat_idx..=max_lat_idx,
                        min_lon_idx..=max_lon_idx
                    ])
                } else if time_dim_idx == 0 && lat_dim_idx == 2 && lon_dim_idx == 1 {
                    var_data.slice(ndarray::s![
                        time_index,
                        min_lon_idx..=max_lon_idx,
                        min_lat_idx..=max_lat_idx
                    ])
                } else if time_dim_idx == 1 && lat_dim_idx == 0 && lon_dim_idx == 2 {
                    var_data.slice(ndarray::s![
                        min_lat_idx..=max_lat_idx,
                        time_index,
                        min_lon_idx..=max_lon_idx
                    ])
                } else if time_dim_idx == 1 && lat_dim_idx == 2 && lon_dim_idx == 0 {
                    var_data.slice(ndarray::s![
                        min_lon_idx..=max_lon_idx,
                        time_index,
                        min_lat_idx..=max_lat_idx
                    ])
                } else if time_dim_idx == 2 && lat_dim_idx == 0 && lon_dim_idx == 1 {
                    var_data.slice(ndarray::s![
                        min_lat_idx..=max_lat_idx,
                        min_lon_idx..=max_lon_idx,
                        time_index
                    ])
                } else {
                    var_data.slice(ndarray::s![
                        min_lon_idx..=max_lon_idx,
                        min_lat_idx..=max_lat_idx,
                        time_index
                    ])
                };

                // Since we extracted a 2D slice from a 3D array, we need to convert the dimensionality
                Ok(slice.to_owned().into_dimensionality::<ndarray::Ix2>()?)
            } else {
                // Handle other dimensionality cases
                Err(RossbyError::DataNotFound {
                    message: format!("Unsupported data dimensionality: {}", var_data.ndim()),
                })
            }
        } else {
            // Variable doesn't have a time dimension, assume it's already 2D
            if var_data.ndim() == 2 {
                // Assume [lat, lon] or [lon, lat]
                let slice = if lat_dim_idx == 0 && lon_dim_idx == 1 {
                    var_data.slice(ndarray::s![
                        min_lat_idx..=max_lat_idx,
                        min_lon_idx..=max_lon_idx
                    ])
                } else {
                    var_data.slice(ndarray::s![
                        min_lon_idx..=max_lon_idx,
                        min_lat_idx..=max_lat_idx
                    ])
                };

                Ok(slice.to_owned())
            } else {
                Err(RossbyError::DataNotFound {
                    message: format!(
                        "Expected a 2D array without time dimension, got {}D",
                        var_data.ndim()
                    ),
                })
            }
        }
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
