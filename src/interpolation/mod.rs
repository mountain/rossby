//! Interpolation algorithms for spatial data.
//!
//! This module provides various interpolation methods for querying
//! values at arbitrary points within the data grid.

pub mod bicubic;
pub mod bilinear;
pub mod common;
pub mod nearest;

use crate::error::Result;

/// Trait for interpolation methods
pub trait Interpolator {
    /// Interpolate a value at the given fractional indices
    fn interpolate(&self, data: &[f32], shape: &[usize], indices: &[f64]) -> Result<f32>;

    /// Get the name of this interpolation method
    fn name(&self) -> &str;
}

/// Get an interpolator by name
pub fn get_interpolator(name: &str) -> Result<Box<dyn Interpolator>> {
    match name.to_lowercase().as_str() {
        "nearest" => Ok(Box::new(nearest::NearestInterpolator)),
        "bilinear" => Ok(Box::new(bilinear::BilinearInterpolator)),
        "bicubic" => Ok(Box::new(bicubic::BicubicInterpolator)),
        _ => Err(crate::error::RossbyError::InvalidParameter {
            param: "interpolation".to_string(),
            message: format!("Unknown interpolation method: {}", name),
        }),
    }
}
