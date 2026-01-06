//! Numeric range validation functions
//!
//! This module provides functions to validate numeric values according to
//! [`ValidationRule::Range`] constraints.
//!
//! # Functions
//!
//! - [`validate_range`] - Check if a value is within min/max bounds
//!
//! # Example
//!
//! ```
//! use dynamic_cli::validator::range_validator::validate_range;
//!
//! // Validate value is between 0 and 100
//! validate_range(50.0, "percentage", Some(0.0), Some(100.0))?;
//!
//! // Validate value is at least 0
//! validate_range(5.0, "count", Some(0.0), None)?;
//!
//! // Validate value is at most 1.0
//! validate_range(0.5, "probability", None, Some(1.0))?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::error::{Result, ValidationError};

/// Validate that a numeric value is within specified bounds
///
/// This function checks if a value satisfies the range constraints:
/// - If `min` is specified: value >= min
/// - If `max` is specified: value <= max
/// - If both are specified: min <= value <= max
///
/// # Arguments
///
/// * `value` - The numeric value to validate
/// * `arg_name` - Name of the argument (for error messages)
/// * `min` - Optional minimum value (inclusive)
/// * `max` - Optional maximum value (inclusive)
///
/// # Returns
///
/// - `Ok(())` if the value is within the specified range
/// - `Err(ValidationError::OutOfRange)` if the value is outside the range
///
/// # Range Types
///
/// The function supports several range configurations:
///
/// - **Both bounds**: `validate_range(x, "arg", Some(0.0), Some(100.0))` → 0 ≤ x ≤ 100
/// - **Lower bound only**: `validate_range(x, "arg", Some(0.0), None)` → x ≥ 0
/// - **Upper bound only**: `validate_range(x, "arg", None, Some(100.0))` → x ≤ 100
/// - **No bounds**: `validate_range(x, "arg", None, None)` → always valid
///
/// # Example
///
/// ```
/// use dynamic_cli::validator::range_validator::validate_range;
///
/// // Validate percentage (0-100)
/// assert!(validate_range(50.0, "percentage", Some(0.0), Some(100.0)).is_ok());
/// assert!(validate_range(-10.0, "percentage", Some(0.0), Some(100.0)).is_err());
/// assert!(validate_range(150.0, "percentage", Some(0.0), Some(100.0)).is_err());
///
/// // Validate non-negative count
/// assert!(validate_range(5.0, "count", Some(0.0), None).is_ok());
/// assert!(validate_range(-1.0, "count", Some(0.0), None).is_err());
///
/// // Validate probability (0-1)
/// assert!(validate_range(0.5, "prob", Some(0.0), Some(1.0)).is_ok());
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
///
/// # Error Messages
///
/// If the value is out of range, the error includes:
/// - The argument name
/// - The actual value
/// - The expected range (min and max)
///
/// Example: `percentage must be between 0 and 100, got -10`
///
/// # Special Values
///
/// The function handles special floating-point values:
/// - **Infinity**: Can be compared normally
/// - **NaN**: Always fails validation (NaN comparisons always return false)
///
/// # Edge Cases
///
/// - If both `min` and `max` are `None`, validation always succeeds
/// - Boundary values are **inclusive**: `validate_range(0.0, "x", Some(0.0), Some(1.0))` is valid
/// - If `min > max`, this is a configuration error (should be caught by config validation)
pub fn validate_range(
    value: f64,
    arg_name: &str,
    min: Option<f64>,
    max: Option<f64>,
) -> Result<()> {
    // Handle special case: NaN is never valid
    // (NaN comparisons always return false, so it would fail anyway,
    // but we make it explicit for clarity)
    if value.is_nan() {
        return Err(ValidationError::OutOfRange {
            arg_name: arg_name.to_string(),
            value,
            min: min.unwrap_or(f64::NEG_INFINITY),
            max: max.unwrap_or(f64::INFINITY),
        }
        .into());
    }

    // Check minimum bound (inclusive)
    if let Some(min_val) = min {
        if value < min_val {
            return Err(ValidationError::OutOfRange {
                arg_name: arg_name.to_string(),
                value,
                min: min_val,
                max: max.unwrap_or(f64::INFINITY),
            }
            .into());
        }
    }

    // Check maximum bound (inclusive)
    if let Some(max_val) = max {
        if value > max_val {
            return Err(ValidationError::OutOfRange {
                arg_name: arg_name.to_string(),
                value,
                min: min.unwrap_or(f64::NEG_INFINITY),
                max: max_val,
            }
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Tests for both bounds (min and max)
    // ========================================================================

    #[test]
    fn test_validate_range_within_bounds() {
        // Value in the middle of range
        assert!(validate_range(50.0, "percentage", Some(0.0), Some(100.0)).is_ok());

        // Value at lower boundary (inclusive)
        assert!(validate_range(0.0, "percentage", Some(0.0), Some(100.0)).is_ok());

        // Value at upper boundary (inclusive)
        assert!(validate_range(100.0, "percentage", Some(0.0), Some(100.0)).is_ok());
    }

    #[test]
    fn test_validate_range_below_minimum() {
        let result = validate_range(-10.0, "percentage", Some(0.0), Some(100.0));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::OutOfRange {
                arg_name,
                value,
                min,
                max,
            }) => {
                assert_eq!(arg_name, "percentage");
                assert_eq!(value, -10.0);
                assert_eq!(min, 0.0);
                assert_eq!(max, 100.0);
            }
            other => panic!("Expected OutOfRange error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_range_above_maximum() {
        let result = validate_range(150.0, "percentage", Some(0.0), Some(100.0));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::OutOfRange {
                arg_name,
                value,
                min,
                max,
            }) => {
                assert_eq!(arg_name, "percentage");
                assert_eq!(value, 150.0);
                assert_eq!(min, 0.0);
                assert_eq!(max, 100.0);
            }
            other => panic!("Expected OutOfRange error, got {:?}", other),
        }
    }

    // ========================================================================
    // Tests for minimum bound only
    // ========================================================================

    #[test]
    fn test_validate_range_min_only_valid() {
        // Value above minimum
        assert!(validate_range(5.0, "count", Some(0.0), None).is_ok());

        // Value at minimum (boundary)
        assert!(validate_range(0.0, "count", Some(0.0), None).is_ok());

        // Large positive value (no upper bound)
        assert!(validate_range(1_000_000.0, "count", Some(0.0), None).is_ok());
    }

    #[test]
    fn test_validate_range_min_only_invalid() {
        let result = validate_range(-5.0, "count", Some(0.0), None);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::OutOfRange {
                arg_name,
                value,
                min,
                max,
            }) => {
                assert_eq!(arg_name, "count");
                assert_eq!(value, -5.0);
                assert_eq!(min, 0.0);
                assert!(max.is_infinite() && max.is_sign_positive()); // f64::INFINITY
            }
            other => panic!("Expected OutOfRange error, got {:?}", other),
        }
    }

    // ========================================================================
    // Tests for maximum bound only
    // ========================================================================

    #[test]
    fn test_validate_range_max_only_valid() {
        // Value below maximum
        assert!(validate_range(0.5, "probability", None, Some(1.0)).is_ok());

        // Value at maximum (boundary)
        assert!(validate_range(1.0, "probability", None, Some(1.0)).is_ok());

        // Large negative value (no lower bound)
        assert!(validate_range(-1_000_000.0, "temperature", None, Some(100.0)).is_ok());
    }

    #[test]
    fn test_validate_range_max_only_invalid() {
        let result = validate_range(1.5, "probability", None, Some(1.0));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::OutOfRange {
                arg_name,
                value,
                min,
                max,
            }) => {
                assert_eq!(arg_name, "probability");
                assert_eq!(value, 1.5);
                assert!(min.is_infinite() && min.is_sign_negative()); // f64::NEG_INFINITY
                assert_eq!(max, 1.0);
            }
            other => panic!("Expected OutOfRange error, got {:?}", other),
        }
    }

    // ========================================================================
    // Tests for no bounds
    // ========================================================================

    #[test]
    fn test_validate_range_no_bounds() {
        // Any value should be valid with no bounds
        assert!(validate_range(0.0, "value", None, None).is_ok());
        assert!(validate_range(-1000.0, "value", None, None).is_ok());
        assert!(validate_range(1000.0, "value", None, None).is_ok());
        assert!(validate_range(f64::INFINITY, "value", None, None).is_ok());
        assert!(validate_range(f64::NEG_INFINITY, "value", None, None).is_ok());
    }

    // ========================================================================
    // Tests for special floating-point values
    // ========================================================================

    #[test]
    fn test_validate_range_nan() {
        // NaN should always fail validation
        let result = validate_range(f64::NAN, "value", Some(0.0), Some(100.0));
        assert!(result.is_err());

        // Even with no bounds
        let result = validate_range(f64::NAN, "value", None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_range_infinity_positive() {
        // Positive infinity should fail if max is finite
        assert!(validate_range(f64::INFINITY, "value", Some(0.0), Some(100.0)).is_err());

        // But succeed if no max
        assert!(validate_range(f64::INFINITY, "value", Some(0.0), None).is_ok());

        // And with no bounds
        assert!(validate_range(f64::INFINITY, "value", None, None).is_ok());
    }

    #[test]
    fn test_validate_range_infinity_negative() {
        // Negative infinity should fail if min is finite
        assert!(validate_range(f64::NEG_INFINITY, "value", Some(0.0), Some(100.0)).is_err());

        // But succeed if no min
        assert!(validate_range(f64::NEG_INFINITY, "value", None, Some(100.0)).is_ok());

        // And with no bounds
        assert!(validate_range(f64::NEG_INFINITY, "value", None, None).is_ok());
    }

    // ========================================================================
    // Tests for boundary conditions
    // ========================================================================

    #[test]
    fn test_validate_range_exact_boundaries() {
        // Test that boundaries are inclusive

        // Lower boundary
        assert!(validate_range(0.0, "x", Some(0.0), Some(1.0)).is_ok());

        // Upper boundary
        assert!(validate_range(1.0, "x", Some(0.0), Some(1.0)).is_ok());

        // Both boundaries at once
        assert!(validate_range(0.0, "x", Some(0.0), Some(0.0)).is_ok()); // min == max == value
    }

    #[test]
    fn test_validate_range_tiny_differences() {
        // Test with very small differences (floating-point precision)

        // Just inside range
        assert!(validate_range(0.0000001, "x", Some(0.0), Some(1.0)).is_ok());
        assert!(validate_range(0.9999999, "x", Some(0.0), Some(1.0)).is_ok());

        // Just outside range
        assert!(validate_range(-0.0000001, "x", Some(0.0), Some(1.0)).is_err());
        assert!(validate_range(1.0000001, "x", Some(0.0), Some(1.0)).is_err());
    }

    // ========================================================================
    // Tests for negative ranges
    // ========================================================================

    #[test]
    fn test_validate_range_negative_values() {
        // Range entirely in negative numbers
        assert!(validate_range(-50.0, "temperature", Some(-100.0), Some(0.0)).is_ok());
        assert!(validate_range(-100.0, "temperature", Some(-100.0), Some(0.0)).is_ok());
        assert!(validate_range(0.0, "temperature", Some(-100.0), Some(0.0)).is_ok());

        // Out of negative range
        assert!(validate_range(-150.0, "temperature", Some(-100.0), Some(0.0)).is_err());
        assert!(validate_range(10.0, "temperature", Some(-100.0), Some(0.0)).is_err());
    }

    // ========================================================================
    // Tests for very large and very small values
    // ========================================================================

    #[test]
    fn test_validate_range_large_values() {
        let very_large = 1e308; // Close to f64::MAX

        // Within large range
        assert!(validate_range(very_large, "value", Some(0.0), None).is_ok());

        // Outside large range
        assert!(validate_range(very_large, "value", Some(0.0), Some(1e307)).is_err());
    }

    #[test]
    fn test_validate_range_small_values() {
        let very_small = 1e-308; // Close to f64::MIN_POSITIVE

        // Within small range
        assert!(validate_range(very_small, "value", Some(0.0), Some(1.0)).is_ok());

        // Outside small range
        assert!(validate_range(very_small, "value", Some(1e-307), Some(1.0)).is_err());
    }

    // ========================================================================
    // Tests for zero
    // ========================================================================

    #[test]
    fn test_validate_range_zero() {
        // Zero at boundaries
        assert!(validate_range(0.0, "x", Some(0.0), Some(1.0)).is_ok());
        assert!(validate_range(0.0, "x", Some(-1.0), Some(0.0)).is_ok());

        // Zero in middle
        assert!(validate_range(0.0, "x", Some(-1.0), Some(1.0)).is_ok());

        // Zero outside range
        assert!(validate_range(0.0, "x", Some(1.0), Some(2.0)).is_err());
        assert!(validate_range(0.0, "x", Some(-2.0), Some(-1.0)).is_err());
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_validate_range_realistic_scenarios() {
        // Percentage (0-100)
        assert!(validate_range(50.0, "percentage", Some(0.0), Some(100.0)).is_ok());
        assert!(validate_range(-1.0, "percentage", Some(0.0), Some(100.0)).is_err());
        assert!(validate_range(101.0, "percentage", Some(0.0), Some(100.0)).is_err());

        // Probability (0-1)
        assert!(validate_range(0.5, "probability", Some(0.0), Some(1.0)).is_ok());
        assert!(validate_range(1.5, "probability", Some(0.0), Some(1.0)).is_err());

        // Temperature in Celsius (-273.15 to infinity)
        assert!(validate_range(-100.0, "temperature", Some(-273.15), None).is_ok());
        assert!(validate_range(-300.0, "temperature", Some(-273.15), None).is_err());

        // Age (0 to 150)
        assert!(validate_range(25.0, "age", Some(0.0), Some(150.0)).is_ok());
        assert!(validate_range(200.0, "age", Some(0.0), Some(150.0)).is_err());
    }
}
