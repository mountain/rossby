//! Assertion utilities for testing.
//!
//! This module provides helper functions for making assertions in tests,
//! particularly for floating-point comparisons.

/// Default epsilon for floating-point comparisons
pub const DEFAULT_EPSILON: f32 = 1e-6;

/// Assert that two floating-point values are approximately equal.
///
/// # Arguments
///
/// * `actual` - The actual value
/// * `expected` - The expected value
/// * `epsilon` - The maximum allowed difference (default: 1e-6)
///
/// # Panics
///
/// Panics if the absolute difference between `actual` and `expected` is greater than `epsilon`.
pub fn assert_approx_eq(actual: f32, expected: f32, epsilon: Option<f32>) {
    let epsilon = epsilon.unwrap_or(DEFAULT_EPSILON);
    let diff = (actual - expected).abs();

    assert!(
        diff <= epsilon,
        "Values not approximately equal: actual = {}, expected = {}, diff = {}, epsilon = {}",
        actual,
        expected,
        diff,
        epsilon
    );
}

/// Assert that two arrays of floating-point values are approximately element-wise equal.
///
/// # Arguments
///
/// * `actual` - The actual array
/// * `expected` - The expected array
/// * `epsilon` - The maximum allowed difference for each element (default: 1e-6)
///
/// # Panics
///
/// Panics if the arrays have different lengths or if any element-wise comparison fails.
pub fn assert_array_approx_eq(actual: &[f32], expected: &[f32], epsilon: Option<f32>) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "Arrays have different lengths: actual = {}, expected = {}",
        actual.len(),
        expected.len()
    );

    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        let eps = epsilon.unwrap_or(DEFAULT_EPSILON);

        assert!(
            diff <= eps,
            "Arrays differ at index {}: actual = {}, expected = {}, diff = {}, epsilon = {}",
            i,
            a,
            e,
            diff,
            eps
        );
    }
}

/// Assert that a result is within expected bounds.
///
/// # Arguments
///
/// * `actual` - The actual value
/// * `min` - The minimum expected value (inclusive)
/// * `max` - The maximum expected value (inclusive)
///
/// # Panics
///
/// Panics if `actual` is less than `min` or greater than `max`.
pub fn assert_in_range(actual: f32, min: f32, max: f32) {
    assert!(
        actual >= min && actual <= max,
        "Value not in range: actual = {}, min = {}, max = {}",
        actual,
        min,
        max
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_approx_eq() {
        // These should pass
        assert_approx_eq(1.0, 1.0, None);
        assert_approx_eq(1.0, 1.0000001, None);
        assert_approx_eq(1.0, 1.001, Some(0.01));

        // This would fail: assert_approx_eq(1.0, 1.1, None);
    }

    #[test]
    fn test_assert_array_approx_eq() {
        // These should pass
        assert_array_approx_eq(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0], None);
        assert_array_approx_eq(&[1.0, 2.0, 3.0], &[1.0000001, 2.0000001, 3.0000001], None);
        assert_array_approx_eq(&[1.0, 2.0, 3.0], &[1.001, 2.001, 3.001], Some(0.01));

        // These would fail:
        // assert_array_approx_eq(&[1.0, 2.0, 3.0], &[1.0, 2.0], None);
        // assert_array_approx_eq(&[1.0, 2.0, 3.0], &[1.0, 2.0, 4.0], None);
    }

    #[test]
    fn test_assert_in_range() {
        // These should pass
        assert_in_range(5.0, 0.0, 10.0);
        assert_in_range(0.0, 0.0, 10.0);
        assert_in_range(10.0, 0.0, 10.0);

        // These would fail:
        // assert_in_range(-1.0, 0.0, 10.0);
        // assert_in_range(11.0, 0.0, 10.0);
    }
}
