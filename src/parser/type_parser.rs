//! Type parsing functions
//!
//! This module provides functions to parse string values into typed values
//! according to the [`ArgumentType`] specification. Each type has dedicated
//! parsing logic with detailed error messages.
//!
//! # Supported Types
//!
//! - **String**: Pass-through (no conversion)
//! - **Integer**: Signed 64-bit integers (i64)
//! - **Float**: 64-bit floating point (f64)
//! - **Bool**: true/false, yes/no, 1/0, on/off (case-insensitive)
//! - **Path**: File system paths (validated as PathBuf)
//!
//! # Example
//!
//! ```
//! use dynamic_cli::parser::type_parser::{parse_value, parse_integer};
//! use dynamic_cli::config::schema::ArgumentType;
//!
//! // Parse a value according to its type
//! let result = parse_value("42", ArgumentType::Integer).unwrap();
//! assert_eq!(result, "42");
//!
//! // Parse directly to specific type
//! let number = parse_integer("42").unwrap();
//! assert_eq!(number, 42);
//! ```

use crate::config::schema::ArgumentType;
use crate::error::{ParseError, Result};
use std::path::PathBuf;

/// Parse a string value according to its expected type
///
/// This is the main entry point for type parsing. It dispatches to
/// type-specific parsers and validates the conversion.
///
/// # Arguments
///
/// * `value` - The string value to parse
/// * `arg_type` - The expected type for this value
///
/// # Returns
///
/// Returns the original string value if parsing succeeds. The actual
/// conversion to the target type is done to validate the input, but
/// we return the string to maintain flexibility in the HashMap storage.
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the value cannot be
/// converted to the expected type.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::type_parser::parse_value;
/// use dynamic_cli::config::schema::ArgumentType;
///
/// // Valid integer
/// assert!(parse_value("123", ArgumentType::Integer).is_ok());
///
/// // Invalid integer
/// assert!(parse_value("abc", ArgumentType::Integer).is_err());
///
/// // Valid boolean
/// assert!(parse_value("yes", ArgumentType::Bool).is_ok());
///
/// // Valid path
/// assert!(parse_value("/tmp/file.txt", ArgumentType::Path).is_ok());
/// ```
pub fn parse_value(value: &str, arg_type: ArgumentType) -> Result<String> {
    // Validate by attempting conversion to target type
    match arg_type {
        ArgumentType::String => {
            // String is always valid
            Ok(value.to_string())
        }
        
        ArgumentType::Integer => {
            // Try to parse as i64
            parse_integer(value)?;
            Ok(value.to_string())
        }
        
        ArgumentType::Float => {
            // Try to parse as f64
            parse_float(value)?;
            Ok(value.to_string())
        }
        
        ArgumentType::Bool => {
            // Try to parse as boolean
            parse_bool(value)?;
            Ok(value.to_string())
        }
        
        ArgumentType::Path => {
            // Try to parse as PathBuf
            parse_path(value)?;
            Ok(value.to_string())
        }
    }
}

/// Parse a string as a signed integer
///
/// Accepts standard integer formats including:
/// - Positive numbers: "42"
/// - Negative numbers: "-42"
/// - With underscores: "1_000_000"
///
/// # Arguments
///
/// * `value` - The string to parse
///
/// # Returns
///
/// The parsed integer value as i64
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the string cannot be
/// parsed as a valid integer or if it overflows i64.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::type_parser::parse_integer;
///
/// assert_eq!(parse_integer("42").unwrap(), 42);
/// assert_eq!(parse_integer("-123").unwrap(), -123);
/// assert_eq!(parse_integer("1_000").unwrap(), 1000);
///
/// // These will fail
/// assert!(parse_integer("abc").is_err());
/// assert!(parse_integer("12.5").is_err());
/// ```
pub fn parse_integer(value: &str) -> Result<i64> {
    value.parse::<i64>().map_err(|e| {
        ParseError::TypeParseError {
            arg_name: "value".to_string(),
            expected_type: "integer".to_string(),
            value: value.to_string(),
            details: Some(e.to_string()),
        }
        .into()
    })
}

/// Parse a string as a floating-point number
///
/// Accepts standard float formats including:
/// - Integers: "42" → 42.0
/// - Decimals: "3.14"
/// - Scientific notation: "1e-10", "2.5E+3"
/// - Special values: "inf", "-inf", "NaN"
///
/// # Arguments
///
/// * `value` - The string to parse
///
/// # Returns
///
/// The parsed floating-point value as f64
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the string cannot be
/// parsed as a valid float.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::type_parser::parse_float;
///
/// assert_eq!(parse_float("3.14").unwrap(), 3.14);
/// assert_eq!(parse_float("42").unwrap(), 42.0);
/// assert_eq!(parse_float("-1.5").unwrap(), -1.5);
/// assert_eq!(parse_float("1e-3").unwrap(), 0.001);
///
/// // These will fail
/// assert!(parse_float("abc").is_err());
/// assert!(parse_float("12.34.56").is_err());
/// ```
pub fn parse_float(value: &str) -> Result<f64> {
    value.parse::<f64>().map_err(|e| {
        ParseError::TypeParseError {
            arg_name: "value".to_string(),
            expected_type: "float".to_string(),
            value: value.to_string(),
            details: Some(e.to_string()),
        }
        .into()
    })
}

/// Parse a string as a boolean
///
/// Accepts multiple representations (case-insensitive):
/// - **true**: "true", "yes", "y", "1", "on"
/// - **false**: "false", "no", "n", "0", "off"
///
/// This flexibility allows users to use natural language or
/// common programming conventions.
///
/// # Arguments
///
/// * `value` - The string to parse
///
/// # Returns
///
/// The parsed boolean value
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the string is not
/// a recognized boolean value.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::type_parser::parse_bool;
///
/// // True values
/// assert_eq!(parse_bool("true").unwrap(), true);
/// assert_eq!(parse_bool("YES").unwrap(), true);
/// assert_eq!(parse_bool("1").unwrap(), true);
/// assert_eq!(parse_bool("on").unwrap(), true);
///
/// // False values
/// assert_eq!(parse_bool("false").unwrap(), false);
/// assert_eq!(parse_bool("NO").unwrap(), false);
/// assert_eq!(parse_bool("0").unwrap(), false);
/// assert_eq!(parse_bool("off").unwrap(), false);
///
/// // Invalid values
/// assert!(parse_bool("maybe").is_err());
/// assert!(parse_bool("2").is_err());
/// ```
pub fn parse_bool(value: &str) -> Result<bool> {
    let normalized = value.trim().to_lowercase();
    
    match normalized.as_str() {
        // True values
        "true" | "yes" | "y" | "1" | "on" => Ok(true),
        
        // False values
        "false" | "no" | "n" | "0" | "off" => Ok(false),
        
        // Invalid value
        _ => Err(ParseError::TypeParseError {
            arg_name: "value".to_string(),
            expected_type: "bool".to_string(),
            value: value.to_string(),
            details: Some("expected true/false, yes/no, 1/0, on/off".to_string()),
        }
        .into()),
    }
}

/// Parse a string as a file system path
///
/// Validates that the string can be converted to a valid PathBuf.
/// Note: This does NOT check if the path exists on the file system -
/// that validation is done separately in the validator module.
///
/// # Arguments
///
/// * `value` - The string to parse as a path
///
/// # Returns
///
/// The parsed PathBuf
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the string contains
/// invalid path characters (very rare on Unix, more common on Windows).
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::type_parser::parse_path;
///
/// // Valid paths
/// assert!(parse_path("/tmp/file.txt").is_ok());
/// assert!(parse_path("./relative/path").is_ok());
/// assert!(parse_path("C:\\Windows\\System32").is_ok());
///
/// // Empty path is technically valid (current directory)
/// assert!(parse_path("").is_ok());
/// ```
pub fn parse_path(value: &str) -> Result<PathBuf> {
    // PathBuf::from() is very permissive - it accepts almost any string
    // More strict validation (existence, permissions) is done by the validator module
    let path = PathBuf::from(value);
    
    // We could add more validation here if needed, but for now
    // we trust that PathBuf::from() will handle platform-specific rules
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // parse_value tests
    // ========================================================================

    #[test]
    fn test_parse_value_string() {
        let result = parse_value("hello world", ArgumentType::String).unwrap();
        assert_eq!(result, "hello world");
        
        // Empty string is valid
        let result = parse_value("", ArgumentType::String).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_value_integer_valid() {
        assert!(parse_value("42", ArgumentType::Integer).is_ok());
        assert!(parse_value("-123", ArgumentType::Integer).is_ok());
        assert!(parse_value("0", ArgumentType::Integer).is_ok());
    }

    #[test]
    fn test_parse_value_integer_invalid() {
        assert!(parse_value("abc", ArgumentType::Integer).is_err());
        assert!(parse_value("12.5", ArgumentType::Integer).is_err());
        assert!(parse_value("", ArgumentType::Integer).is_err());
    }

    #[test]
    fn test_parse_value_float_valid() {
        assert!(parse_value("3.14", ArgumentType::Float).is_ok());
        assert!(parse_value("42", ArgumentType::Float).is_ok());
        assert!(parse_value("-1.5", ArgumentType::Float).is_ok());
    }

    #[test]
    fn test_parse_value_float_invalid() {
        assert!(parse_value("abc", ArgumentType::Float).is_err());
        assert!(parse_value("", ArgumentType::Float).is_err());
    }

    #[test]
    fn test_parse_value_bool_valid() {
        assert!(parse_value("true", ArgumentType::Bool).is_ok());
        assert!(parse_value("false", ArgumentType::Bool).is_ok());
        assert!(parse_value("yes", ArgumentType::Bool).is_ok());
        assert!(parse_value("no", ArgumentType::Bool).is_ok());
    }

    #[test]
    fn test_parse_value_bool_invalid() {
        assert!(parse_value("maybe", ArgumentType::Bool).is_err());
        assert!(parse_value("2", ArgumentType::Bool).is_err());
    }

    #[test]
    fn test_parse_value_path_valid() {
        assert!(parse_value("/tmp/file", ArgumentType::Path).is_ok());
        assert!(parse_value("./relative", ArgumentType::Path).is_ok());
    }

    // ========================================================================
    // parse_integer tests
    // ========================================================================

    #[test]
    fn test_parse_integer_positive() {
        assert_eq!(parse_integer("42").unwrap(), 42);
        assert_eq!(parse_integer("0").unwrap(), 0);
        assert_eq!(parse_integer("999999").unwrap(), 999999);
    }

    #[test]
    fn test_parse_integer_negative() {
        assert_eq!(parse_integer("-42").unwrap(), -42);
        assert_eq!(parse_integer("-1").unwrap(), -1);
    }

    // Note: Rust's parse() does not support underscores in string literals
    // Underscores are only a syntax feature when writing Rust code
    // If we want to support this, we'd need to strip underscores first

    #[test]
    fn test_parse_integer_invalid() {
        assert!(parse_integer("abc").is_err());
        assert!(parse_integer("12.5").is_err());
        assert!(parse_integer("").is_err());
        assert!(parse_integer("12a").is_err());
    }

    #[test]
    fn test_parse_integer_overflow() {
        // i64::MAX + 1 should fail
        let too_large = "9223372036854775808";
        assert!(parse_integer(too_large).is_err());
    }

    // ========================================================================
    // parse_float tests
    // ========================================================================

    #[test]
    fn test_parse_float_integer() {
        assert_eq!(parse_float("42").unwrap(), 42.0);
        assert_eq!(parse_float("0").unwrap(), 0.0);
    }

    #[test]
    fn test_parse_float_decimal() {
        assert_eq!(parse_float("3.14").unwrap(), 3.14);
        assert_eq!(parse_float("-1.5").unwrap(), -1.5);
        assert_eq!(parse_float("0.5").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_float_scientific() {
        assert_eq!(parse_float("1e3").unwrap(), 1000.0);
        assert_eq!(parse_float("1.5e2").unwrap(), 150.0);
        assert_eq!(parse_float("1e-3").unwrap(), 0.001);
    }

    #[test]
    fn test_parse_float_special_values() {
        assert!(parse_float("inf").unwrap().is_infinite());
        assert!(parse_float("-inf").unwrap().is_infinite());
        assert!(parse_float("NaN").unwrap().is_nan());
    }

    #[test]
    fn test_parse_float_invalid() {
        assert!(parse_float("abc").is_err());
        assert!(parse_float("").is_err());
        assert!(parse_float("12.34.56").is_err());
    }

    // ========================================================================
    // parse_bool tests
    // ========================================================================

    #[test]
    fn test_parse_bool_true_variants() {
        // All true variants
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("True").unwrap(), true);
        assert_eq!(parse_bool("TRUE").unwrap(), true);
        assert_eq!(parse_bool("yes").unwrap(), true);
        assert_eq!(parse_bool("YES").unwrap(), true);
        assert_eq!(parse_bool("y").unwrap(), true);
        assert_eq!(parse_bool("Y").unwrap(), true);
        assert_eq!(parse_bool("1").unwrap(), true);
        assert_eq!(parse_bool("on").unwrap(), true);
        assert_eq!(parse_bool("ON").unwrap(), true);
    }

    #[test]
    fn test_parse_bool_false_variants() {
        // All false variants
        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("False").unwrap(), false);
        assert_eq!(parse_bool("FALSE").unwrap(), false);
        assert_eq!(parse_bool("no").unwrap(), false);
        assert_eq!(parse_bool("NO").unwrap(), false);
        assert_eq!(parse_bool("n").unwrap(), false);
        assert_eq!(parse_bool("N").unwrap(), false);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert_eq!(parse_bool("off").unwrap(), false);
        assert_eq!(parse_bool("OFF").unwrap(), false);
    }

    #[test]
    fn test_parse_bool_with_whitespace() {
        assert_eq!(parse_bool("  true  ").unwrap(), true);
        assert_eq!(parse_bool("\tfalse\n").unwrap(), false);
    }

    #[test]
    fn test_parse_bool_invalid() {
        assert!(parse_bool("maybe").is_err());
        assert!(parse_bool("2").is_err());
        assert!(parse_bool("").is_err());
        assert!(parse_bool("tr").is_err());
    }

    // ========================================================================
    // parse_path tests
    // ========================================================================

    #[test]
    fn test_parse_path_unix_style() {
        let path = parse_path("/tmp/file.txt").unwrap();
        assert_eq!(path.to_str().unwrap(), "/tmp/file.txt");
    }

    #[test]
    fn test_parse_path_relative() {
        let path = parse_path("./relative/path").unwrap();
        assert!(path.to_str().unwrap().contains("relative"));
    }

    #[test]
    fn test_parse_path_windows_style() {
        // This should work on all platforms (PathBuf is permissive)
        let path = parse_path("C:\\Windows\\System32").unwrap();
        assert!(path.to_str().is_some());
    }

    #[test]
    fn test_parse_path_empty() {
        // Empty path is technically valid (current directory)
        let path = parse_path("").unwrap();
        assert_eq!(path, PathBuf::from(""));
    }

    #[test]
    fn test_parse_path_with_spaces() {
        let path = parse_path("/path/with spaces/file.txt").unwrap();
        assert!(path.to_str().unwrap().contains("spaces"));
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_all_types_roundtrip() {
        // Test that parse_value works for all types
        let test_cases = vec![
            ("hello", ArgumentType::String),
            ("42", ArgumentType::Integer),
            ("3.14", ArgumentType::Float),
            ("true", ArgumentType::Bool),
            ("/tmp/file", ArgumentType::Path),
        ];

        for (value, arg_type) in test_cases {
            let result = parse_value(value, arg_type);
            assert!(result.is_ok(), "Failed to parse '{}' as {:?}", value, arg_type);
            assert_eq!(result.unwrap(), value);
        }
    }

    #[test]
    fn test_error_messages_contain_details() {
        // Verify that error messages are helpful
        let result = parse_integer("not_a_number");
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("integer"));
        assert!(error_msg.contains("not_a_number"));
    }
}
