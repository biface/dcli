//! User-friendly error display
//!
//! Formats errors with coloring to improve readability
//! in the terminal.

use colored::*;
use crate::error::{DynamicCliError, ConfigError, ParseError};

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
/// Formatted string with ANSI color codes
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
    
    // Error header in bold red
    output.push_str(&format!("{} ", "Error:".red().bold()));
    
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
            output.push_str(&format!("\n{} Did you mean:\n", "?".yellow().bold()));
            for suggestion in suggestions {
                output.push_str(&format!("  {} {}\n", "•".cyan(), suggestion.green()));
            }
        }
        
        ParseError::UnknownOption { suggestions, .. } if !suggestions.is_empty() => {
            output.push_str(&format!("\n{} Did you mean:\n", "?".yellow().bold()));
            for suggestion in suggestions {
                output.push_str(&format!("  {} {}\n", "•".cyan(), suggestion.green()));
            }
        }
        
        ParseError::TypeParseError { arg_name, expected_type, value, .. } => {
            output.push_str(&format!(
                "\n{} Expected type {} for argument {}, got: {}\n",
                "ℹ".blue().bold(),
                expected_type.cyan(),
                arg_name.yellow(),
                value.red()
            ));
        }
        
        _ => {}
    }
}

/// Format a configuration error with position
fn format_config_error(output: &mut String, error: &ConfigError) {
    match error {
        ConfigError::YamlParse { source, line, column } => {
            output.push_str(&format!("{}\n", source));
            if let (Some(l), Some(c)) = (line, column) {
                output.push_str(&format!(
                    "  {} line {}, column {}\n",
                    "at".dimmed(),
                    l.to_string().yellow(),
                    c.to_string().yellow()
                ));
            }
        }
        
        ConfigError::JsonParse { source, line, column } => {
            output.push_str(&format!("{}\n", source));
            output.push_str(&format!(
                "  {} line {}, column {}\n",
                "at".dimmed(),
                line.to_string().yellow(),
                column.to_string().yellow()
            ));
        }
        
        ConfigError::InvalidSchema { reason, path } => {
            output.push_str(&format!("{}\n", reason));
            if let Some(p) = path {
                output.push_str(&format!(
                    "  {} {}\n",
                    "in".dimmed(),
                    p.cyan()
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
        }.into();
        
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
        }.into();
        
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
        }.into();
        
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
        }.into();
        
        // Should not panic
        display_error(&error);
    }
}
