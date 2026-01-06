//! File validation functions
//!
//! This module provides functions to validate file paths according to
//! [`ValidationRule::MustExist`] and [`ValidationRule::Extensions`].
//!
//! # Functions
//!
//! - [`validate_file_exists`] - Check if a file or directory exists
//! - [`validate_file_extension`] - Check if a file has an allowed extension
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::validator::file_validator::{validate_file_exists, validate_file_extension};
//! use std::path::Path;
//!
//! let path = Path::new("config.yaml");
//!
//! // Validate file exists
//! validate_file_exists(path, "config_file")?;
//!
//! // Validate file extension
//! let allowed = vec!["yaml".to_string(), "yml".to_string()];
//! validate_file_extension(path, "config_file", &allowed)?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::error::{Result, ValidationError};
use std::path::Path;

/// Validate that a file or directory exists
///
/// This function checks if the given path exists on the file system.
/// It works for both files and directories.
///
/// # Arguments
///
/// * `path` - Path to validate
/// * `arg_name` - Name of the argument (for error messages)
///
/// # Returns
///
/// - `Ok(())` if the path exists
/// - `Err(ValidationError::FileNotFound)` if the path doesn't exist
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::validator::file_validator::validate_file_exists;
/// use std::path::Path;
///
/// let path = Path::new("input.txt");
/// validate_file_exists(path, "input_file")?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
///
/// # Error Messages
///
/// If the file doesn't exist, the error message includes:
/// - The argument name
/// - The full path that was checked
///
/// Example: `File not found for argument 'input_file': "/path/to/input.txt"`
pub fn validate_file_exists(path: &Path, arg_name: &str) -> Result<()> {
    // Check if the path exists on the file system
    // This works for both files and directories
    if !path.exists() {
        return Err(ValidationError::FileNotFound {
            path: path.to_path_buf(),
            arg_name: arg_name.to_string(),
        }
        .into());
    }

    Ok(())
}

/// Validate that a file has one of the expected extensions
///
/// This function checks if the file's extension matches one of the
/// allowed extensions. The comparison is **case-insensitive**.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `arg_name` - Name of the argument (for error messages)
/// * `expected` - List of allowed extensions (without the leading dot)
///
/// # Returns
///
/// - `Ok(())` if the file has an allowed extension
/// - `Err(ValidationError::InvalidExtension)` if the extension doesn't match
///
/// # Extension Format
///
/// Extensions should be specified **without** the leading dot:
/// - ✅ Correct: `["yaml", "yml"]`
/// - ❌ Incorrect: `[".yaml", ".yml"]`
///
/// # Case Sensitivity
///
/// The validation is **case-insensitive**:
/// - `"config.YAML"` matches `["yaml"]` ✅
/// - `"config.Yml"` matches `["yml"]` ✅
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::validator::file_validator::validate_file_extension;
/// use std::path::Path;
///
/// let path = Path::new("config.yaml");
/// let allowed = vec!["yaml".to_string(), "yml".to_string()];
///
/// validate_file_extension(path, "config_file", &allowed)?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
///
/// # Error Messages
///
/// If the extension is invalid, the error includes:
/// - The argument name
/// - The file path
/// - The list of expected extensions
///
/// Example: `Invalid file extension for config_file: "config.txt". Expected: yaml, yml`
pub fn validate_file_extension(
    path: &Path,
    arg_name: &str,
    expected: &[String],
) -> Result<()> {
    // Extract the file extension from the path
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    // Check if we have an extension
    let ext = match extension {
        Some(e) => e,
        None => {
            // File has no extension
            return Err(ValidationError::InvalidExtension {
                arg_name: arg_name.to_string(),
                path: path.to_path_buf(),
                expected: expected.to_vec(),
            }
            .into());
        }
    };

    // Check if the extension is in the list of allowed extensions
    // Convert expected extensions to lowercase for case-insensitive comparison
    let is_valid = expected.iter().any(|allowed| allowed.to_lowercase() == ext);

    if !is_valid {
        return Err(ValidationError::InvalidExtension {
            arg_name: arg_name.to_string(),
            path: path.to_path_buf(),
            expected: expected.to_vec(),
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a temporary file with content
    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.path().join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    // ========================================================================
    // Tests for validate_file_exists
    // ========================================================================

    #[test]
    fn test_validate_file_exists_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "test.txt", "content");

        let result = validate_file_exists(&file_path, "test_file");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_exists_valid_directory() {
        let temp_dir = TempDir::new().unwrap();

        // The temp directory itself exists
        let result = validate_file_exists(temp_dir.path(), "test_dir");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_exists_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist.txt");

        let result = validate_file_exists(&nonexistent, "missing_file");

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(
                ValidationError::FileNotFound { path, arg_name },
            ) => {
                assert_eq!(arg_name, "missing_file");
                assert_eq!(path, nonexistent);
            }
            other => panic!("Expected FileNotFound error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_exists_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "relative.txt", "content");

        // Create a relative path by using only the filename
        let relative = std::path::Path::new(file_path.file_name().unwrap());

        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = validate_file_exists(relative, "relative_file");
        assert!(result.is_ok());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    // ========================================================================
    // Tests for validate_file_extension
    // ========================================================================

    #[test]
    fn test_validate_file_extension_valid_single() {
        let path = Path::new("config.yaml");
        let allowed = vec!["yaml".to_string()];

        let result = validate_file_extension(path, "config", &allowed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_extension_valid_multiple() {
        let path = Path::new("data.csv");
        let allowed = vec!["csv".to_string(), "tsv".to_string(), "txt".to_string()];

        let result = validate_file_extension(path, "data_file", &allowed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_extension_case_insensitive() {
        // File with uppercase extension
        let path1 = Path::new("config.YAML");
        let allowed = vec!["yaml".to_string()];

        assert!(validate_file_extension(path1, "config", &allowed).is_ok());

        // File with mixed case extension
        let path2 = Path::new("config.YaML");
        assert!(validate_file_extension(path2, "config", &allowed).is_ok());

        // Allowed extensions in uppercase
        let path3 = Path::new("config.yaml");
        let allowed_upper = vec!["YAML".to_string()];
        assert!(validate_file_extension(path3, "config", &allowed_upper).is_ok());
    }

    #[test]
    fn test_validate_file_extension_invalid() {
        let path = Path::new("document.txt");
        let allowed = vec!["yaml".to_string(), "yml".to_string()];

        let result = validate_file_extension(path, "doc", &allowed);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(
                ValidationError::InvalidExtension {
                    arg_name,
                    path: error_path,
                    expected,
                },
            ) => {
                assert_eq!(arg_name, "doc");
                assert_eq!(error_path, path);
                assert_eq!(expected, allowed);
            }
            other => panic!("Expected InvalidExtension error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_extension_no_extension() {
        let path = Path::new("makefile");
        let allowed = vec!["txt".to_string()];

        let result = validate_file_extension(path, "build_file", &allowed);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(
                ValidationError::InvalidExtension { .. },
            ) => {
                // Expected
            }
            other => panic!("Expected InvalidExtension error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_extension_hidden_file_with_extension() {
        // Hidden files WITH extensions work normally
        // .hidden.txt has extension "txt"
        let path = Path::new(".hidden.txt");
        let allowed = vec!["txt".to_string()];

        let result = validate_file_extension(path, "hidden_file", &allowed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_extension_hidden_file_no_extension() {
        // Pure hidden files (like .gitignore) have NO extension
        // This should fail validation
        let path = Path::new(".gitignore");
        let allowed = vec!["txt".to_string()];

        let result = validate_file_extension(path, "git_file", &allowed);
        // .gitignore has no extension, so validation should fail
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_extension_multiple_dots() {
        let path = Path::new("archive.tar.gz");
        let allowed = vec!["gz".to_string()];

        // Only the last extension is checked
        let result = validate_file_extension(path, "archive", &allowed);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_extension_empty_allowed_list() {
        let path = Path::new("file.txt");
        let allowed: Vec<String> = vec![];

        let result = validate_file_extension(path, "file", &allowed);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_extension_with_leading_dot() {
        // Even though we specify extensions without dots,
        // the function should still work correctly
        let path = Path::new("config.yaml");

        // User mistakenly includes the dot - should still work
        // because we compare lowercase extensions
        let allowed = vec!["yaml".to_string()];

        let result = validate_file_extension(path, "config", &allowed);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_validate_both_file_and_extension() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "config.yaml", "key: value");

        // First validate existence
        let result1 = validate_file_exists(&file_path, "config_file");
        assert!(result1.is_ok());

        // Then validate extension
        let allowed = vec!["yaml".to_string(), "yml".to_string()];
        let result2 = validate_file_extension(&file_path, "config_file", &allowed);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_validate_wrong_extension_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "data.txt", "some data");

        // File exists
        assert!(validate_file_exists(&file_path, "data_file").is_ok());

        // But extension is wrong
        let allowed = vec!["csv".to_string()];
        let result = validate_file_extension(&file_path, "data_file", &allowed);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_extension_nonexistent_file() {
        // Extension validation doesn't check if file exists
        let path = Path::new("nonexistent.yaml");
        let allowed = vec!["yaml".to_string()];

        // Extension validation succeeds (only checks extension)
        let result = validate_file_extension(path, "config", &allowed);
        assert!(result.is_ok());

        // But existence validation fails
        let result2 = validate_file_exists(path, "config");
        assert!(result2.is_err());
    }
}
