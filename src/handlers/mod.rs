//! HTTP request handlers for the rossby API.
//!
//! This module contains all the endpoint handlers for the web server.

pub mod metadata;
pub mod point;
pub mod image;

pub use metadata::metadata_handler;
pub use point::point_handler;
pub use image::image_handler;
