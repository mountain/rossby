//! Bicubic interpolation.
//!
//! This method uses 16 surrounding grid points to produce smoother
//! interpolation results than bilinear. It's particularly effective for
//! image data, preserving more detail while reducing artifacts.
//!
//! For each dimension, bicubic interpolation uses 4 control points and
//! a cubic polynomial to generate a smooth curve that preserves continuity
//! of both the function and its derivatives.
//!
//! While most commonly used for 2D data (hence "bicubic"), this implementation
//! generalizes to N dimensions through recursive application.

use super::Interpolator;
use crate::error::Result;
use crate::interpolation::common;

/// Bicubic interpolator
pub struct BicubicInterpolator;

impl Interpolator for BicubicInterpolator {
    fn interpolate(&self, data: &[f32], shape: &[usize], indices: &[f64]) -> Result<f32> {
        // Validate dimensions
        if indices.len() != shape.len() {
            return Err(crate::error::RossbyError::Interpolation {
                message: format!(
                    "Dimension mismatch: indices has {} dimensions but shape has {} dimensions",
                    indices.len(),
                    shape.len()
                ),
            });
        }

        // For 0D (scalar) input
        if indices.is_empty() {
            if data.len() != 1 {
                return Err(crate::error::RossbyError::Interpolation {
                    message: "Expected scalar data (length 1) for 0D interpolation".to_string(),
                });
            }
            return Ok(data[0]);
        }

        // For each dimension, we need at least 4 points for cubic interpolation
        for (i, &size) in shape.iter().enumerate() {
            if size < 4 {
                // Fall back to bilinear for small dimensions
                // Note: For production code, we might want to implement a fallback to bilinear here
                return Err(crate::error::RossbyError::Interpolation {
                    message: format!(
                        "Dimension {} has size {}, but bicubic interpolation requires at least 4 points per dimension. Consider using bilinear interpolation instead.",
                        i, size
                    ),
                });
            }
        }

        // Recursive implementation of n-dimensional cubic interpolation
        interpolate_nd(data, shape, indices, 0)
    }

    fn name(&self) -> &str {
        "bicubic"
    }
}

/// Recursive implementation of n-dimensional cubic interpolation
fn interpolate_nd(data: &[f32], shape: &[usize], indices: &[f64], dim: usize) -> Result<f32> {
    // Base case: we've handled all dimensions
    if dim == indices.len() {
        // Calculate flat index based on the current indices
        let mut idx_array = Vec::with_capacity(indices.len());
        for &index in indices {
            idx_array.push(index.floor() as usize);
        }

        let flat_idx = common::flat_index(&idx_array, shape)?;
        if flat_idx >= data.len() {
            return Err(crate::error::RossbyError::Interpolation {
                message: format!(
                    "Index out of bounds: calculated index {} exceeds data length {}",
                    flat_idx,
                    data.len()
                ),
            });
        }

        return Ok(data[flat_idx]);
    }

    // Clamp the index for the current dimension
    let idx = common::clamp_index(indices[dim], shape[dim]);

    // Get the integer index and fractional component
    let i = idx.floor() as usize;
    let frac = idx - i as f64;

    // Get the 4 surrounding points in this dimension
    // We need points at i-1, i, i+1, i+2
    let mut positions = [0; 4];
    positions[0] = if i > 0 { i - 1 } else { 0 };
    positions[1] = i;
    positions[2] = (i + 1).min(shape[dim] - 1);
    positions[3] = (i + 2).min(shape[dim] - 1);

    // For each of the four points in this dimension,
    // recursively interpolate along the remaining dimensions
    let mut new_indices = indices.to_vec();
    let mut values = [0.0; 4];

    for j in 0..4 {
        new_indices[dim] = positions[j] as f64;
        values[j] = interpolate_nd(data, shape, &new_indices, dim + 1)?;
    }

    // Calculate cubic interpolation weights
    let weights = common::cubic_weights(frac);

    // Apply weights to the four values
    let mut result = 0.0;
    for j in 0..4 {
        result += values[j] as f64 * weights[j];
    }

    Ok(result as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bicubic_interpolation_1d() {
        // Need at least 4 points in each dimension for cubic interpolation
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let shape = vec![5];
        let interpolator = BicubicInterpolator;

        // Exact indices
        assert_eq!(
            interpolator.interpolate(&data, &shape, &[1.0]).unwrap(),
            2.0
        );
        assert_eq!(
            interpolator.interpolate(&data, &shape, &[2.0]).unwrap(),
            3.0
        );
        assert_eq!(
            interpolator.interpolate(&data, &shape, &[3.0]).unwrap(),
            4.0
        );

        // Fractional indices
        // Cubic interpolation should pass through the control points
        assert!((interpolator.interpolate(&data, &shape, &[1.5]).unwrap() - 2.5).abs() < 1e-5);

        // Test boundary conditions
        // At the edges, should handle gracefully
        assert_eq!(
            interpolator.interpolate(&data, &shape, &[0.0]).unwrap(),
            1.0
        );
        assert_eq!(
            interpolator.interpolate(&data, &shape, &[4.0]).unwrap(),
            5.0
        );
    }

    #[test]
    fn test_bicubic_interpolation_2d() {
        // Need a larger grid for bicubic (at least 4x4)
        let data = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let shape = vec![4, 4];
        let interpolator = BicubicInterpolator;

        // Exact points
        assert_eq!(
            interpolator
                .interpolate(&data, &shape, &[1.0, 1.0])
                .unwrap(),
            6.0
        );
        assert_eq!(
            interpolator
                .interpolate(&data, &shape, &[2.0, 2.0])
                .unwrap(),
            11.0
        );

        // Center of the grid
        let center_value = interpolator
            .interpolate(&data, &shape, &[1.5, 1.5])
            .unwrap();
        assert!((center_value - 8.5).abs() < 1e-5);

        // Smoothness test - points along a line should change smoothly
        let v1 = interpolator
            .interpolate(&data, &shape, &[1.5, 1.0])
            .unwrap();
        let v2 = interpolator
            .interpolate(&data, &shape, &[1.5, 1.25])
            .unwrap();
        let v3 = interpolator
            .interpolate(&data, &shape, &[1.5, 1.5])
            .unwrap();
        let v4 = interpolator
            .interpolate(&data, &shape, &[1.5, 1.75])
            .unwrap();
        let v5 = interpolator
            .interpolate(&data, &shape, &[1.5, 2.0])
            .unwrap();

        // Each should be increasing
        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v4 < v5);
    }

    #[test]
    fn test_bicubic_error_cases() {
        // Too small grid
        let data = vec![1.0, 2.0, 3.0];
        let shape = vec![3];
        let interpolator = BicubicInterpolator;

        // Should error because dimension size < 4
        let result = interpolator.interpolate(&data, &shape, &[1.0]);
        assert!(result.is_err());

        // Dimension mismatch
        let data = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let shape = vec![4, 4];

        let result = interpolator.interpolate(&data, &shape, &[1.0]);
        assert!(result.is_err());

        let result = interpolator.interpolate(&data, &shape, &[1.0, 1.0, 1.0]);
        assert!(result.is_err());
    }
}
