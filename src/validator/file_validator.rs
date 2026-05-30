//! File validation utilities
//!
//! This module provides functions for validating file paths, including
//! checking for file existence and validating file extensions.
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::validator::{validate_file_exists, validate_file_extension};
//! use std::path::Path;
//!
//! let path = Path::new("config.yaml");
//!
//! // Check if file exists
//! validate_file_exists(path, "config")?;
//!
//! // Check if extension is allowed
//! let allowed = vec!["yaml".to_string(), "yml".to_string()];
//! validate_file_extension(path, "config", &allowed)?;
//!
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::error::{Result, ValidationError};
use std::path::Path;

/// Validate that a file or directory exists at the given path.
///
/// # Arguments
///
/// * `path` - The path to check
/// * `arg_name` - The argument name (used in error messages)
///
/// # Returns
///
/// - `Ok(())` if the path exists
/// - `Err(ValidationError::FileNotFound)` if the path does not exist
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::validator::validate_file_exists;
/// use std::path::Path;
///
/// validate_file_exists(Path::new("config.yaml"), "config")?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn validate_file_exists(path: &Path, arg_name: &str) -> Result<()> {
    if !path.exists() {
        return Err(ValidationError::FileNotFound {
            path: path.to_path_buf(),
            arg_name: arg_name.to_string(),
            suggestion: Some(format!(
                "Check that the file '{}' exists and the path is correct",
                path.display()
            )),
        }
        .into());
    }
    Ok(())
}

/// Validate that a file has an allowed extension.
///
/// The comparison is case-insensitive. Extensions should be provided
/// without the leading dot (e.g., `"yaml"`, not `".yaml"`).
///
/// # Arguments
///
/// * `path` - The path whose extension to check
/// * `arg_name` - The argument name (used in error messages)
/// * `allowed` - List of allowed extensions (without leading dot)
///
/// # Returns
///
/// - `Ok(())` if the extension is in the allowed list
/// - `Err(ValidationError::InvalidExtension)` otherwise
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::validator::validate_file_extension;
/// use std::path::Path;
///
/// let allowed = vec!["yaml".to_string(), "yml".to_string()];
/// validate_file_extension(Path::new("config.yaml"), "config", &allowed)?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn validate_file_extension(path: &Path, arg_name: &str, allowed: &[String]) -> Result<()> {
    if allowed.is_empty() {
        return Err(ValidationError::InvalidExtension {
            path: path.to_path_buf(),
            arg_name: arg_name.to_string(),
            expected: allowed.to_vec(),
        }
        .into());
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext {
        Some(ext) if allowed.iter().any(|a| a.to_lowercase() == ext) => Ok(()),
        _ => Err(ValidationError::InvalidExtension {
            path: path.to_path_buf(),
            arg_name: arg_name.to_string(),
            expected: allowed.to_vec(),
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper: create a NamedTempFile with a given extension and content.
    ///
    /// Uses `NamedTempFile` directly instead of `TempDir` + `File::create`
    /// to avoid the `File::create` race condition under parallel test execution.
    fn temp_file_with_ext(ext: &str, content: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new()
            .suffix(&format!(".{}", ext))
            .tempfile()
            .expect("failed to create NamedTempFile");
        f.write_all(content.as_bytes())
            .expect("failed to write to NamedTempFile");
        f
    }

    // ========================================================================
    // Tests for validate_file_exists
    // ========================================================================

    #[test]
    fn test_validate_file_exists_valid_file() {
        let f = temp_file_with_ext("txt", "content");
        assert!(validate_file_exists(f.path(), "test_file").is_ok());
    }

    #[test]
    fn test_validate_file_exists_valid_directory() {
        // std::env::temp_dir() always exists — no file creation needed.
        let dir = std::env::temp_dir();
        assert!(validate_file_exists(&dir, "test_dir").is_ok());
    }

    #[test]
    fn test_validate_file_exists_nonexistent() {
        let path = std::path::Path::new("/tmp/dynamic_cli_no_such_file_xyz.txt");
        let result = validate_file_exists(path, "missing_file");
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::FileNotFound {
                path: p,
                arg_name,
                ..
            }) => {
                assert_eq!(arg_name, "missing_file");
                assert_eq!(p, path);
            }
            other => panic!("Expected FileNotFound error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_exists_relative_path() {
        // Validates with a relative path guaranteed to exist at the project
        // root. Avoids set_current_dir() which mutates the process-wide
        // working directory and causes data races under parallel test execution.
        let relative = std::path::Path::new("Cargo.toml");
        assert!(validate_file_exists(relative, "cargo_manifest").is_ok());
    }

    // ========================================================================
    // Tests for validate_file_extension
    // ========================================================================

    #[test]
    fn test_validate_file_extension_valid_single() {
        let path = Path::new("config.yaml");
        let allowed = vec!["yaml".to_string()];
        assert!(validate_file_extension(path, "config", &allowed).is_ok());
    }

    #[test]
    fn test_validate_file_extension_valid_multiple() {
        let path = Path::new("data.csv");
        let allowed = vec!["csv".to_string(), "tsv".to_string(), "txt".to_string()];
        assert!(validate_file_extension(path, "data_file", &allowed).is_ok());
    }

    #[test]
    fn test_validate_file_extension_case_insensitive() {
        let allowed = vec!["yaml".to_string()];
        assert!(validate_file_extension(Path::new("config.YAML"), "config", &allowed).is_ok());
        assert!(validate_file_extension(Path::new("config.YaML"), "config", &allowed).is_ok());
        let allowed_upper = vec!["YAML".to_string()];
        assert!(
            validate_file_extension(Path::new("config.yaml"), "config", &allowed_upper).is_ok()
        );
    }

    #[test]
    fn test_validate_file_extension_invalid() {
        let path = Path::new("document.txt");
        let allowed = vec!["yaml".to_string(), "yml".to_string()];
        let result = validate_file_extension(path, "doc", &allowed);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Validation(ValidationError::InvalidExtension {
                arg_name,
                path: error_path,
                expected,
            }) => {
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
            crate::error::DynamicCliError::Validation(ValidationError::InvalidExtension {
                ..
            }) => {}
            other => panic!("Expected InvalidExtension error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_file_extension_hidden_file_with_extension() {
        let path = Path::new(".hidden.txt");
        let allowed = vec!["txt".to_string()];
        assert!(validate_file_extension(path, "hidden_file", &allowed).is_ok());
    }

    #[test]
    fn test_validate_file_extension_hidden_file_no_extension() {
        let path = Path::new(".gitignore");
        let allowed = vec!["txt".to_string()];
        assert!(validate_file_extension(path, "git_file", &allowed).is_err());
    }

    #[test]
    fn test_validate_file_extension_multiple_dots() {
        let path = Path::new("archive.tar.gz");
        let allowed = vec!["gz".to_string()];
        assert!(validate_file_extension(path, "archive", &allowed).is_ok());
    }

    #[test]
    fn test_validate_file_extension_empty_allowed_list() {
        let path = Path::new("file.txt");
        let allowed: Vec<String> = vec![];
        assert!(validate_file_extension(path, "file", &allowed).is_err());
    }

    #[test]
    fn test_validate_file_extension_with_leading_dot() {
        let path = Path::new("config.yaml");
        let allowed = vec!["yaml".to_string()];
        assert!(validate_file_extension(path, "config", &allowed).is_ok());
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_validate_both_file_and_extension() {
        let f = temp_file_with_ext("yaml", "key: value");
        assert!(validate_file_exists(f.path(), "config_file").is_ok());
        let allowed = vec!["yaml".to_string(), "yml".to_string()];
        assert!(validate_file_extension(f.path(), "config_file", &allowed).is_ok());
    }

    #[test]
    fn test_validate_wrong_extension_existing_file() {
        let f = temp_file_with_ext("txt", "some data");
        assert!(validate_file_exists(f.path(), "data_file").is_ok());
        let allowed = vec!["csv".to_string()];
        assert!(validate_file_extension(f.path(), "data_file", &allowed).is_err());
    }

    #[test]
    fn test_validate_extension_nonexistent_file() {
        let path = Path::new("nonexistent.yaml");
        let allowed = vec!["yaml".to_string()];
        assert!(validate_file_extension(path, "config", &allowed).is_ok());
        assert!(validate_file_exists(path, "config").is_err());
    }
}
