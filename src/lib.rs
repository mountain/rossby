//! # rossby
//!
//! A blazingly fast, in-memory, NetCDF-to-API server.
//!
//! This library provides the core functionality for loading NetCDF files into memory
//! and serving them via a high-performance HTTP API with support for interpolation
//! and visualization.

pub mod colormaps;
pub mod config;
pub mod data_loader;
pub mod error;
pub mod handlers;
pub mod interpolation;
pub mod state;

pub use config::Config;
pub use error::{Result, RossbyError};
pub use state::AppState;
