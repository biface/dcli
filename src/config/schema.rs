//! Configuration schema definitions
//!
//! This module defines all data structures for representing
//! CLI/REPL configurations loaded from YAML or JSON files.
//!
//! # Main Components
//!
//! - [`CommandsConfig`]: Root configuration structure
//! - [`CommandDefinition`]: Individual command specification
//! - [`ArgumentType`]: Supported argument types
//! - [`ValidationRule`]: Validation constraints

use serde::{Deserialize, Serialize};

/// Complete configuration for CLI/REPL commands
///
/// This is the root structure deserialized from YAML/JSON files.
/// It contains metadata about the interface and all command definitions.
///
/// # Example YAML
///
/// ```yaml
/// metadata:
///   version: "1.0.0"
///   prompt: "myapp"
///   prompt_suffix: " > "
/// commands:
///   - name: hello
///     description: "Say hello"
///     # ... more fields
/// global_options: []
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CommandsConfig {
    /// Metadata about the application interface
    pub metadata: Metadata,

    /// List of all available commands
    pub commands: Vec<CommandDefinition>,

    /// Global options available to all commands
    #[serde(default)]
    pub global_options: Vec<OptionDefinition>,
}

/// Metadata for the CLI/REPL interface
///
/// Contains information about the application version
/// and prompt customization for REPL mode.
///
/// # Fields
///
/// - `version`: Application version string
/// - `prompt`: Command prompt prefix (e.g., "myapp")
/// - `prompt_suffix`: Suffix after prompt (e.g., " > ")
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Metadata {
    /// Application version (e.g., "1.0.0")
    pub version: String,

    /// Prompt prefix displayed in REPL mode
    ///
    /// Example: "chrom-rs" will display as "chrom-rs > "
    pub prompt: String,

    /// Prompt suffix (typically " > " or ": ")
    #[serde(default = "default_prompt_suffix")]
    pub prompt_suffix: String,
}

/// Default prompt suffix
fn default_prompt_suffix() -> String {
    " > ".to_string()
}

/// Definition of a single command
///
/// Describes a command with its arguments, options, and validation rules.
/// Each command must have a corresponding handler implementation.
///
/// # Example
///
/// ```yaml
/// name: simulate
/// aliases: [sim, run]
/// description: "Run a simulation"
/// required: true
/// arguments:
///   - name: input_file
///     arg_type: path
///     required: true
///     description: "Input configuration file"
///     validation:
///       - must_exist: true
///       - extensions: [yaml, json]
/// options: []
/// implementation: "simulate_handler"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CommandDefinition {
    /// Command name (used for invocation)
    pub name: String,

    /// Alternative names for the command
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Human-readable description for help text
    pub description: String,

    /// Whether this command is required to be implemented
    ///
    /// If true, the application will fail to start if no handler is registered.
    #[serde(default)]
    pub required: bool,

    /// Positional arguments
    #[serde(default)]
    pub arguments: Vec<ArgumentDefinition>,

    /// Named options (flags)
    #[serde(default)]
    pub options: Vec<OptionDefinition>,

    /// Name of the handler implementation
    ///
    /// This string is used to match the command with its
    /// registered handler in the CommandRegistry.
    pub implementation: String,
}

/// Definition of a positional argument
///
/// Positional arguments are required in order and don't have
/// a flag prefix (unlike options).
///
/// # Example
///
/// ```yaml
/// name: input_file
/// arg_type: path
/// required: true
/// description: "Path to input file"
/// validation:
///   - must_exist: true
///   - extensions: [yaml, yml]
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ArgumentDefinition {
    /// Argument name (used in error messages and documentation)
    pub name: String,

    /// Expected type of the argument
    pub arg_type: ArgumentType,

    /// Whether the argument is mandatory
    pub required: bool,

    /// Human-readable description
    pub description: String,

    /// Validation rules to apply
    #[serde(default)]
    pub validation: Vec<ValidationRule>,
}

/// Definition of a named option (flag)
///
/// Options are optional (by default) and can be specified
/// with short (`-o`) or long (`--option`) forms.
///
/// # Example
///
/// ```yaml
/// name: output
/// short: o
/// long: output
/// option_type: path
/// required: false
/// default: "output.txt"
/// description: "Output file path"
/// choices: []
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OptionDefinition {
    /// Option name (internal identifier)
    pub name: String,

    /// Short form (single character, e.g., "o" for -o)
    pub short: Option<String>,

    /// Long form (e.g., "output" for --output)
    pub long: Option<String>,

    /// Expected type of the option value
    pub option_type: ArgumentType,

    /// Whether this option is mandatory
    #[serde(default)]
    pub required: bool,

    /// Default value if not specified
    pub default: Option<String>,

    /// Human-readable description
    pub description: String,

    /// Restricted set of allowed values
    ///
    /// If non-empty, the value must be one of these choices.
    #[serde(default)]
    pub choices: Vec<String>,
}

/// Supported argument and option types
///
/// These types are used for automatic parsing and validation
/// of user input.
///
/// # Serialization
///
/// Types are serialized as lowercase strings in YAML/JSON:
/// - `String` → "string"
/// - `Integer` → "integer"
/// - `Float` → "float"
/// - `Bool` → "bool"
/// - `Path` → "path"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgumentType {
    /// UTF-8 string
    String,

    /// Signed integer (i64)
    Integer,

    /// Floating-point number (f64)
    Float,

    /// Boolean value (true/false, yes/no, 1/0)
    Bool,

    /// File system path
    ///
    /// Represents a path that may or may not exist,
    /// depending on validation rules.
    Path,
}

impl ArgumentType {
    /// Get the type name as a string for error messages
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::config::schema::ArgumentType;
    ///
    /// assert_eq!(ArgumentType::Integer.as_str(), "integer");
    /// assert_eq!(ArgumentType::Path.as_str(), "path");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            ArgumentType::String => "string",
            ArgumentType::Integer => "integer",
            ArgumentType::Float => "float",
            ArgumentType::Bool => "bool",
            ArgumentType::Path => "path",
        }
    }
}

/// Validation rules for arguments and options
///
/// These rules are applied after type parsing to enforce
/// additional constraints on values.
///
/// # Variants
///
/// - `MustExist`: For paths, require that the file/directory exists
/// - `Extensions`: For paths, restrict to specific file extensions
/// - `Range`: For numbers, enforce min/max bounds
///
/// # Serialization
///
/// Rules use untagged enum serialization:
///
/// ```yaml
/// # MustExist
/// - must_exist: true
///
/// # Extensions
/// - extensions: [yaml, yml, json]
///
/// # Range
/// - min: 0.0
///   max: 100.0
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ValidationRule {
    /// Require that a path exists on the file system
    MustExist { must_exist: bool },

    /// Restrict file extensions (for path arguments)
    ///
    /// Extensions should be specified without the leading dot.
    /// Example: `["yaml", "yml"]` matches "config.yaml" and "data.yml"
    Extensions { extensions: Vec<String> },

    /// Enforce numeric range constraints
    ///
    /// Either or both bounds can be specified:
    /// - `min: Some(0.0), max: None` → x ≥ 0
    /// - `min: None, max: Some(100.0)` → x ≤ 100
    /// - `min: Some(0.0), max: Some(100.0)` → 0 ≤ x ≤ 100
    Range { min: Option<f64>, max: Option<f64> },
}

impl CommandsConfig {
    /// Create a minimal valid configuration for testing
    ///
    /// This is useful for unit tests and examples.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::config::schema::CommandsConfig;
    ///
    /// let config = CommandsConfig::minimal();
    /// assert_eq!(config.metadata.version, "0.1.0");
    /// assert!(config.commands.is_empty());
    /// ```
    #[cfg(test)]
    pub fn minimal() -> Self {
        Self {
            metadata: Metadata {
                version: "0.1.0".to_string(),
                prompt: "test".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![],
            global_options: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_type_as_str() {
        assert_eq!(ArgumentType::String.as_str(), "string");
        assert_eq!(ArgumentType::Integer.as_str(), "integer");
        assert_eq!(ArgumentType::Float.as_str(), "float");
        assert_eq!(ArgumentType::Bool.as_str(), "bool");
        assert_eq!(ArgumentType::Path.as_str(), "path");
    }

    #[test]
    fn test_default_prompt_suffix() {
        assert_eq!(default_prompt_suffix(), " > ");
    }

    #[test]
    fn test_minimal_config() {
        let config = CommandsConfig::minimal();

        assert_eq!(config.metadata.version, "0.1.0");
        assert_eq!(config.metadata.prompt, "test");
        assert_eq!(config.metadata.prompt_suffix, " > ");
        assert!(config.commands.is_empty());
        assert!(config.global_options.is_empty());
    }

    #[test]
    fn test_deserialize_argument_type() {
        // Test YAML deserialization of ArgumentType
        let yaml = r#"
            type: string
        "#;

        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(rename = "type")]
            type_field: ArgumentType,
        }

        let result: TestStruct = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.type_field, ArgumentType::String);
    }

    #[test]
    fn test_deserialize_metadata() {
        let yaml = r#"
            version: "1.0.0"
            prompt: "myapp"
            prompt_suffix: " $ "
        "#;

        let metadata: Metadata = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.prompt, "myapp");
        assert_eq!(metadata.prompt_suffix, " $ ");
    }

    #[test]
    fn test_deserialize_metadata_with_default() {
        // Test that prompt_suffix gets default value if not specified
        let yaml = r#"
            version: "1.0.0"
            prompt: "myapp"
        "#;

        let metadata: Metadata = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(metadata.prompt_suffix, " > ");
    }

    #[test]
    fn test_deserialize_command_definition() {
        let yaml = r#"
            name: test_cmd
            aliases: [tc, test]
            description: "A test command"
            required: true
            arguments: []
            options: []
            implementation: "test_handler"
        "#;

        let cmd: CommandDefinition = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(cmd.name, "test_cmd");
        assert_eq!(cmd.aliases, vec!["tc", "test"]);
        assert_eq!(cmd.description, "A test command");
        assert!(cmd.required);
        assert_eq!(cmd.implementation, "test_handler");
    }

    #[test]
    fn test_deserialize_argument_definition() {
        let yaml = r#"
            name: input_file
            arg_type: path
            required: true
            description: "Input file"
            validation:
              - must_exist: true
              - extensions: [yaml, yml]
        "#;

        let arg: ArgumentDefinition = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(arg.name, "input_file");
        assert_eq!(arg.arg_type, ArgumentType::Path);
        assert!(arg.required);
        assert_eq!(arg.description, "Input file");
        assert_eq!(arg.validation.len(), 2);
    }

    #[test]
    fn test_deserialize_option_definition() {
        let yaml = r#"
            name: output
            short: o
            long: output
            option_type: path
            required: false
            default: "out.txt"
            description: "Output file"
            choices: []
        "#;

        let opt: OptionDefinition = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(opt.name, "output");
        assert_eq!(opt.short, Some("o".to_string()));
        assert_eq!(opt.long, Some("output".to_string()));
        assert_eq!(opt.option_type, ArgumentType::Path);
        assert!(!opt.required);
        assert_eq!(opt.default, Some("out.txt".to_string()));
    }

    #[test]
    fn test_deserialize_validation_rule_must_exist() {
        let yaml = r#"
            must_exist: true
        "#;

        let rule: ValidationRule = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(rule, ValidationRule::MustExist { must_exist: true });
    }

    #[test]
    fn test_deserialize_validation_rule_extensions() {
        let yaml = r#"
            extensions: [yaml, yml, json]
        "#;

        let rule: ValidationRule = serde_yaml::from_str(yaml).unwrap();

        match rule {
            ValidationRule::Extensions { extensions } => {
                assert_eq!(extensions, vec!["yaml", "yml", "json"]);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_deserialize_validation_rule_range() {
        let yaml = r#"
            min: 0.0
            max: 100.0
        "#;

        let rule: ValidationRule = serde_yaml::from_str(yaml).unwrap();

        match rule {
            ValidationRule::Range { min, max } => {
                assert_eq!(min, Some(0.0));
                assert_eq!(max, Some(100.0));
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_deserialize_full_config() {
        let yaml = r#"
            metadata:
              version: "1.0.0"
              prompt: "test"
              prompt_suffix: " > "
            commands:
              - name: hello
                aliases: []
                description: "Say hello"
                required: false
                arguments: []
                options: []
                implementation: "hello_handler"
            global_options: []
        "#;

        let config: CommandsConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].name, "hello");
    }

    #[test]
    fn test_serialize_and_deserialize_roundtrip() {
        let original = CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: "test".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![CommandDefinition {
                name: "cmd1".to_string(),
                aliases: vec!["c1".to_string()],
                description: "Test command".to_string(),
                required: true,
                arguments: vec![],
                options: vec![],
                implementation: "handler1".to_string(),
            }],
            global_options: vec![],
        };

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&original).unwrap();

        // Deserialize back
        let deserialized: CommandsConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"
        {
            "metadata": {
                "version": "1.0.0",
                "prompt": "test",
                "prompt_suffix": " > "
            },
            "commands": [],
            "global_options": []
        }
        "#;

        let config: CommandsConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.commands.len(), 0);
    }
}
