//! HTTP request handlers for the rossby API.
//!
//! This module contains all the endpoint handlers for the web server.

pub mod image;
pub mod metadata;
pub mod point;

pub use image::image_handler;
pub use metadata::metadata_handler;
pub use point::point_handler;
