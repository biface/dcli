//! Error types for dynamic-cli
//!
//! Defines all possible error types with context and clear messages.

use std::path::PathBuf;
use thiserror::Error;

/// Main error for the dynamic-cli framework
///
/// Encompasses all possible error categories. Uses `thiserror`
/// to automatically generate `Display` and `Error` implementations.
#[derive(Debug, Error)]
pub enum DynamicCliError {
    /// Errors related to the configuration file
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// Command parsing errors
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// Validation errors
    #[error(transparent)]
    Validation(#[from] ValidationError),

    /// Execution errors
    #[error(transparent)]
    Execution(#[from] ExecutionError),

    /// Registry errors
    #[error(transparent)]
    Registry(#[from] RegistryError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ═══════════════════════════════════════════════════════════
// CONFIGURATION ERRORS
// ═══════════════════════════════════════════════════════════

/// Errors related to loading and parsing the configuration file
///
/// These errors occur when loading the `commands.yaml` or `commands.json`
/// file and its structural validation.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Configuration file not found
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    /// use std::path::PathBuf;
    ///
    /// let error = ConfigError::FileNotFound {
    ///     path: PathBuf::from("missing.yaml"),
    /// };
    /// ```
    #[error("Configuration file not found: {path:?}")]
    FileNotFound { path: PathBuf },

    /// Unsupported file extension
    ///
    /// Only `.yaml`, `.yml` and `.json` are supported.
    #[error("Unsupported file format: '{extension}'. Supported: .yaml, .yml, .json")]
    UnsupportedFormat { extension: String },

    /// YAML parsing error
    #[error("Failed to parse YAML configuration at line {line:?}, column {column:?}: {source}")]
    YamlParse {
        #[source]
        source: serde_yaml::Error,
        /// Position in the file (if available)
        line: Option<usize>,
        column: Option<usize>,
    },

    /// JSON parsing error
    #[error("Failed to parse JSON configuration at line {line}, column {column}: {source}")]
    JsonParse {
        #[source]
        source: serde_json::Error,
        /// Position in the file
        line: usize,
        column: usize,
    },

    /// Invalid configuration schema
    ///
    /// The file structure doesn't match the expected format.
    #[error("Invalid configuration schema: {reason} (at {path:?})")]
    InvalidSchema {
        reason: String,
        /// Path in the config (e.g., "commands[0].options[2].type")
        path: Option<String>,
    },

    /// Duplicate command (same name or alias)
    #[error("Duplicate command name or alias: '{name}'")]
    DuplicateCommand { name: String },

    /// Unknown argument type
    #[error("Unknown argument type: '{type_name}' in {context}")]
    UnknownType { type_name: String, context: String },

    /// Inconsistent configuration
    ///
    /// For example, a default value that's not in the allowed choices.
    #[error("Configuration inconsistency: {details}")]
    Inconsistency { details: String },
}

// ═══════════════════════════════════════════════════════════
// PARSING ERRORS
// ═══════════════════════════════════════════════════════════

/// Errors when parsing user commands
///
/// These errors occur when analyzing arguments provided
/// by the user in CLI or REPL mode.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Unknown command
    ///
    /// The user typed a command that doesn't exist.
    /// Includes suggestions based on Levenshtein distance.
    #[error("Unknown command: '{command}'. Type 'help' for available commands.")]
    UnknownCommand {
        command: String,
        /// Similar command suggestions
        suggestions: Vec<String>,
    },

    /// Missing required positional argument
    #[error("Missing required argument: {argument} for command '{command}'")]
    MissingArgument { argument: String, command: String },

    /// Missing required option
    #[error("Missing required option: --{option} for command '{command}'")]
    MissingOption { option: String, command: String },

    /// Too many positional arguments
    #[error("Too many arguments for command '{command}'. Expected {expected}, got {got}")]
    TooManyArguments {
        command: String,
        expected: usize,
        got: usize,
    },

    /// Unknown option
    ///
    /// Includes similar option suggestions.
    #[error("Unknown option: {flag} for command '{command}'")]
    UnknownOption {
        flag: String,
        command: String,
        /// Similar option suggestions
        suggestions: Vec<String>,
    },

    /// Type parsing error
    ///
    /// The user provided a value that can't be converted
    /// to the expected type (e.g., "abc" for an integer).
    #[error("Failed to parse {arg_name} as {expected_type}: '{value}'{}", 
        .details.as_ref().map(|d| format!(" ({})", d)).unwrap_or_default())]
    TypeParseError {
        arg_name: String,
        expected_type: String,
        value: String,
        /// Error details (e.g., "not a valid integer")
        details: Option<String>,
    },

    /// Value not in allowed choices
    #[error("Invalid value for {arg_name}: '{value}'. Must be one of: {}", 
        .choices.join(", "))]
    InvalidChoice {
        arg_name: String,
        value: String,
        choices: Vec<String>,
    },

    /// Invalid command syntax
    #[error("Invalid command syntax: {details}{}", 
        .hint.as_ref().map(|h| format!("\nHint: {}", h)).unwrap_or_default())]
    InvalidSyntax {
        details: String,
        /// Example of correct syntax
        hint: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════
// VALIDATION ERRORS
// ═══════════════════════════════════════════════════════════

/// Errors during argument validation
///
/// These errors occur after parsing, during validation
/// of constraints defined in the configuration.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Required file doesn't exist
    #[error("File not found for argument '{arg_name}': {path:?}")]
    FileNotFound { path: PathBuf, arg_name: String },

    /// Invalid file extension
    #[error("Invalid file extension for {arg_name}: {path:?}. Expected: {}", 
        .expected.join(", "))]
    InvalidExtension {
        arg_name: String,
        path: PathBuf,
        expected: Vec<String>,
    },

    /// Value out of allowed range
    #[error("{arg_name} must be between {min} and {max}, got {value}")]
    OutOfRange {
        arg_name: String,
        value: f64,
        min: f64,
        max: f64,
    },

    /// Custom constraint not met
    #[error("Validation failed for {arg_name}: {reason}")]
    CustomConstraint { arg_name: String, reason: String },

    /// Dependency between arguments not satisfied
    ///
    /// Some arguments require the presence of other arguments.
    #[error("{arg_name} requires {required_arg} to be specified")]
    MissingDependency {
        arg_name: String,
        required_arg: String,
    },

    /// Mutually exclusive arguments
    ///
    /// Some arguments cannot be used together.
    #[error("Options {arg1} and {arg2} cannot be used together")]
    MutuallyExclusive { arg1: String, arg2: String },
}

// ═══════════════════════════════════════════════════════════
// EXECUTION ERRORS
// ═══════════════════════════════════════════════════════════

/// Errors during command execution
///
/// These errors occur during user code execution.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// Command handler not found
    ///
    /// The implementation name in the config doesn't match any
    /// registered handler.
    #[error("No handler registered for command '{command}' (implementation: '{implementation}')")]
    HandlerNotFound {
        command: String,
        implementation: String,
    },

    /// Error during context downcasting
    ///
    /// The handler tried to downcast the context to an incorrect type.
    #[error("Failed to downcast execution context to expected type: {expected_type}")]
    ContextDowncastFailed { expected_type: String },

    /// Invalid context state for this operation
    #[error("Invalid context state: {reason}")]
    InvalidContextState { reason: String },

    /// Error in command implementation
    ///
    /// Wraps errors from user code.
    #[error("Command execution failed: {0}")]
    CommandFailed(#[source] anyhow::Error),

    /// Command interrupted by user
    ///
    /// User pressed Ctrl+C during execution.
    #[error("Command interrupted by user")]
    Interrupted,
}

// ═══════════════════════════════════════════════════════════
// REGISTRY ERRORS
// ═══════════════════════════════════════════════════════════

/// Errors related to the command registry
///
/// These errors occur when registering commands
/// and handlers in the registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Attempt to register an already existing command
    #[error("Command '{name}' is already registered")]
    DuplicateRegistration { name: String },

    /// Alias already used by another command
    #[error("Alias '{alias}' is already used by command '{existing_command}'")]
    DuplicateAlias {
        alias: String,
        existing_command: String,
    },

    /// Missing handler for a definition
    ///
    /// A command is defined in the config but no handler
    /// has been registered for it.
    #[error("No handler provided for command '{command}'")]
    MissingHandler { command: String },
}

// ═══════════════════════════════════════════════════════════
// HELPERS FOR CREATING CONTEXTUAL ERRORS
// ═══════════════════════════════════════════════════════════

impl ParseError {
    /// Create an unknown command error with suggestions
    ///
    /// Uses Levenshtein distance to find similar commands.
    ///
    /// # Arguments
    ///
    /// * `command` - The command typed by the user
    /// * `available` - List of available commands
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let available = vec!["simulate".to_string(), "validate".to_string()];
    /// let error = ParseError::unknown_command_with_suggestions("simulat", &available);
    /// ```
    pub fn unknown_command_with_suggestions(command: &str, available: &[String]) -> Self {
        let suggestions = crate::error::find_similar_strings(command, available, 3);
        Self::UnknownCommand {
            command: command.to_string(),
            suggestions,
        }
    }

    /// Create an unknown option error with suggestions
    pub fn unknown_option_with_suggestions(
        flag: &str,
        command: &str,
        available: &[String],
    ) -> Self {
        let suggestions = crate::error::find_similar_strings(flag, available, 2);
        Self::UnknownOption {
            flag: flag.to_string(),
            command: command.to_string(),
            suggestions,
        }
    }
}

impl ConfigError {
    /// Create a YAML error with position
    ///
    /// Extracts position information from the serde_yaml error.
    pub fn yaml_parse_with_location(source: serde_yaml::Error) -> Self {
        let location = source.location();
        Self::YamlParse {
            source,
            line: location.as_ref().map(|l| l.line()),
            column: location.map(|l| l.column()),
        }
    }

    /// Create a JSON error with position
    ///
    /// Extracts position information from the serde_json error.
    pub fn json_parse_with_location(source: serde_json::Error) -> Self {
        Self::JsonParse {
            line: source.line(),
            column: source.column(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::FileNotFound {
            path: PathBuf::from("/path/to/config.yaml"),
        };
        let message = format!("{}", error);
        assert!(message.contains("not found"));
        assert!(message.contains("config.yaml"));
    }

    #[test]
    fn test_parse_error_with_suggestions() {
        let available = vec!["simulate".to_string(), "validate".to_string()];
        let error = ParseError::unknown_command_with_suggestions("simulat", &available);

        match error {
            ParseError::UnknownCommand {
                command,
                suggestions,
            } => {
                assert_eq!(command, "simulat");
                assert!(!suggestions.is_empty());
                assert!(suggestions.contains(&"simulate".to_string()));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::OutOfRange {
            arg_name: "percentage".to_string(),
            value: 150.0,
            min: 0.0,
            max: 100.0,
        };
        let message = format!("{}", error);
        assert!(message.contains("percentage"));
        assert!(message.contains("150"));
        assert!(message.contains("0"));
        assert!(message.contains("100"));
    }
}
