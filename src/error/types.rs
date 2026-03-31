//! Error types for dynamic-cli
//!
//! Defines all possible error types with context and clear messages.
//!
//! Each error variant carries an optional `suggestion` field that surfaces
//! an actionable hint to the end user. Suggestions are rendered by
//! [`crate::error::display::format_error`] and are never part of the
//! `Display` string itself, keeping machine-readable messages stable.

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
    ///     suggestion: Some("Verify the path and file permissions.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("missing.yaml"));
    /// ```
    #[error("Configuration file not found: {path:?}")]
    FileNotFound {
        path: PathBuf,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Unsupported file extension
    ///
    /// Only `.yaml`, `.yml` and `.json` are supported.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::UnsupportedFormat {
    ///     extension: ".toml".to_string(),
    ///     suggestion: Some("Rename the file with a .yaml, .yml or .json extension.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains(".toml"));
    /// ```
    #[error("Unsupported file format: '{extension}'. Supported: .yaml, .yml, .json")]
    UnsupportedFormat {
        extension: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

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
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::InvalidSchema {
    ///     reason: "Missing required field 'name'".to_string(),
    ///     path: Some("commands[0]".to_string()),
    ///     suggestion: Some("Add a 'name' field to each command entry.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("Missing required field"));
    /// ```
    #[error("Invalid configuration schema: {reason} (at {path:?})")]
    InvalidSchema {
        reason: String,
        /// Path in the config (e.g., "commands[0].options[2].type")
        path: Option<String>,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Duplicate command (same name or alias)
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::DuplicateCommand {
    ///     name: "run".to_string(),
    ///     suggestion: Some("Rename one of the conflicting commands or aliases.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("run"));
    /// ```
    #[error("Duplicate command name or alias: '{name}'")]
    DuplicateCommand {
        name: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Unknown argument type
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::UnknownType {
    ///     type_name: "datetime".to_string(),
    ///     context: "commands[1].options[0]".to_string(),
    ///     suggestion: Some(
    ///         "Supported types: string, integer, float, boolean, file.".to_string()
    ///     ),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("datetime"));
    /// ```
    #[error("Unknown argument type: '{type_name}' in {context}")]
    UnknownType {
        type_name: String,
        context: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Inconsistent configuration
    ///
    /// For example, a default value that's not in the allowed choices.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::Inconsistency {
    ///     details: "Default value 'fast' is not in choices: slow, medium".to_string(),
    ///     suggestion: Some("Ensure the default value matches one of the allowed choices.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("Default value"));
    /// ```
    #[error("Configuration inconsistency: {details}")]
    Inconsistency {
        details: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },
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
        /// Similar command suggestions (from Levenshtein distance)
        suggestions: Vec<String>,
    },

    /// Missing required positional argument
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::MissingArgument {
    ///     argument: "filename".to_string(),
    ///     command: "process".to_string(),
    ///     suggestion: Some("Run --help process to see required arguments.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("filename"));
    /// ```
    #[error("Missing required argument: {argument} for command '{command}'")]
    MissingArgument {
        argument: String,
        command: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Missing required option
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::MissingOption {
    ///     option: "output".to_string(),
    ///     command: "export".to_string(),
    ///     suggestion: Some("Run --help export to see required options.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("output"));
    /// ```
    #[error("Missing required option: --{option} for command '{command}'")]
    MissingOption {
        option: String,
        command: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Too many positional arguments
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::TooManyArguments {
    ///     command: "run".to_string(),
    ///     expected: 1,
    ///     got: 3,
    ///     suggestion: Some("Run --help run for the expected usage.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("run"));
    /// ```
    #[error("Too many arguments for command '{command}'. Expected {expected}, got {got}")]
    TooManyArguments {
        command: String,
        expected: usize,
        got: usize,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Unknown option
    ///
    /// Includes similar option suggestions.
    #[error("Unknown option: {flag} for command '{command}'")]
    UnknownOption {
        flag: String,
        command: String,
        /// Similar option suggestions (from Levenshtein distance)
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
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ValidationError;
    /// use std::path::PathBuf;
    ///
    /// let error = ValidationError::FileNotFound {
    ///     path: PathBuf::from("data.csv"),
    ///     arg_name: "input".to_string(),
    ///     suggestion: Some("Check that the file exists and the path is correct.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("data.csv"));
    /// ```
    #[error("File not found for argument '{arg_name}': {path:?}")]
    FileNotFound {
        path: PathBuf,
        arg_name: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Invalid file extension
    #[error("Invalid file extension for {arg_name}: {path:?}. Expected: {}", 
        .expected.join(", "))]
    InvalidExtension {
        arg_name: String,
        path: PathBuf,
        expected: Vec<String>,
    },

    /// Value out of allowed range
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ValidationError;
    ///
    /// let error = ValidationError::OutOfRange {
    ///     arg_name: "percentage".to_string(),
    ///     value: 150.0,
    ///     min: 0.0,
    ///     max: 100.0,
    ///     suggestion: Some("Value must be between 0 and 100.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("150"));
    /// ```
    #[error("{arg_name} must be between {min} and {max}, got {value}")]
    OutOfRange {
        arg_name: String,
        value: f64,
        min: f64,
        max: f64,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Custom constraint not met
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ValidationError;
    ///
    /// let error = ValidationError::CustomConstraint {
    ///     arg_name: "email".to_string(),
    ///     reason: "not a valid email address".to_string(),
    ///     suggestion: Some("Provide a valid email address (e.g. user@example.com).".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("email"));
    /// ```
    #[error("Validation failed for {arg_name}: {reason}")]
    CustomConstraint {
        arg_name: String,
        reason: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Dependency between arguments not satisfied
    ///
    /// Some arguments require the presence of other arguments.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ValidationError;
    ///
    /// let error = ValidationError::MissingDependency {
    ///     arg_name: "output-format".to_string(),
    ///     required_arg: "output".to_string(),
    ///     suggestion: Some("Add --output to your command.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("output-format"));
    /// ```
    #[error("{arg_name} requires {required_arg} to be specified")]
    MissingDependency {
        arg_name: String,
        required_arg: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Mutually exclusive arguments
    ///
    /// Some arguments cannot be used together.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ValidationError;
    ///
    /// let error = ValidationError::MutuallyExclusive {
    ///     arg1: "--verbose".to_string(),
    ///     arg2: "--quiet".to_string(),
    ///     suggestion: Some("Remove one of the two conflicting options.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("--verbose"));
    /// ```
    #[error("Options {arg1} and {arg2} cannot be used together")]
    MutuallyExclusive {
        arg1: String,
        arg2: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },
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
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ExecutionError;
    ///
    /// let error = ExecutionError::HandlerNotFound {
    ///     command: "run".to_string(),
    ///     implementation: "run_handler".to_string(),
    ///     suggestion: Some(
    ///         "Ensure .register_handler(\"run_handler\", ...) was called before running."
    ///             .to_string()
    ///     ),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("run"));
    /// ```
    #[error("No handler registered for command '{command}' (implementation: '{implementation}')")]
    HandlerNotFound {
        command: String,
        implementation: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Error during context downcasting
    ///
    /// The handler tried to downcast the context to an incorrect type.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ExecutionError;
    ///
    /// let error = ExecutionError::ContextDowncastFailed {
    ///     expected_type: "MyAppContext".to_string(),
    ///     suggestion: Some(
    ///         "Check that the context type passed to the handler matches the expected type."
    ///             .to_string()
    ///     ),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("MyAppContext"));
    /// ```
    #[error("Failed to downcast execution context to expected type: {expected_type}")]
    ContextDowncastFailed {
        expected_type: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Invalid context state for this operation
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ExecutionError;
    ///
    /// let error = ExecutionError::InvalidContextState {
    ///     reason: "connection pool not initialised".to_string(),
    ///     suggestion: Some("Ensure the context is fully initialised before running commands.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("connection pool"));
    /// ```
    #[error("Invalid context state: {reason}")]
    InvalidContextState {
        reason: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

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
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::RegistryError;
    ///
    /// let error = RegistryError::DuplicateRegistration {
    ///     name: "run".to_string(),
    ///     suggestion: Some("Command names must be unique across the registry.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("run"));
    /// ```
    #[error("Command '{name}' is already registered")]
    DuplicateRegistration {
        name: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Alias already used by another command
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::RegistryError;
    ///
    /// let error = RegistryError::DuplicateAlias {
    ///     alias: "r".to_string(),
    ///     existing_command: "run".to_string(),
    ///     suggestion: Some("Choose a different alias for one of the commands.".to_string()),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("run"));
    /// ```
    #[error("Alias '{alias}' is already used by command '{existing_command}'")]
    DuplicateAlias {
        alias: String,
        existing_command: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },

    /// Missing handler for a definition
    ///
    /// A command is defined in the config but no handler
    /// has been registered for it.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::RegistryError;
    ///
    /// let error = RegistryError::MissingHandler {
    ///     command: "export".to_string(),
    ///     suggestion: Some(
    ///         "Call .register_handler(\"export\", ...) before running.".to_string()
    ///     ),
    /// };
    /// let msg = format!("{}", error);
    /// assert!(msg.contains("export"));
    /// ```
    #[error("No handler provided for command '{command}'")]
    MissingHandler {
        command: String,
        /// Actionable hint surfaced to the user (not part of the Display string)
        suggestion: Option<String>,
    },
}

// ═══════════════════════════════════════════════════════════
// HELPERS FOR CREATING CONTEXTUAL ERRORS
// ═══════════════════════════════════════════════════════════

impl ParseError {
    /// Create an unknown command error with Levenshtein suggestions
    ///
    /// Automatically computes similar command names from the available list.
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
    /// match error {
    ///     ParseError::UnknownCommand { suggestions, .. } => {
    ///         assert!(suggestions.contains(&"simulate".to_string()));
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn unknown_command_with_suggestions(command: &str, available: &[String]) -> Self {
        let suggestions = crate::error::find_similar_strings(command, available, 3);
        Self::UnknownCommand {
            command: command.to_string(),
            suggestions,
        }
    }

    /// Create an unknown option error with Levenshtein suggestions
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let available = vec!["--verbose".to_string(), "--output".to_string()];
    /// let error = ParseError::unknown_option_with_suggestions("--verbos", "run", &available);
    /// match error {
    ///     ParseError::UnknownOption { suggestions, .. } => {
    ///         assert!(suggestions.contains(&"--verbose".to_string()));
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
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

    /// Create a missing argument error with a help hint
    ///
    /// The suggestion automatically refers the user to `--help <command>`.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::missing_argument("filename", "process");
    /// match error {
    ///     ParseError::MissingArgument { suggestion, .. } => {
    ///         assert!(suggestion.is_some());
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn missing_argument(argument: &str, command: &str) -> Self {
        Self::MissingArgument {
            argument: argument.to_string(),
            command: command.to_string(),
            suggestion: Some(format!("Run --help {command} to see required arguments.")),
        }
    }

    /// Create a missing option error with a help hint
    ///
    /// The suggestion automatically refers the user to `--help <command>`.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::missing_option("output", "export");
    /// match error {
    ///     ParseError::MissingOption { suggestion, .. } => {
    ///         assert!(suggestion.is_some());
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn missing_option(option: &str, command: &str) -> Self {
        Self::MissingOption {
            option: option.to_string(),
            command: command.to_string(),
            suggestion: Some(format!("Run --help {command} to see required options.")),
        }
    }

    /// Create a too-many-arguments error with a help hint
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ParseError;
    ///
    /// let error = ParseError::too_many_arguments("run", 1, 3);
    /// match error {
    ///     ParseError::TooManyArguments { suggestion, .. } => {
    ///         assert!(suggestion.is_some());
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn too_many_arguments(command: &str, expected: usize, got: usize) -> Self {
        Self::TooManyArguments {
            command: command.to_string(),
            expected,
            got,
            suggestion: Some(format!("Run --help {command} for the expected usage.")),
        }
    }
}

impl ConfigError {
    /// Create a file-not-found error with a standard suggestion
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    /// use std::path::PathBuf;
    ///
    /// let error = ConfigError::file_not_found(PathBuf::from("commands.yaml"));
    /// match error {
    ///     ConfigError::FileNotFound { suggestion, .. } => {
    ///         assert!(suggestion.is_some());
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound {
            path,
            suggestion: Some("Verify the path and file permissions.".to_string()),
        }
    }

    /// Create an unsupported-format error with a standard suggestion
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ConfigError;
    ///
    /// let error = ConfigError::unsupported_format(".toml");
    /// match error {
    ///     ConfigError::UnsupportedFormat { suggestion, .. } => {
    ///         assert!(suggestion.is_some());
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn unsupported_format(extension: &str) -> Self {
        Self::UnsupportedFormat {
            extension: extension.to_string(),
            suggestion: Some("Rename the file with a .yaml, .yml or .json extension.".to_string()),
        }
    }

    /// Create a YAML parse error with position extracted from the serde error
    pub fn yaml_parse_with_location(source: serde_yaml::Error) -> Self {
        let location = source.location();
        Self::YamlParse {
            source,
            line: location.as_ref().map(|l| l.line()),
            column: location.map(|l| l.column()),
        }
    }

    /// Create a JSON parse error with position extracted from the serde error
    pub fn json_parse_with_location(source: serde_json::Error) -> Self {
        Self::JsonParse {
            line: source.line(),
            column: source.column(),
            source,
        }
    }
}

impl ExecutionError {
    /// Create a handler-not-found error with an actionable suggestion
    ///
    /// The suggestion interpolates the implementation name so the user
    /// knows exactly which `.register_handler()` call is missing.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::ExecutionError;
    ///
    /// let error = ExecutionError::handler_not_found("run", "run_handler");
    /// match error {
    ///     ExecutionError::HandlerNotFound { suggestion, .. } => {
    ///         assert!(suggestion.as_deref().unwrap_or("").contains("run_handler"));
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn handler_not_found(command: &str, implementation: &str) -> Self {
        Self::HandlerNotFound {
            command: command.to_string(),
            implementation: implementation.to_string(),
            suggestion: Some(format!(
                "Ensure .register_handler(\"{implementation}\", ...) was called before running."
            )),
        }
    }
}

impl RegistryError {
    /// Create a missing-handler error with an actionable suggestion
    ///
    /// The suggestion interpolates the command name so the user
    /// knows exactly which `.register_handler()` call is missing.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::error::RegistryError;
    ///
    /// let error = RegistryError::missing_handler("export");
    /// match error {
    ///     RegistryError::MissingHandler { suggestion, .. } => {
    ///         assert!(suggestion.as_deref().unwrap_or("").contains("export"));
    ///     }
    ///     _ => panic!("wrong variant"),
    /// }
    /// ```
    pub fn missing_handler(command: &str) -> Self {
        Self::MissingHandler {
            command: command.to_string(),
            suggestion: Some(format!(
                "Call .register_handler(\"{command}\", ...) before running."
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ConfigError ──────────────────────────────────────────

    #[test]
    fn test_config_file_not_found_display() {
        let error = ConfigError::FileNotFound {
            path: PathBuf::from("/path/to/config.yaml"),
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("not found"));
        assert!(msg.contains("config.yaml"));
    }

    #[test]
    fn test_config_file_not_found_helper_has_suggestion() {
        let error = ConfigError::file_not_found(PathBuf::from("commands.yaml"));
        match error {
            ConfigError::FileNotFound { suggestion, .. } => {
                assert!(suggestion.is_some(), "helper must populate suggestion");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_config_unsupported_format_helper_has_suggestion() {
        let error = ConfigError::unsupported_format(".toml");
        match error {
            ConfigError::UnsupportedFormat {
                suggestion,
                extension,
                ..
            } => {
                assert_eq!(extension, ".toml");
                assert!(suggestion.is_some());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_config_duplicate_command_display() {
        let error = ConfigError::DuplicateCommand {
            name: "run".to_string(),
            suggestion: Some("Rename one of the conflicting commands.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("run"));
        // suggestion must NOT appear in Display (it's rendered separately)
        assert!(!msg.contains("Rename"));
    }

    #[test]
    fn test_config_unknown_type_display() {
        let error = ConfigError::UnknownType {
            type_name: "datetime".to_string(),
            context: "commands[0]".to_string(),
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("datetime"));
    }

    #[test]
    fn test_config_inconsistency_display() {
        let error = ConfigError::Inconsistency {
            details: "default not in choices".to_string(),
            suggestion: Some("hint".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("default not in choices"));
        assert!(!msg.contains("hint")); // suggestion separate from Display
    }

    #[test]
    fn test_config_invalid_schema_display() {
        let error = ConfigError::InvalidSchema {
            reason: "missing field".to_string(),
            path: Some("commands[0]".to_string()),
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("missing field"));
    }

    // ── ParseError ───────────────────────────────────────────

    #[test]
    fn test_parse_unknown_command_with_suggestions() {
        let available = vec!["simulate".to_string(), "validate".to_string()];
        let error = ParseError::unknown_command_with_suggestions("simulat", &available);
        match error {
            ParseError::UnknownCommand {
                command,
                suggestions,
            } => {
                assert_eq!(command, "simulat");
                assert!(suggestions.contains(&"simulate".to_string()));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_missing_argument_helper_has_suggestion() {
        let error = ParseError::missing_argument("filename", "process");
        match error {
            ParseError::MissingArgument {
                suggestion,
                command,
                ..
            } => {
                assert_eq!(command, "process");
                let s = suggestion.unwrap();
                assert!(s.contains("process"));
                assert!(s.contains("--help"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_missing_option_helper_has_suggestion() {
        let error = ParseError::missing_option("output", "export");
        match error {
            ParseError::MissingOption {
                suggestion, option, ..
            } => {
                assert_eq!(option, "output");
                let s = suggestion.unwrap();
                assert!(s.contains("export"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_too_many_arguments_helper_has_suggestion() {
        let error = ParseError::too_many_arguments("run", 1, 3);
        match error {
            ParseError::TooManyArguments {
                suggestion,
                expected,
                got,
                ..
            } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 3);
                assert!(suggestion.is_some());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_parse_missing_argument_suggestion_none_by_default() {
        // Direct construction without helper: suggestion is caller's responsibility
        let error = ParseError::MissingArgument {
            argument: "file".to_string(),
            command: "run".to_string(),
            suggestion: None,
        };
        match error {
            ParseError::MissingArgument { suggestion, .. } => assert!(suggestion.is_none()),
            _ => panic!("wrong variant"),
        }
    }

    // ── ValidationError ──────────────────────────────────────

    #[test]
    fn test_validation_out_of_range_display() {
        let error = ValidationError::OutOfRange {
            arg_name: "percentage".to_string(),
            value: 150.0,
            min: 0.0,
            max: 100.0,
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("percentage"));
        assert!(msg.contains("150"));
        assert!(msg.contains("0"));
        assert!(msg.contains("100"));
    }

    #[test]
    fn test_validation_out_of_range_suggestion_not_in_display() {
        let error = ValidationError::OutOfRange {
            arg_name: "percentage".to_string(),
            value: 150.0,
            min: 0.0,
            max: 100.0,
            suggestion: Some("Value must be between 0 and 100.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(!msg.contains("Value must be between")); // suggestion is separate
    }

    #[test]
    fn test_validation_file_not_found_suggestion() {
        let error = ValidationError::FileNotFound {
            path: PathBuf::from("data.csv"),
            arg_name: "input".to_string(),
            suggestion: Some("Check that the file exists.".to_string()),
        };
        match error {
            ValidationError::FileNotFound { suggestion, .. } => {
                assert!(suggestion.is_some());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_validation_missing_dependency_suggestion() {
        let error = ValidationError::MissingDependency {
            arg_name: "format".to_string(),
            required_arg: "output".to_string(),
            suggestion: Some("Add --output to your command.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("format"));
        assert!(msg.contains("output"));
    }

    #[test]
    fn test_validation_mutually_exclusive_suggestion() {
        let error = ValidationError::MutuallyExclusive {
            arg1: "--verbose".to_string(),
            arg2: "--quiet".to_string(),
            suggestion: Some("Remove one of the two conflicting options.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("--verbose"));
        assert!(msg.contains("--quiet"));
    }

    // ── ExecutionError ───────────────────────────────────────

    #[test]
    fn test_execution_handler_not_found_helper_interpolates_impl() {
        let error = ExecutionError::handler_not_found("run", "run_handler");
        match error {
            ExecutionError::HandlerNotFound {
                suggestion,
                implementation,
                ..
            } => {
                assert_eq!(implementation, "run_handler");
                let s = suggestion.unwrap();
                assert!(s.contains("run_handler"));
                assert!(s.contains("register_handler"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_execution_context_downcast_failed_display() {
        let error = ExecutionError::ContextDowncastFailed {
            expected_type: "MyAppContext".to_string(),
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("MyAppContext"));
    }

    #[test]
    fn test_execution_invalid_context_state_suggestion() {
        let error = ExecutionError::InvalidContextState {
            reason: "pool not ready".to_string(),
            suggestion: Some("Ensure context is initialised.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("pool not ready"));
    }

    // ── RegistryError ────────────────────────────────────────

    #[test]
    fn test_registry_missing_handler_helper_interpolates_command() {
        let error = RegistryError::missing_handler("export");
        match error {
            RegistryError::MissingHandler {
                suggestion,
                command,
            } => {
                assert_eq!(command, "export");
                let s = suggestion.unwrap();
                assert!(s.contains("export"));
                assert!(s.contains("register_handler"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_registry_duplicate_registration_display() {
        let error = RegistryError::DuplicateRegistration {
            name: "run".to_string(),
            suggestion: None,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("run"));
    }

    #[test]
    fn test_registry_duplicate_alias_display() {
        let error = RegistryError::DuplicateAlias {
            alias: "r".to_string(),
            existing_command: "run".to_string(),
            suggestion: Some("Choose a different alias.".to_string()),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("run"));
        assert!(!msg.contains("Choose")); // suggestion separate from Display
    }
}
