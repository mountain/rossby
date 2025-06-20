//! Error types for the rossby application.
//!
//! This module defines a comprehensive error enum that covers all possible
//! error conditions in the application, following the guidelines in AGENT.md.

use thiserror::Error;

/// The main error type for rossby operations.
#[derive(Error, Debug)]
pub enum RossbyError {
    /// NetCDF file operation errors
    #[error("NetCDF error: {message}")]
    NetCdf { message: String },

    /// Conversion errors
    #[error("Conversion error: {message}")]
    Conversion { message: String },

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Invalid coordinate errors
    #[error("Invalid coordinates: {message}")]
    InvalidCoordinates { message: String },

    /// Physical value not found in coordinate array
    #[error("Physical value not found: {dimension}={value}. Available values: {available:?}")]
    PhysicalValueNotFound {
        dimension: String,
        value: f64,
        available: Vec<f64>,
    },

    /// Invalid parameter errors
    #[error("Invalid parameter: {param} - {message}")]
    InvalidParameter { param: String, message: String },

    /// Data not found errors
    #[error("Data not found: {message}")]
    DataNotFound { message: String },

    /// Variable not found errors
    #[error("Variable not found: {name}")]
    VariableNotFound { name: String },

    /// Index out of bounds errors
    #[error("Index out of bounds: {param}={value}, max allowed is {max}")]
    IndexOutOfBounds {
        param: String,
        value: String,
        max: usize,
    },

    /// Interpolation errors
    #[error("Interpolation error: {message}")]
    Interpolation { message: String },

    /// Image generation errors
    #[error("Image generation error: {message}")]
    ImageGeneration { message: String },

    /// Invalid variable(s) errors
    #[error("Invalid variable(s): [{names:?}]")]
    InvalidVariables { names: Vec<String> },

    /// Variable not suitable for image rendering
    #[error("Variable {name} is not suitable for image rendering. It must be a 2D grid with latitude and longitude dimensions.")]
    VariableNotSuitableForImage { name: String },

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Dimension not found errors
    #[error("Dimension not found: {name}. Available dimensions: {available:?}. If using a canonical name, try using it with an underscore prefix (e.g., '_latitude') or set up dimension_aliases in config.")]
    DimensionNotFound {
        name: String,
        available: Vec<String>,
        aliases: std::collections::HashMap<String, String>,
    },

    /// Server errors
    #[error("Server error: {message}")]
    Server { message: String },

    /// Payload too large error
    #[error("Payload too large: {message}. Requested points: {requested}, maximum allowed: {max_allowed}")]
    PayloadTooLarge {
        message: String,
        requested: usize,
        max_allowed: usize,
    },
}

/// Convenience type alias for Results with RossbyError
pub type Result<T> = std::result::Result<T, RossbyError>;

// Implement From for common error types
impl From<String> for RossbyError {
    fn from(message: String) -> Self {
        RossbyError::Server { message }
    }
}

impl From<&str> for RossbyError {
    fn from(message: &str) -> Self {
        RossbyError::Server {
            message: message.to_string(),
        }
    }
}

impl From<std::num::ParseIntError> for RossbyError {
    fn from(err: std::num::ParseIntError) -> Self {
        RossbyError::Conversion {
            message: err.to_string(),
        }
    }
}

impl From<std::num::ParseFloatError> for RossbyError {
    fn from(err: std::num::ParseFloatError) -> Self {
        RossbyError::Conversion {
            message: err.to_string(),
        }
    }
}

impl From<ndarray::ShapeError> for RossbyError {
    fn from(err: ndarray::ShapeError) -> Self {
        RossbyError::Conversion {
            message: format!("Array shape error: {}", err),
        }
    }
}

impl From<netcdf::Error> for RossbyError {
    fn from(err: netcdf::Error) -> Self {
        RossbyError::NetCdf {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        // Test string to error conversion
        let err: RossbyError = "test error".into();
        match err {
            RossbyError::Server { message } => assert_eq!(message, "test error"),
            _ => panic!("Wrong error variant"),
        }

        // Test parse int error conversion
        let parse_err = "abc".parse::<i32>().unwrap_err();
        let err: RossbyError = parse_err.into();
        match err {
            RossbyError::Conversion { message } => assert!(message.contains("invalid digit")),
            _ => panic!("Wrong error variant"),
        }
    }
}
