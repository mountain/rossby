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

    /// Invalid parameter errors
    #[error("Invalid parameter: {param} - {message}")]
    InvalidParameter { param: String, message: String },

    /// Data not found errors
    #[error("Data not found: {message}")]
    DataNotFound { message: String },

    /// Variable not found errors
    #[error("Variable not found: {name}")]
    VariableNotFound { name: String },

    /// Invalid variables errors
    #[error("Invalid variable(s): [{}]", names.join(", "))]
    InvalidVariables { names: Vec<String> },

    /// Variable not suitable for image rendering
    #[error("Variable {name} is not suitable for image rendering. It must be a 2D grid with latitude and longitude dimensions.")]
    VariableNotSuitableForImage { name: String },

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

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Server errors
    #[error("Server error: {message}")]
    Server { message: String },
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
