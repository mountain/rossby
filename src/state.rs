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
    /// Reverse dimension aliases mapping (canonical name -> file-specific name)
    dimension_aliases_reverse: HashMap<String, String>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(
        config: Config,
        metadata: Metadata,
        data: HashMap<String, Array<f32, IxDyn>>,
    ) -> Self {
        // Build the reverse dimension aliases mapping
        let mut dimension_aliases_reverse = HashMap::new();
        for (canonical, file_specific) in &config.data.dimension_aliases {
            dimension_aliases_reverse.insert(canonical.clone(), file_specific.clone());
        }

        Self {
            config,
            metadata,
            data,
            dimension_aliases_reverse,
        }
    }

    /// Resolve a dimension name to its file-specific name
    ///
    /// This function handles three cases:
    /// 1. Direct file-specific dimension name (e.g., "lat")
    /// 2. Prefixed canonical name (e.g., "_latitude")
    /// 3. Dimension aliases from config (e.g., "latitude" -> "lat")
    ///
    /// Returns the file-specific dimension name or an error if not found
    pub fn resolve_dimension<'a>(&'a self, name: &'a str) -> Result<&'a str> {
        // Case 1: Check if the name is a direct file-specific dimension name
        if self.metadata.dimensions.contains_key(name) {
            return Ok(name);
        }

        // Case 2: Check if it's a prefixed canonical name (starting with "_")
        if let Some(canonical) = name.strip_prefix('_') {
            if let Some(file_specific) = self.dimension_aliases_reverse.get(canonical) {
                // Make sure the file-specific name actually exists
                if self.metadata.dimensions.contains_key(file_specific) {
                    return Ok(file_specific);
                }
            }
        }

        // Case 3: Check if it's an unprefixed canonical name from config aliases
        if let Some(file_specific) = self.dimension_aliases_reverse.get(name) {
            if self.metadata.dimensions.contains_key(file_specific) {
                return Ok(file_specific);
            }
        }

        // Couldn't resolve the dimension name
        Err(RossbyError::DimensionNotFound {
            name: name.to_string(),
            available: self.metadata.dimensions.keys().cloned().collect(),
            aliases: self.dimension_aliases_reverse.clone(),
        })
    }

    /// Get the canonical name for a dimension, if it has one
    pub fn get_canonical_dimension_name(&self, file_specific: &str) -> Option<&str> {
        for (canonical, fs) in &self.dimension_aliases_reverse {
            if fs == file_specific {
                return Some(canonical);
            }
        }
        None
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
        if let Ok(file_specific) = self.resolve_dimension(name) {
            self.metadata.coordinates.get(file_specific)
        } else {
            None
        }
    }

    /// Get coordinate values for a dimension with error handling
    pub fn get_coordinate_checked(&self, name: &str) -> Result<&Vec<f64>> {
        let file_specific = self.resolve_dimension(name)?;
        self.metadata
            .coordinates
            .get(file_specific)
            .ok_or_else(|| RossbyError::DataNotFound {
                message: format!("Coordinate not found: {}", file_specific),
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

    /// Check if a coordinate exists
    pub fn has_coordinate(&self, name: &str) -> bool {
        self.metadata.coordinates.contains_key(name)
    }

    /// Find the index of a coordinate value within its array
    /// Returns the nearest index if exact match is not found
    pub fn find_coordinate_index(&self, dim_name: &str, value: f64) -> Result<usize> {
        let _file_specific = self.resolve_dimension(dim_name)?;
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

    /// Find the index of a coordinate value within its array using exact match
    /// Returns an error if the value is not found
    pub fn find_coordinate_index_exact(&self, dim_name: &str, value: f64) -> Result<usize> {
        let file_specific = self.resolve_dimension(dim_name)?;
        let coords = self.get_coordinate_checked(file_specific)?;

        // Early return for empty coordinates (shouldn't happen in valid files)
        if coords.is_empty() {
            return Err(RossbyError::DataNotFound {
                message: format!("Coordinate {} is empty", dim_name),
            });
        }

        // Find the exact match
        for (i, &coord) in coords.iter().enumerate() {
            if (coord - value).abs() < f64::EPSILON {
                return Ok(i);
            }
        }

        // No exact match found
        Err(RossbyError::PhysicalValueNotFound {
            dimension: dim_name.to_string(),
            value,
            available: coords.clone(),
        })
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
        // Try standard names first, then try with aliases
        let lon_coords = self
            .get_coordinate_checked("lon")
            .or_else(|_| self.get_coordinate_checked("_longitude"))
            .or_else(|_| self.get_coordinate_checked("longitude"))?;

        let lat_coords = self
            .get_coordinate_checked("lat")
            .or_else(|_| self.get_coordinate_checked("_latitude"))
            .or_else(|_| self.get_coordinate_checked("latitude"))?;

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
    /// with support for additional dimensions
    pub fn get_data_slice_with_dims(
        &self,
        var_name: &str,
        min_lon: f32,
        min_lat: f32,
        max_lon: f32,
        max_lat: f32,
        dim_indices: &HashMap<String, usize>,
    ) -> Result<Array<f32, ndarray::Ix2>> {
        // Get the variable data
        let var_data = self.get_variable_checked(var_name)?;

        // Get the variable dimensions
        let var_meta = self.get_variable_metadata_checked(var_name)?;
        let dimensions = &var_meta.dimensions;

        // Find the indices for lat and lon in the dimensions
        let mut lat_dim_idx_opt = None;
        let mut lon_dim_idx_opt = None;

        for (i, dim) in dimensions.iter().enumerate() {
            if dim == "lat" || dim == "latitude" {
                lat_dim_idx_opt = Some(i);
            } else if dim == "lon" || dim == "longitude" {
                lon_dim_idx_opt = Some(i);
            }
        }

        // Ensure we have lat and lon dimensions
        let lat_dim_idx = lat_dim_idx_opt.ok_or_else(|| RossbyError::DataNotFound {
            message: format!(
                "Variable {} does not have a latitude dimension (looking for 'lat' or 'latitude')",
                var_name
            ),
        })?;

        let lon_dim_idx = lon_dim_idx_opt.ok_or_else(|| RossbyError::DataNotFound {
            message: format!("Variable {} does not have a longitude dimension (looking for 'lon' or 'longitude')", var_name),
        })?;

        // Get coordinate arrays - try both common naming conventions
        let lon_coords = if self.metadata.coordinates.contains_key("lon") {
            self.get_coordinate_checked("lon")?
        } else {
            self.get_coordinate_checked("longitude")?
        };

        let lat_coords = if self.metadata.coordinates.contains_key("lat") {
            self.get_coordinate_checked("lat")?
        } else {
            self.get_coordinate_checked("latitude")?
        };

        // Check for empty coordinate arrays
        if lon_coords.is_empty() || lat_coords.is_empty() {
            // Return an empty 2D array rather than failing
            return Ok(Array::from_elem((0, 0), 0.0));
        }

        // Find index ranges for the bounding box with safety checks
        // Handle the case of dateline crossing (min_lon > max_lon)
        let (min_lon_idx, max_lon_idx) = if min_lon <= max_lon {
            // Normal case - no dateline crossing
            let min_idx = lon_coords
                .iter()
                .position(|&lon| lon as f32 >= min_lon)
                .unwrap_or(0);

            let max_idx = lon_coords
                .iter()
                .rposition(|&lon| lon as f32 <= max_lon)
                .unwrap_or(lon_coords.len() - 1);

            (min_idx, max_idx)
        } else {
            // Dateline crossing case - treat as empty slice for now
            // The actual handling of dateline crossing happens in the image handler
            // through adjust_for_dateline_crossing function
            (0, 0)
        };

        let min_lat_idx = lat_coords
            .iter()
            .position(|&lat| lat as f32 >= min_lat)
            .unwrap_or(0);

        let max_lat_idx = lat_coords
            .iter()
            .rposition(|&lat| lat as f32 <= max_lat)
            .unwrap_or(lat_coords.len() - 1);

        // Special handling for dateline crossing: if min_lon > max_lon and we're returning an empty slice
        if min_lon > max_lon {
            // Return a minimal valid slice that the image handler can work with
            return Ok(Array::from_elem((max_lat_idx - min_lat_idx + 1, 1), 0.0));
        }

        // Create a mutable clone of the data array to work with
        let mut data_array = var_data.to_owned();

        // Track dimension indices as we go - they will change as we slice
        // Start with the definite indices we just extracted
        let mut current_lat_idx_opt = Some(lat_dim_idx);
        let mut current_lon_idx_opt = Some(lon_dim_idx);

        // Process non-lat/lon dimensions first
        // Sort dimensions by index in descending order so we can slice without affecting indices
        let mut non_lat_lon_dims: Vec<(usize, String)> = dimensions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != lat_dim_idx && *i != lon_dim_idx)
            .map(|(i, name)| (i, name.clone()))
            .collect();

        // Sort in reverse order (highest index first) so we can remove dimensions without affecting indices
        non_lat_lon_dims.sort_by(|a, b| b.0.cmp(&a.0));

        // Slice each non-lat/lon dimension, with automatic removal counting
        for (removed_count, (dim_idx, dim_name)) in non_lat_lon_dims.into_iter().enumerate() {
            // Get the index to slice at (default to 0 if not provided)
            let index = dim_indices.get(&dim_name).copied().unwrap_or(0);

            // Use index_axis to select just the slice at the specified index
            // Adjust the dimension index based on how many dimensions we've already removed
            // Make sure we don't underflow when calculating the adjusted dimension index
            let adjusted_dim_idx = if dim_idx >= removed_count {
                dim_idx - removed_count
            } else {
                // This should never happen with properly sorted indices,
                // but we're being defensive against underflow
                tracing::warn!(
                    original_dim_idx = dim_idx,
                    removed_count = removed_count,
                    "Dimension index underflow prevented"
                );
                0
            };
            data_array = data_array.index_axis_move(ndarray::Axis(adjusted_dim_idx), index);

            // Update lat/lon indices to account for the removed dimension
            current_lat_idx_opt = current_lat_idx_opt.map(|idx| {
                if idx > dim_idx {
                    // If lat dimension is after this one, decrement its index
                    // This is safe because we've already checked idx > dim_idx
                    idx - 1
                } else {
                    // Otherwise, keep the same index
                    idx
                }
            });

            current_lon_idx_opt = current_lon_idx_opt.map(|idx| {
                if idx > dim_idx {
                    // If lon dimension is after this one, decrement its index
                    // This is safe because we've already checked idx > dim_idx
                    idx - 1
                } else {
                    // Otherwise, keep the same index
                    idx
                }
            });
        }

        // After slicing all non-lat/lon dimensions, we should have just lat and lon left
        if data_array.ndim() != 2 {
            return Err(RossbyError::DataNotFound {
                message: format!(
                    "Expected a 2D array after slicing all non-lat/lon dimensions, got {}D",
                    data_array.ndim()
                ),
            });
        }

        // Now slice the lat/lon dimensions
        // We need to determine which dimension is which in our 2D array
        let lat_idx = current_lat_idx_opt.ok_or_else(|| RossbyError::DataNotFound {
            message: "Lost track of latitude dimension during slicing".to_string(),
        })?;

        let lon_idx = current_lon_idx_opt.ok_or_else(|| RossbyError::DataNotFound {
            message: "Lost track of longitude dimension during slicing".to_string(),
        })?;

        let lat_is_first = lat_idx < lon_idx;
        let lon_is_first = lon_idx < lat_idx;

        // Create a slice of the lat/lon region
        let result = if lat_is_first {
            // Latitude is the first dimension (rows), longitude is the second (columns)
            data_array.slice(ndarray::s![
                min_lat_idx..=max_lat_idx,
                min_lon_idx..=max_lon_idx
            ])
        } else if lon_is_first {
            // Longitude is the first dimension (rows), latitude is the second (columns)
            data_array.slice(ndarray::s![
                min_lon_idx..=max_lon_idx,
                min_lat_idx..=max_lat_idx
            ])
        } else {
            // Should not happen given our checks above
            return Err(RossbyError::DataNotFound {
                message: "Could not determine dimension order after slicing".to_string(),
            });
        };
        // Convert the result to a 2D array and return it
        Ok(result.to_owned().into_dimensionality::<ndarray::Ix2>()?)
    }

    /// Extract a 2D data slice for a variable at a given time and spatial bounds
    /// This is the original implementation that calls the new get_data_slice_with_dims
    /// with only the time dimension specified
    pub fn get_data_slice(
        &self,
        var_name: &str,
        time_index: usize,
        min_lon: f32,
        min_lat: f32,
        max_lon: f32,
        max_lat: f32,
    ) -> Result<Array<f32, ndarray::Ix2>> {
        // Create a HashMap with just the time dimension index
        let mut dim_indices = HashMap::new();

        // Check if this variable has a time dimension
        let var_meta = self.get_variable_metadata_checked(var_name)?;
        if var_meta.dimensions.contains(&"time".to_string()) {
            dim_indices.insert("time".to_string(), time_index);
        }

        // Call the new method with the prepared dimension indices
        self.get_data_slice_with_dims(var_name, min_lon, min_lat, max_lon, max_lat, &dim_indices)
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
