//! Common utilities for interpolation algorithms.
//!
//! This module provides shared functionality used by various interpolation methods.

use crate::error::Result;

/// Map a coordinate value to a fractional grid index
pub fn coord_to_index(_coord: f64, _coord_values: &[f64]) -> Result<f64> {
    // TODO: Implement coordinate to index mapping
    // This is a placeholder that will be implemented in Phase 5
    Ok(0.0)
}

/// Clamp an index to valid bounds
pub fn clamp_index(index: f64, size: usize) -> f64 {
    index.max(0.0).min((size - 1) as f64)
}

/// Get the weight for linear interpolation
pub fn linear_weight(fraction: f64) -> (f64, f64) {
    (1.0 - fraction, fraction)
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
}
