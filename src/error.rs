//! Error types for the rossby application.
//!
//! This module defines a comprehensive error enum that covers all possible
//! error conditions in the application, following the guidelines in AGENT.md.

use thiserror::Error;

/// The main error type for rossby operations.
#[derive(Error, Debug)]
pub enum RossbyError {
    /// NetCDF file operation errors
    #[error("NetCDF error: {0}")]
    NetCdf(#[from] netcdf::Error),

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
