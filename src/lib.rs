//! # rossby
//!
//! A blazingly fast, in-memory, NetCDF-to-API server.
//!
//! This library provides the core functionality for loading NetCDF files into memory
//! and serving them via a high-performance HTTP API with support for interpolation
//! and visualization.
//!
//! ## Key Features
//!
//! - **Zero-configuration NetCDF serving**: Load any NetCDF file and instantly serve it via HTTP API
//! - **Blazing-fast performance**: In-memory data storage with microsecond query latency
//! - **Rich interpolation support**: Multiple interpolation methods for flexible data access
//! - **Beautiful visualizations**: Matplotlib-inspired colormaps for image generation
//!
//! ## Architecture
//!
//! - **Data Layer**: Loads NetCDF files into memory for fast access
//! - **API Layer**: Exposes data through a RESTful HTTP API
//! - **Processing**: Supports multiple interpolation methods and colormap rendering

pub mod colormaps;
pub mod config;
pub mod data_loader;
pub mod error;
pub mod handlers;
pub mod interpolation;
pub mod logging;
pub mod state;

pub use config::Config;
pub use error::{Result, RossbyError};
pub use logging::{
    generate_request_id, log_data_loaded, log_request_error, log_request_success,
    log_timed_operation, setup_logging, start_timed_operation, TimedOperationGuard,
};
pub use state::{AppState, AttributeValue, Dimension, Metadata, Variable};
