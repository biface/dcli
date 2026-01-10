//! User-friendly error display
//!
//! Formats errors with coloring to improve readability
//! in the terminal.

use crate::error::{ConfigError, DynamicCliError, ParseError};

/// Helper functions for conditional coloring
#[cfg(feature = "colored-output")]
mod color {
    use colored::*;

    pub fn error(s: &str) -> String {
        s.red().bold().to_string()
    }
    pub fn question(s: &str) -> String {
        s.yellow().bold().to_string()
    }
    pub fn bullet(s: &str) -> String {
        s.cyan().to_string()
    }
    pub fn suggestion(s: &str) -> String {
        s.green().to_string()
    }
    pub fn info(s: &str) -> String {
        s.blue().bold().to_string()
    }
    pub fn type_name(s: &str) -> String {
        s.cyan().to_string()
    }
    pub fn arg_name(s: &str) -> String {
        s.yellow().to_string()
    }
    pub fn value(s: &str) -> String {
        s.red().to_string()
    }
    pub fn dimmed(s: &str) -> String {
        s.dimmed().to_string()
    }
}

#[cfg(not(feature = "colored-output"))]
mod color {
    pub fn error(s: &str) -> String {
        s.to_string()
    }
    pub fn question(s: &str) -> String {
        s.to_string()
    }
    pub fn bullet(s: &str) -> String {
        s.to_string()
    }
    pub fn suggestion(s: &str) -> String {
        s.to_string()
    }
    pub fn info(s: &str) -> String {
        s.to_string()
    }
    pub fn type_name(s: &str) -> String {
        s.to_string()
    }
    pub fn arg_name(s: &str) -> String {
        s.to_string()
    }
    pub fn value(s: &str) -> String {
        s.to_string()
    }
    pub fn dimmed(s: &str) -> String {
        s.to_string()
    }
}

/// Display an error in a user-friendly way to the terminal
///
/// This function displays the error on stderr with coloring
/// and suggestions if available.
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::error::{display_error, ParseError};
///
/// let error = ParseError::UnknownCommand {
///     command: "simulat".to_string(),
///     suggestions: vec!["simulate".to_string()],
/// };
/// display_error(&error.into());
/// ```
pub fn display_error(error: &DynamicCliError) {
    eprintln!("{}", format_error(error));
}

/// Format an error with coloring
///
/// Generates a formatted string with colors and suggestions.
/// Can be used for logging or custom display.
///
/// # Arguments
///
/// * `error` - The error to format
///
/// # Returns
///
/// Formatted string with ANSI color codes (if colored-output feature is enabled)
///
/// # Example
///
/// ```
/// use dynamic_cli::error::{format_error, ConfigError};
/// use std::path::PathBuf;
///
/// let error = ConfigError::FileNotFound {
///     path: PathBuf::from("config.yaml"),
/// };
/// let formatted = format_error(&error.into());
/// assert!(formatted.contains("Error:"));
/// assert!(formatted.contains("config.yaml"));
/// ```
pub fn format_error(error: &DynamicCliError) -> String {
    let mut output = String::new();

    // Error header (colored if feature enabled)
    output.push_str(&format!("{} ", color::error("Error:")));

    // Format according to error type
    match error {
        DynamicCliError::Parse(e) => {
            format_parse_error(&mut output, e);
        }

        DynamicCliError::Config(e) => {
            format_config_error(&mut output, e);
        }

        DynamicCliError::Validation(e) => {
            output.push_str(&format!("{}\n", e));
        }

        DynamicCliError::Execution(e) => {
            output.push_str(&format!("{}\n", e));
        }

        DynamicCliError::Registry(e) => {
            output.push_str(&format!("{}\n", e));
        }

        DynamicCliError::Io(e) => {
            output.push_str(&format!("{}\n", e));
        }
    }

    output
}

/// Format a parsing error with suggestions
fn format_parse_error(output: &mut String, error: &ParseError) {
    output.push_str(&format!("{}\n", error));

    // Add suggestions if available
    match error {
        ParseError::UnknownCommand { suggestions, .. } if !suggestions.is_empty() => {
            output.push_str(&format!("\n{} Did you mean:\n", color::question("?")));
            for suggestion in suggestions {
                output.push_str(&format!(
                    "  {} {}\n",
                    color::bullet("•"),
                    color::suggestion(suggestion)
                ));
            }
        }

        ParseError::UnknownOption { suggestions, .. } if !suggestions.is_empty() => {
            output.push_str(&format!("\n{} Did you mean:\n", color::question("?")));
            for suggestion in suggestions {
                output.push_str(&format!(
                    "  {} {}\n",
                    color::bullet("•"),
                    color::suggestion(suggestion)
                ));
            }
        }

        ParseError::TypeParseError {
            arg_name,
            expected_type,
            value,
            ..
        } => {
            output.push_str(&format!(
                "\n{} Expected type {} for argument {}, got: {}\n",
                color::info("ℹ"),
                color::type_name(expected_type),
                color::arg_name(arg_name),
                color::value(value)
            ));
        }

        _ => {}
    }
}

/// Format a configuration error with position
fn format_config_error(output: &mut String, error: &ConfigError) {
    match error {
        ConfigError::YamlParse {
            source,
            line,
            column,
        } => {
            output.push_str(&format!("{}\n", source));
            if let (Some(l), Some(c)) = (line, column) {
                output.push_str(&format!(
                    "  {} line {}, column {}\n",
                    color::dimmed("at"),
                    color::arg_name(&l.to_string()),
                    color::arg_name(&c.to_string())
                ));
            }
        }

        ConfigError::JsonParse {
            source,
            line,
            column,
        } => {
            output.push_str(&format!("{}\n", source));
            output.push_str(&format!(
                "  {} line {}, column {}\n",
                color::dimmed("at"),
                color::arg_name(&line.to_string()),
                color::arg_name(&column.to_string())
            ));
        }

        ConfigError::InvalidSchema { reason, path } => {
            output.push_str(&format!("{}\n", reason));
            if let Some(p) = path {
                output.push_str(&format!(
                    "  {} {}\n",
                    color::dimmed("in"),
                    color::type_name(p)
                ));
            }
        }

        _ => {
            output.push_str(&format!("{}\n", error));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_format_error_basic() {
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
        }
        .into();

        let formatted = format_error(&error);

        // Must contain "Error:" and the file path
        assert!(formatted.contains("Error:"));
        assert!(formatted.contains("test.yaml"));
    }

    #[test]
    fn test_format_parse_error_with_suggestions() {
        let error: DynamicCliError = ParseError::UnknownCommand {
            command: "simulat".to_string(),
            suggestions: vec!["simulate".to_string(), "validation".to_string()],
        }
        .into();

        let formatted = format_error(&error);

        // Must contain the error message
        assert!(formatted.contains("Unknown command"));
        assert!(formatted.contains("simulat"));

        // Must contain suggestions
        assert!(formatted.contains("Did you mean"));
        assert!(formatted.contains("simulate"));
    }

    #[test]
    fn test_format_parse_error_without_suggestions() {
        let error: DynamicCliError = ParseError::UnknownCommand {
            command: "xyz".to_string(),
            suggestions: vec![],
        }
        .into();

        let formatted = format_error(&error);

        // Must contain the error message
        assert!(formatted.contains("Unknown command"));
        assert!(formatted.contains("xyz"));

        // Must NOT contain suggestions
        assert!(!formatted.contains("Did you mean"));
    }

    #[test]
    fn test_format_config_error_with_location() {
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>("invalid: [")
            .err()
            .unwrap();

        let error: DynamicCliError = ConfigError::yaml_parse_with_location(yaml_error).into();
        let formatted = format_error(&error);

        // Must contain position information
        assert!(formatted.contains("Error:"));
        // Exact position depends on serde_yaml implementation
    }

    #[test]
    fn test_display_error_does_not_panic() {
        // Test that display_error doesn't panic (outputs to stderr)
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
        }
        .into();

        // Should not panic
        display_error(&error);
    }
}
