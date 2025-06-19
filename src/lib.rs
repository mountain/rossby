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
    create_http_trace_layer, generate_request_id, init_tracing, log_data_load_stats, log_error,
    log_operation_end, log_operation_start, log_request_error, log_timed_operation,
};
pub use state::{AppState, AttributeValue, Dimension, Metadata, Variable};
