//! Argument validation module
//!
//! This module provides functions to validate parsed arguments against
//! the rules defined in the configuration. It completes the parsing
//! layer by ensuring values meet all specified constraints.
//!
//! # Overview
//!
//! The validator module works with [`ValidationRule`] definitions from
//! the configuration to validate parsed argument values. It supports:
//!
//! - **File validation**: Existence checks and extension restrictions
//! - **Range validation**: Numeric bounds (min/max)
//! - **Type-specific validation**: Applied after type parsing
//!
//! # Architecture
//!
//! ```text
//! Configuration (YAML/JSON)
//!     ↓
//! ValidationRule definitions
//!     ↓
//! Parsed arguments (HashMap<String, String>)
//!     ↓
//! Validators (this module)
//!     ↓
//! Validated values (Result<(), ValidationError>)
//! ```
//!
//! # Submodules
//!
//! - [`file_validator`]: File existence and extension validation
//! - [`range_validator`]: Numeric range validation
//!
//! # Usage Example
//!
//! ```no_run
//! use dynamic_cli::validator::{file_validator, range_validator};
//! use dynamic_cli::config::schema::ValidationRule;
//! use std::path::Path;
//!
//! // Validate file exists
//! let path = Path::new("config.yaml");
//! file_validator::validate_file_exists(path, "config")?;
//!
//! // Validate file extension
//! file_validator::validate_file_extension(
//!     path,
//!     "config",
//!     &["yaml".to_string(), "yml".to_string()]
//! )?;
//!
//! // Validate numeric range
//! range_validator::validate_range(75.0, "percentage", Some(0.0), Some(100.0))?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! # Integration with Other Modules
//!
//! ## With Parser Module
//!
//! The validator works with values after they've been parsed:
//!
//! ```text
//! User Input: "simulate input.dat --threshold 0.5"
//!     ↓ [parser]
//! HashMap { "input": "input.dat", "threshold": "0.5" }
//!     ↓ [validator]
//! Validated ✓
//! ```
//!
//! ## With Config Module
//!
//! Validation rules come from the configuration:
//!
//! ```yaml
//! arguments:
//!   - name: input
//!     arg_type: path
//!     validation:
//!       - must_exist: true
//!       - extensions: [dat, csv]
//!   - name: threshold
//!     arg_type: float
//!     validation:
//!       - min: 0.0
//!         max: 1.0
//! ```
//!
//! ## With Error Module
//!
//! All validation errors use [`ValidationError`]:
//!
//! - `FileNotFound` - File doesn't exist
//! - `InvalidExtension` - Wrong file extension
//! - `OutOfRange` - Value outside min/max bounds
//! - `CustomConstraint` - Custom validation failed
//!
//! # Complete Workflow Example
//!
//! ```no_run
//! use dynamic_cli::config::schema::{ArgumentDefinition, ArgumentType, ValidationRule};
//! use dynamic_cli::validator::{file_validator, validate_file_exists, validate_file_extension};
//! use std::collections::HashMap;
//! use std::path::Path;
//!
//! // Define an argument with validation rules
//! let arg_def = ArgumentDefinition {
//!     name: "input_file".to_string(),
//!     arg_type: ArgumentType::Path,
//!     required: true,
//!     description: "Input data file".to_string(),
//!     validation: vec![
//!         ValidationRule::MustExist { must_exist: true },
//!         ValidationRule::Extensions {
//!             extensions: vec!["csv".to_string(), "tsv".to_string()],
//!         },
//!     ],
//! };
//!
//! // Parse arguments
//! let mut args = HashMap::new();
//! args.insert("input_file".to_string(), "data.csv".to_string());
//!
//! // Validate (would check if file exists in real scenario)
//! if let Some(value) = args.get(&arg_def.name) {
//!     let path = Path::new(value);
//!     for rule in &arg_def.validation {
//!         match rule {
//!             ValidationRule::MustExist { must_exist } if *must_exist => {
//!                 validate_file_exists(path, &arg_def.name)?;
//!             },
//!             ValidationRule::Extensions { extensions } => {
//!                 validate_file_extension(path, &arg_def.name, extensions)?;
//!             },
//!             _ => {}
//!         }
//!     }
//! }
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! # Design Philosophy
//!
//! ## Fail Fast
//!
//! Validation happens early, before command execution, to catch
//! errors as soon as possible. This prevents wasted computation
//! and provides immediate feedback to users.
//!
//! ## Clear Error Messages
//!
//! All validation errors include:
//! - The argument name that failed
//! - The actual value provided
//! - The expected constraint
//! - Helpful context for fixing the issue
//!
//! ## Type-Appropriate Validation
//!
//! Validation rules are matched to argument types:
//! - `Path` arguments: file existence, extensions
//! - `Integer`/`Float` arguments: range constraints
//! - All types: custom constraints
//!
//! ## No Side Effects
//!
//! Validators only check values - they don't modify files,
//! create directories, or perform any side effects.
//!
//! [`ValidationRule`]: crate::config::schema::ValidationRule
//! [`ValidationError`]: crate::error::ValidationError
//! [`CommandDefinition`]: crate::config::schema::CommandDefinition

// Public submodules
pub mod file_validator;
pub mod range_validator;

// Re-export commonly used functions for convenience
pub use file_validator::{validate_file_exists, validate_file_extension};
pub use range_validator::validate_range;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, ValidationRule};
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper to create a temporary file
    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut file = fs::File::create(&path).unwrap();
        write!(file, "{}", content).unwrap();
        path
    }

    // ========================================================================
    // Integration tests - Real-world scenarios
    // ========================================================================

    #[test]
    fn test_validate_configuration_file_argument() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_temp_file(&temp_dir, "config.yaml", "test: value");

        // Define validation rules
        let extensions = vec!["yaml".to_string(), "yml".to_string()];

        // Validate - both checks should pass
        assert!(validate_file_exists(&config_path, "config").is_ok());
        assert!(validate_file_extension(&config_path, "config", &extensions).is_ok());
    }

    #[test]
    fn test_validate_data_file_with_wrong_extension() {
        let temp_dir = TempDir::new().unwrap();
        let data_path = create_temp_file(&temp_dir, "data.txt", "some data");

        let extensions = vec!["csv".to_string(), "tsv".to_string()];

        // File exists
        assert!(validate_file_exists(&data_path, "data").is_ok());

        // But wrong extension
        assert!(validate_file_extension(&data_path, "data", &extensions).is_err());
    }

    #[test]
    fn test_validate_missing_file() {
        let missing_path = Path::new("/nonexistent/file.dat");

        // Should fail existence check
        assert!(validate_file_exists(missing_path, "input").is_err());

        // Extension check would pass (but we stop at existence)
        let extensions = vec!["dat".to_string()];
        assert!(validate_file_extension(missing_path, "input", &extensions).is_ok());
    }

    #[test]
    fn test_validate_percentage_argument() {
        // Valid percentages
        assert!(validate_range(0.0, "percentage", Some(0.0), Some(100.0)).is_ok());
        assert!(validate_range(50.0, "percentage", Some(0.0), Some(100.0)).is_ok());
        assert!(validate_range(100.0, "percentage", Some(0.0), Some(100.0)).is_ok());

        // Invalid percentages
        assert!(validate_range(-1.0, "percentage", Some(0.0), Some(100.0)).is_err());
        assert!(validate_range(101.0, "percentage", Some(0.0), Some(100.0)).is_err());
    }

    #[test]
    fn test_validate_threshold_argument() {
        // Common ML/science threshold: 0.0 to 1.0
        assert!(validate_range(0.0, "threshold", Some(0.0), Some(1.0)).is_ok());
        assert!(validate_range(0.5, "threshold", Some(0.0), Some(1.0)).is_ok());
        assert!(validate_range(1.0, "threshold", Some(0.0), Some(1.0)).is_ok());

        // Out of bounds
        assert!(validate_range(-0.1, "threshold", Some(0.0), Some(1.0)).is_err());
        assert!(validate_range(1.1, "threshold", Some(0.0), Some(1.0)).is_err());
    }

    // ========================================================================
    // Multiple validation rules on same argument
    // ========================================================================

    #[test]
    fn test_validate_argument_with_multiple_rules() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "data.csv", "col1,col2\n1,2\n");

        // Simulate ArgumentDefinition with multiple validation rules
        let rules = vec![
            ValidationRule::MustExist { must_exist: true },
            ValidationRule::Extensions {
                extensions: vec!["csv".to_string(), "tsv".to_string()],
            },
        ];

        // Apply all rules
        for rule in &rules {
            match rule {
                ValidationRule::MustExist { must_exist } => {
                    if *must_exist {
                        assert!(validate_file_exists(&file_path, "data").is_ok());
                    }
                }
                ValidationRule::Extensions { extensions } => {
                    assert!(validate_file_extension(&file_path, "data", extensions).is_ok());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_validate_fails_at_first_invalid_rule() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "data.txt", "content");

        let rules = vec![
            ValidationRule::MustExist { must_exist: true },
            ValidationRule::Extensions {
                extensions: vec!["csv".to_string()], // Wrong extension!
            },
        ];

        // First rule passes
        if let ValidationRule::MustExist { must_exist } = &rules[0] {
            if *must_exist {
                assert!(validate_file_exists(&file_path, "data").is_ok());
            }
        }

        // Second rule fails
        if let ValidationRule::Extensions { extensions } = &rules[1] {
            assert!(validate_file_extension(&file_path, "data", extensions).is_err());
        }
    }

    // ========================================================================
    // Testing validation with ArgumentDefinition structure
    // ========================================================================

    #[test]
    fn test_validate_with_argument_definition() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "input.dat", "data");

        // Create an ArgumentDefinition similar to config
        let arg_def = ArgumentDefinition {
            name: "input_file".to_string(),
            arg_type: ArgumentType::Path,
            required: true,
            description: "Input data file".to_string(),
            validation: vec![
                ValidationRule::MustExist { must_exist: true },
                ValidationRule::Extensions {
                    extensions: vec!["dat".to_string(), "bin".to_string()],
                },
            ],
        };

        // Validate according to definition
        let value = file_path.to_str().unwrap();

        for rule in &arg_def.validation {
            match rule {
                ValidationRule::MustExist { must_exist } => {
                    if *must_exist {
                        let path = Path::new(value);
                        assert!(validate_file_exists(path, &arg_def.name).is_ok());
                    }
                }
                ValidationRule::Extensions { extensions } => {
                    let path = Path::new(value);
                    assert!(validate_file_extension(path, &arg_def.name, extensions).is_ok());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_validate_numeric_argument_with_definition() {
        let arg_def = ArgumentDefinition {
            name: "temperature".to_string(),
            arg_type: ArgumentType::Float,
            required: true,
            description: "Temperature in Celsius".to_string(),
            validation: vec![ValidationRule::Range {
                min: Some(-273.15), // Absolute zero
                max: None,
            }],
        };

        // Valid temperatures
        let values = vec!["0.0", "25.0", "100.0", "-273.15"];

        for value in values {
            let num_value: f64 = value.parse().unwrap();

            for rule in &arg_def.validation {
                if let ValidationRule::Range { min, max } = rule {
                    assert!(validate_range(num_value, &arg_def.name, *min, *max).is_ok());
                }
            }
        }

        // Invalid temperature (below absolute zero)
        let invalid_value: f64 = "-300.0".parse().unwrap();
        if let ValidationRule::Range { min, max } = &arg_def.validation[0] {
            assert!(validate_range(invalid_value, &arg_def.name, *min, *max).is_err());
        }
    }

    // ========================================================================
    // Cross-module integration tests
    // ========================================================================

    #[test]
    fn test_validate_parsed_arguments() {
        use std::collections::HashMap;

        let temp_dir = TempDir::new().unwrap();
        let input_path = create_temp_file(&temp_dir, "data.csv", "1,2,3");

        // Simulated parsed arguments from parser module
        let mut parsed_args = HashMap::new();
        parsed_args.insert(
            "input".to_string(),
            input_path.to_str().unwrap().to_string(),
        );
        parsed_args.insert("threshold".to_string(), "0.75".to_string());

        // Validation rules from config
        let input_rules = vec![
            ValidationRule::MustExist { must_exist: true },
            ValidationRule::Extensions {
                extensions: vec!["csv".to_string()],
            },
        ];

        let threshold_rules = vec![ValidationRule::Range {
            min: Some(0.0),
            max: Some(1.0),
        }];

        // Validate input file
        if let Some(value) = parsed_args.get("input") {
            let path = Path::new(value);
            for rule in &input_rules {
                match rule {
                    ValidationRule::MustExist { must_exist } if *must_exist => {
                        assert!(validate_file_exists(path, "input").is_ok());
                    }
                    ValidationRule::Extensions { extensions } => {
                        assert!(validate_file_extension(path, "input", extensions).is_ok());
                    }
                    _ => {}
                }
            }
        }

        // Validate threshold
        if let Some(value) = parsed_args.get("threshold") {
            let num_value: f64 = value.parse().unwrap();
            for rule in &threshold_rules {
                if let ValidationRule::Range { min, max } = rule {
                    assert!(validate_range(num_value, "threshold", *min, *max).is_ok());
                }
            }
        }
    }

    #[test]
    fn test_error_messages_are_descriptive() {
        // Test that error messages contain helpful information

        // File not found error
        let result = validate_file_exists(Path::new("/missing/file.txt"), "config");
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("File not found"));
        assert!(error_msg.contains("config"));
        assert!(error_msg.contains("/missing/file.txt"));

        // Invalid extension error
        let result = validate_file_extension(
            Path::new("file.txt"),
            "data",
            &vec!["csv".to_string(), "json".to_string()],
        );
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Invalid file extension"));
        assert!(error_msg.contains("data"));
        assert!(error_msg.contains("csv"));

        // Out of range error
        let result = validate_range(150.0, "percentage", Some(0.0), Some(100.0));
        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("percentage"));
        assert!(error_msg.contains("150"));
        assert!(error_msg.contains("0"));
        assert!(error_msg.contains("100"));
    }
}
