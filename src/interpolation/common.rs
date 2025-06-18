//! Common utilities for interpolation algorithms.
//!
//! This module provides shared functionality used by various interpolation methods.

use crate::error::Result;

/// Map a coordinate value to a fractional grid index
///
/// This function takes a coordinate value (e.g., a longitude or latitude)
/// and maps it to a fractional index in the coordinate array.
///
/// # Arguments
///
/// * `coord` - The coordinate value to map
/// * `coord_values` - The array of coordinate values (must be sorted)
///
/// # Returns
///
/// A fractional index where the integer part is the lower bound index
/// and the fractional part represents the position between the lower and upper indices.
pub fn coord_to_index(coord: f64, coord_values: &[f64]) -> Result<f64> {
    if coord_values.is_empty() {
        return Err(crate::error::RossbyError::Interpolation {
            message: "Empty coordinate values array".to_string(),
        });
    }

    let n = coord_values.len();

    // Handle edge cases where coordinate is outside the range
    if coord <= coord_values[0] {
        return Ok(0.0);
    }
    if coord >= coord_values[n - 1] {
        return Ok((n - 1) as f64);
    }

    // Binary search to find the index
    let mut low = 0;
    let mut high = n - 1;

    while high - low > 1 {
        let mid = (low + high) / 2;
        if coord_values[mid] <= coord {
            low = mid;
        } else {
            high = mid;
        }
    }

    // Linear interpolation between the two closest points
    let fraction = (coord - coord_values[low]) / (coord_values[high] - coord_values[low]);
    Ok(low as f64 + fraction)
}

/// Clamp an index to valid bounds
pub fn clamp_index(index: f64, size: usize) -> f64 {
    index.max(0.0).min((size - 1) as f64)
}

/// Get the weight for linear interpolation
pub fn linear_weight(fraction: f64) -> (f64, f64) {
    (1.0 - fraction, fraction)
}

/// Calculate index from multi-dimensional indices
///
/// Converts n-dimensional indices into a flat array index using row-major order.
///
/// # Arguments
///
/// * `indices` - Array of indices for each dimension
/// * `shape` - The shape of the n-dimensional array
///
/// # Returns
///
/// The corresponding 1D array index
pub fn flat_index(indices: &[usize], shape: &[usize]) -> Result<usize> {
    if indices.len() != shape.len() {
        return Err(crate::error::RossbyError::Interpolation {
            message: format!(
                "Dimension mismatch: indices has {} dimensions but shape has {} dimensions",
                indices.len(),
                shape.len()
            ),
        });
    }

    let mut index = 0;
    let mut stride = 1;

    // Calculate index in row-major order (last dimension varies fastest)
    for i in (0..shape.len()).rev() {
        if indices[i] >= shape[i] {
            return Err(crate::error::RossbyError::Interpolation {
                message: format!(
                    "Index out of bounds: index {} is {} but dimension size is {}",
                    i, indices[i], shape[i]
                ),
            });
        }
        index += indices[i] * stride;
        stride *= shape[i];
    }

    Ok(index)
}

/// Get the weights for cubic interpolation
pub fn cubic_weights(fraction: f64) -> [f64; 4] {
    let x = fraction;
    let x2 = x * x;
    let x3 = x2 * x;

    // Cubic interpolation weights (Catmull-Rom spline)
    let w0 = -0.5 * x + 1.0 * x2 - 0.5 * x3;
    let w1 = 1.0 - 2.5 * x2 + 1.5 * x3;
    let w2 = 0.5 * x + 2.0 * x2 - 1.5 * x3;
    let w3 = -0.5 * x2 + 0.5 * x3;

    [w0, w1, w2, w3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_index() {
        assert_eq!(clamp_index(-1.0, 10), 0.0);
        assert_eq!(clamp_index(5.5, 10), 5.5);
        assert_eq!(clamp_index(15.0, 10), 9.0);
    }

    #[test]
    fn test_linear_weight() {
        let (w0, w1) = linear_weight(0.3);
        assert!((w0 - 0.7).abs() < 1e-10);
        assert!((w1 - 0.3).abs() < 1e-10);
        assert!((w0 + w1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_index_1d() {
        let shape = vec![5];
        assert_eq!(flat_index(&[0], &shape).unwrap(), 0);
        assert_eq!(flat_index(&[3], &shape).unwrap(), 3);
        assert_eq!(flat_index(&[4], &shape).unwrap(), 4);
    }

    #[test]
    fn test_flat_index_2d() {
        let shape = vec![3, 4]; // 3 rows, 4 columns
        assert_eq!(flat_index(&[0, 0], &shape).unwrap(), 0);
        assert_eq!(flat_index(&[0, 1], &shape).unwrap(), 1);
        assert_eq!(flat_index(&[1, 0], &shape).unwrap(), 4);
        assert_eq!(flat_index(&[2, 3], &shape).unwrap(), 11);
    }

    #[test]
    fn test_flat_index_3d() {
        let shape = vec![2, 3, 4]; // 2 x 3 x 4 array
        assert_eq!(flat_index(&[0, 0, 0], &shape).unwrap(), 0);
        assert_eq!(flat_index(&[0, 0, 1], &shape).unwrap(), 1);
        assert_eq!(flat_index(&[0, 1, 0], &shape).unwrap(), 4);
        assert_eq!(flat_index(&[1, 0, 0], &shape).unwrap(), 12);
        assert_eq!(flat_index(&[1, 2, 3], &shape).unwrap(), 23);
    }

    #[test]
    fn test_flat_index_out_of_bounds() {
        let shape = vec![2, 3];
        let result = flat_index(&[2, 0], &shape);
        assert!(result.is_err());

        let result = flat_index(&[0, 3], &shape);
        assert!(result.is_err());
    }

    #[test]
    fn test_flat_index_dimension_mismatch() {
        let shape = vec![2, 3];
        let result = flat_index(&[1], &shape);
        assert!(result.is_err());

        let result = flat_index(&[1, 1, 1], &shape);
        assert!(result.is_err());
    }

    #[test]
    fn test_cubic_weights() {
        // Test at exactly 0.5
        let weights = cubic_weights(0.5);
        assert!((weights[0] - (-0.0625)).abs() < 1e-10);
        assert!((weights[1] - 0.5625).abs() < 1e-10);
        assert!((weights[2] - 0.5625).abs() < 1e-10);
        assert!((weights[3] - (-0.0625)).abs() < 1e-10);

        // Sum of weights should be 1.0
        assert!((weights.iter().sum::<f64>() - 1.0).abs() < 1e-10);

        // Test at 0.0 - should heavily weight the second point (index 1)
        let weights = cubic_weights(0.0);
        assert!((weights[1] - 1.0).abs() < 1e-10);

        // Test at 1.0 - should heavily weight the third point (index 2)
        let weights = cubic_weights(1.0);
        assert!((weights[2] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_coord_to_index_empty_array() {
        let result = coord_to_index(5.0, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_coord_to_index_below_range() {
        let coords = vec![10.0, 20.0, 30.0, 40.0];
        let result = coord_to_index(5.0, &coords).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_coord_to_index_above_range() {
        let coords = vec![10.0, 20.0, 30.0, 40.0];
        let result = coord_to_index(45.0, &coords).unwrap();
        assert_eq!(result, 3.0);
    }

    #[test]
    fn test_coord_to_index_exact_match() {
        let coords = vec![10.0, 20.0, 30.0, 40.0];
        let result = coord_to_index(20.0, &coords).unwrap();
        assert_eq!(result, 1.0);
    }

    #[test]
    fn test_coord_to_index_interpolation() {
        let coords = vec![10.0, 20.0, 30.0, 40.0];
        let result = coord_to_index(25.0, &coords).unwrap();
        assert_eq!(result, 1.5);
    }

    #[test]
    fn test_coord_to_index_non_uniform() {
        // Non-uniform grid spacing
        let coords = vec![10.0, 15.0, 30.0, 50.0];
        let result = coord_to_index(20.0, &coords).unwrap();
        assert!((result - 1.33333).abs() < 0.0001);
    }
}
