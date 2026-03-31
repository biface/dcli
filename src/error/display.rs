//! User-friendly error display
//!
//! Formats errors with coloring to improve readability in the terminal.
//!
//! The `colored` crate is used unconditionally, consistent with the rest of
//! the framework's output (help formatter, REPL prompt). Applications that
//! need plain-text output can disable ANSI codes at the OS level or redirect
//! stderr to a file.
//!
//! # Output format
//!
//! ```text
//! Error: <main message>
//!   ℹ  <suggestion>       ← only when a suggestion is available
//! ```
//!
//! For parse errors with Levenshtein suggestions:
//!
//! ```text
//! Error: Unknown command: 'simulat'. Type 'help' for available commands.
//!
//! ?  Did you mean:
//!   •  simulate
//!   •  simulate2
//! ```

use colored::Colorize;

use crate::error::{
    ConfigError, DynamicCliError, ExecutionError, ParseError, RegistryError, ValidationError,
};

// ═══════════════════════════════════════════════════════════
// COLOR PALETTE  (mirrors DefaultHelpFormatter)
// ═══════════════════════════════════════════════════════════

/// Render text as a bold red error label (used for "Error:")
fn color_error(s: &str) -> String {
    s.red().bold().to_string()
}

/// Render a question mark prompt (used before "Did you mean:")
fn color_question(s: &str) -> String {
    s.yellow().bold().to_string()
}

/// Render a bullet point character
fn color_bullet(s: &str) -> String {
    s.cyan().to_string()
}

/// Render a Levenshtein suggestion (command / option name)
fn color_suggestion(s: &str) -> String {
    s.green().to_string()
}

/// Render an info symbol (used before the actionable suggestion line)
fn color_info(s: &str) -> String {
    s.blue().bold().to_string()
}

/// Render a type name or path
fn color_type_name(s: &str) -> String {
    s.cyan().to_string()
}

/// Render an argument or option name
fn color_arg_name(s: &str) -> String {
    s.yellow().to_string()
}

/// Render an invalid value
fn color_value(s: &str) -> String {
    s.red().to_string()
}

/// Render dimmed secondary text (e.g., "at", "in")
fn color_dimmed(s: &str) -> String {
    s.dimmed().to_string()
}

// ═══════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════

/// Print an error to stderr in a user-friendly way
///
/// Writes the formatted error (with ANSI colors) to stderr.
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

/// Format an error as a colored, human-readable string
///
/// Generates a string suitable for display in the terminal.
/// The format is:
///
/// ```text
/// Error: <main message>
///   ℹ  <actionable suggestion>
/// ```
///
/// For parse errors with Levenshtein suggestions, a "Did you mean:" block
/// is appended instead of the `ℹ` line.
///
/// # Arguments
///
/// * `error` - The error to format
///
/// # Example
///
/// ```
/// use dynamic_cli::error::{format_error, ConfigError};
/// use std::path::PathBuf;
///
/// let error: dynamic_cli::error::DynamicCliError = ConfigError::FileNotFound {
///     path: PathBuf::from("config.yaml"),
///     suggestion: Some("Verify the path and file permissions.".to_string()),
/// }.into();
///
/// let formatted = format_error(&error);
/// assert!(formatted.contains("Error:"));
/// assert!(formatted.contains("config.yaml"));
/// ```
pub fn format_error(error: &DynamicCliError) -> String {
    let mut output = String::new();

    output.push_str(&format!("{} ", color_error("Error:")));

    match error {
        DynamicCliError::Parse(e) => format_parse_error(&mut output, e),
        DynamicCliError::Config(e) => format_config_error(&mut output, e),
        DynamicCliError::Validation(e) => format_validation_error(&mut output, e),
        DynamicCliError::Execution(e) => format_execution_error(&mut output, e),
        DynamicCliError::Registry(e) => format_registry_error(&mut output, e),
        DynamicCliError::Io(e) => output.push_str(&format!("{}\n", e)),
    }

    output
}

// ═══════════════════════════════════════════════════════════
// CATEGORY FORMATTERS
// ═══════════════════════════════════════════════════════════

/// Format a parse error, appending Levenshtein suggestions when available
fn format_parse_error(output: &mut String, error: &ParseError) {
    output.push_str(&format!("{}\n", error));

    match error {
        ParseError::UnknownCommand { suggestions, .. } if !suggestions.is_empty() => {
            output.push_str(&format!("\n{} Did you mean:\n", color_question("?")));
            for s in suggestions {
                output.push_str(&format!(
                    "  {} {}\n",
                    color_bullet("•"),
                    color_suggestion(s)
                ));
            }
        }

        ParseError::UnknownOption { suggestions, .. } if !suggestions.is_empty() => {
            output.push_str(&format!("\n{} Did you mean:\n", color_question("?")));
            for s in suggestions {
                output.push_str(&format!(
                    "  {} {}\n",
                    color_bullet("•"),
                    color_suggestion(s)
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
                color_info("ℹ"),
                color_type_name(expected_type),
                color_arg_name(arg_name),
                color_value(value)
            ));
        }

        ParseError::MissingArgument { suggestion, .. }
        | ParseError::MissingOption { suggestion, .. }
        | ParseError::TooManyArguments { suggestion, .. } => {
            append_suggestion(output, suggestion.as_deref());
        }

        _ => {}
    }
}

/// Format a configuration error, showing parse positions and suggestions
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
                    color_dimmed("at"),
                    color_arg_name(&l.to_string()),
                    color_arg_name(&c.to_string())
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
                color_dimmed("at"),
                color_arg_name(&line.to_string()),
                color_arg_name(&column.to_string())
            ));
        }

        ConfigError::InvalidSchema {
            reason,
            path,
            suggestion,
        } => {
            output.push_str(&format!("{}\n", reason));
            if let Some(p) = path {
                output.push_str(&format!(
                    "  {} {}\n",
                    color_dimmed("in"),
                    color_type_name(p)
                ));
            }
            append_suggestion(output, suggestion.as_deref());
        }

        ConfigError::FileNotFound { suggestion, .. }
        | ConfigError::UnsupportedFormat { suggestion, .. }
        | ConfigError::DuplicateCommand { suggestion, .. }
        | ConfigError::UnknownType { suggestion, .. }
        | ConfigError::Inconsistency { suggestion, .. } => {
            output.push_str(&format!("{}\n", error));
            append_suggestion(output, suggestion.as_deref());
        }
    }
}

/// Format a validation error with its actionable suggestion
fn format_validation_error(output: &mut String, error: &ValidationError) {
    output.push_str(&format!("{}\n", error));

    let suggestion = match error {
        ValidationError::FileNotFound { suggestion, .. } => suggestion.as_deref(),
        ValidationError::OutOfRange { suggestion, .. } => suggestion.as_deref(),
        ValidationError::CustomConstraint { suggestion, .. } => suggestion.as_deref(),
        ValidationError::MissingDependency { suggestion, .. } => suggestion.as_deref(),
        ValidationError::MutuallyExclusive { suggestion, .. } => suggestion.as_deref(),
        // InvalidExtension already lists the expected extensions in the message
        ValidationError::InvalidExtension { .. } => None,
    };

    append_suggestion(output, suggestion);
}

/// Format an execution error with its actionable suggestion
fn format_execution_error(output: &mut String, error: &ExecutionError) {
    output.push_str(&format!("{}\n", error));

    let suggestion = match error {
        ExecutionError::HandlerNotFound { suggestion, .. } => suggestion.as_deref(),
        ExecutionError::ContextDowncastFailed { suggestion, .. } => suggestion.as_deref(),
        ExecutionError::InvalidContextState { suggestion, .. } => suggestion.as_deref(),
        // CommandFailed and Interrupted carry no structured suggestion
        ExecutionError::CommandFailed(_) | ExecutionError::Interrupted => None,
    };

    append_suggestion(output, suggestion);
}

/// Format a registry error with its actionable suggestion
fn format_registry_error(output: &mut String, error: &RegistryError) {
    output.push_str(&format!("{}\n", error));

    let suggestion = match error {
        RegistryError::DuplicateRegistration { suggestion, .. } => suggestion.as_deref(),
        RegistryError::DuplicateAlias { suggestion, .. } => suggestion.as_deref(),
        RegistryError::MissingHandler { suggestion, .. } => suggestion.as_deref(),
    };

    append_suggestion(output, suggestion);
}

// ═══════════════════════════════════════════════════════════
// SHARED HELPER
// ═══════════════════════════════════════════════════════════

/// Append a suggestion line to the output buffer
///
/// Renders the line only when `suggestion` is `Some`. The format is:
///
/// ```text
///   ℹ  <suggestion text>
/// ```
///
/// When `suggestion` is `None`, this is a no-op.
fn append_suggestion(output: &mut String, suggestion: Option<&str>) {
    if let Some(s) = suggestion {
        output.push_str(&format!("  {} {}\n", color_info("ℹ"), s));
    }
}

// ═══════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── format_error — Config ────────────────────────────────

    #[test]
    fn test_format_config_file_not_found_contains_path() {
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
            suggestion: None,
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("Error:"));
        assert!(formatted.contains("test.yaml"));
    }

    #[test]
    fn test_format_config_file_not_found_with_suggestion() {
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
            suggestion: Some("Verify the path.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("Verify the path."));
    }

    #[test]
    fn test_format_config_file_not_found_no_suggestion_no_hint_line() {
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
            suggestion: None,
        }
        .into();

        let formatted = format_error(&error);
        // The ℹ line must not appear when suggestion is None
        assert!(!formatted.contains('ℹ'));
    }

    #[test]
    fn test_format_config_unsupported_format_with_suggestion() {
        let error: DynamicCliError = ConfigError::UnsupportedFormat {
            extension: ".toml".to_string(),
            suggestion: Some("Use .yaml instead.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains(".toml"));
        assert!(formatted.contains("Use .yaml instead."));
    }

    #[test]
    fn test_format_config_yaml_parse_contains_location() {
        let yaml_error = serde_yaml::from_str::<serde_yaml::Value>("invalid: [")
            .err()
            .unwrap();

        let error: DynamicCliError = ConfigError::yaml_parse_with_location(yaml_error).into();
        let formatted = format_error(&error);
        assert!(formatted.contains("Error:"));
    }

    #[test]
    fn test_format_config_invalid_schema_with_path_and_suggestion() {
        let error: DynamicCliError = ConfigError::InvalidSchema {
            reason: "missing field".to_string(),
            path: Some("commands[0]".to_string()),
            suggestion: Some("Add a name field.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("missing field"));
        assert!(formatted.contains("commands[0]"));
        assert!(formatted.contains("Add a name field."));
    }

    // ── format_error — Parse ─────────────────────────────────

    #[test]
    fn test_format_parse_unknown_command_with_suggestions() {
        let error: DynamicCliError = ParseError::UnknownCommand {
            command: "simulat".to_string(),
            suggestions: vec!["simulate".to_string(), "validation".to_string()],
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("Unknown command"));
        assert!(formatted.contains("simulat"));
        assert!(formatted.contains("Did you mean"));
        assert!(formatted.contains("simulate"));
    }

    #[test]
    fn test_format_parse_unknown_command_no_suggestions() {
        let error: DynamicCliError = ParseError::UnknownCommand {
            command: "xyz".to_string(),
            suggestions: vec![],
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("xyz"));
        assert!(!formatted.contains("Did you mean"));
    }

    #[test]
    fn test_format_parse_missing_argument_with_suggestion() {
        let error: DynamicCliError = ParseError::MissingArgument {
            argument: "file".to_string(),
            command: "process".to_string(),
            suggestion: Some("Run --help process to see required arguments.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("file"));
        assert!(formatted.contains("Run --help process"));
    }

    #[test]
    fn test_format_parse_missing_option_with_suggestion() {
        let error: DynamicCliError = ParseError::MissingOption {
            option: "output".to_string(),
            command: "export".to_string(),
            suggestion: Some("Run --help export to see required options.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("output"));
        assert!(formatted.contains("Run --help export"));
    }

    #[test]
    fn test_format_parse_too_many_arguments_with_suggestion() {
        let error: DynamicCliError = ParseError::TooManyArguments {
            command: "run".to_string(),
            expected: 1,
            got: 3,
            suggestion: Some("Run --help run for the expected usage.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("run"));
        assert!(formatted.contains("Run --help run"));
    }

    #[test]
    fn test_format_parse_type_parse_error_shows_info_block() {
        let error: DynamicCliError = ParseError::TypeParseError {
            arg_name: "count".to_string(),
            expected_type: "integer".to_string(),
            value: "abc".to_string(),
            details: None,
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("integer"));
        assert!(formatted.contains("count"));
        assert!(formatted.contains("abc"));
    }

    // ── format_error — Validation ────────────────────────────

    #[test]
    fn test_format_validation_file_not_found_with_suggestion() {
        let error: DynamicCliError = ValidationError::FileNotFound {
            path: PathBuf::from("data.csv"),
            arg_name: "input".to_string(),
            suggestion: Some("Check that the file exists.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("data.csv"));
        assert!(formatted.contains("Check that the file exists."));
    }

    #[test]
    fn test_format_validation_out_of_range_with_suggestion() {
        let error: DynamicCliError = ValidationError::OutOfRange {
            arg_name: "percentage".to_string(),
            value: 150.0,
            min: 0.0,
            max: 100.0,
            suggestion: Some("Value must be between 0 and 100.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("percentage"));
        assert!(formatted.contains("Value must be between 0 and 100."));
    }

    #[test]
    fn test_format_validation_mutually_exclusive_with_suggestion() {
        let error: DynamicCliError = ValidationError::MutuallyExclusive {
            arg1: "--verbose".to_string(),
            arg2: "--quiet".to_string(),
            suggestion: Some("Remove one of the two conflicting options.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("--verbose"));
        assert!(formatted.contains("Remove one of the two conflicting options."));
    }

    #[test]
    fn test_format_validation_missing_dependency_with_suggestion() {
        let error: DynamicCliError = ValidationError::MissingDependency {
            arg_name: "format".to_string(),
            required_arg: "output".to_string(),
            suggestion: Some("Add --output to your command.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("format"));
        assert!(formatted.contains("Add --output to your command."));
    }

    #[test]
    fn test_format_validation_invalid_extension_no_suggestion_line() {
        // InvalidExtension has no suggestion field; the message itself lists extensions
        let error: DynamicCliError = ValidationError::InvalidExtension {
            arg_name: "input".to_string(),
            path: PathBuf::from("data.png"),
            expected: vec![".csv".to_string(), ".tsv".to_string()],
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("data.png"));
        assert!(!formatted.contains('ℹ'));
    }

    // ── format_error — Execution ─────────────────────────────

    #[test]
    fn test_format_execution_handler_not_found_with_suggestion() {
        let error: DynamicCliError = ExecutionError::HandlerNotFound {
            command: "run".to_string(),
            implementation: "run_handler".to_string(),
            suggestion: Some(
                "Ensure .register_handler(\"run_handler\", ...) was called.".to_string(),
            ),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("run"));
        assert!(formatted.contains("run_handler"));
        assert!(formatted.contains("register_handler"));
    }

    #[test]
    fn test_format_execution_context_downcast_failed_with_suggestion() {
        let error: DynamicCliError = ExecutionError::ContextDowncastFailed {
            expected_type: "MyCtx".to_string(),
            suggestion: Some("Check the context type.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("MyCtx"));
        assert!(formatted.contains("Check the context type."));
    }

    #[test]
    fn test_format_execution_interrupted_no_suggestion() {
        let error: DynamicCliError = ExecutionError::Interrupted.into();
        let formatted = format_error(&error);
        assert!(formatted.contains("interrupted"));
        assert!(!formatted.contains('ℹ'));
    }

    // ── format_error — Registry ──────────────────────────────

    #[test]
    fn test_format_registry_missing_handler_with_suggestion() {
        let error: DynamicCliError = RegistryError::MissingHandler {
            command: "export".to_string(),
            suggestion: Some("Call .register_handler(\"export\", ...) before running.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("export"));
        assert!(formatted.contains("register_handler"));
    }

    #[test]
    fn test_format_registry_duplicate_registration_with_suggestion() {
        let error: DynamicCliError = RegistryError::DuplicateRegistration {
            name: "run".to_string(),
            suggestion: Some("Command names must be unique.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("run"));
        assert!(formatted.contains("Command names must be unique."));
    }

    #[test]
    fn test_format_registry_duplicate_alias_with_suggestion() {
        let error: DynamicCliError = RegistryError::DuplicateAlias {
            alias: "r".to_string(),
            existing_command: "run".to_string(),
            suggestion: Some("Choose a different alias.".to_string()),
        }
        .into();

        let formatted = format_error(&error);
        assert!(formatted.contains("run"));
        assert!(formatted.contains("Choose a different alias."));
    }

    // ── display_error ────────────────────────────────────────

    #[test]
    fn test_display_error_does_not_panic() {
        let error: DynamicCliError = ConfigError::FileNotFound {
            path: PathBuf::from("test.yaml"),
            suggestion: None,
        }
        .into();
        // Writes to stderr — must not panic
        display_error(&error);
    }
}
