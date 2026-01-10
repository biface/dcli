//! Configuration module
//!
//! This module provides functionality for loading, parsing, and validating
//! YAML/JSON configuration files that define CLI/REPL commands.
//!
//! # Module Structure
//!
//! - [`schema`]: Data structures for configuration
//! - [`loader`]: Functions to load configuration files
//! - [`validator`]: Configuration validation logic
//!
//! # Quick Start
//!
//! ```no_run
//! use dynamic_cli::config::{loader::load_config, validator::validate_config};
//!
//! // Load configuration from file
//! let config = load_config("commands.yaml")?;
//!
//! // Validate the configuration
//! validate_config(&config)?;
//!
//! println!("Loaded {} commands", config.commands.len());
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! # Configuration File Format
//!
//! ## YAML Example
//!
//! ```yaml
//! metadata:
//!   version: "1.0.0"
//!   prompt: "myapp"
//!   prompt_suffix: " > "
//!
//! commands:
//!   - name: hello
//!     aliases: [hi]
//!     description: "Say hello"
//!     required: false
//!     arguments:
//!       - name: name
//!         arg_type: string
//!         required: true
//!         description: "Name to greet"
//!         validation: []
//!     options:
//!       - name: loud
//!         short: l
//!         long: loud
//!         option_type: bool
//!         required: false
//!         description: "Use uppercase"
//!         choices: []
//!     implementation: "hello_handler"
//!
//! global_options: []
//! ```
//!
//! ## JSON Example
//!
//! ```json
//! {
//!   "metadata": {
//!     "version": "1.0.0",
//!     "prompt": "myapp"
//!   },
//!   "commands": [
//!     {
//!       "name": "hello",
//!       "aliases": [],
//!       "description": "Say hello",
//!       "required": false,
//!       "arguments": [],
//!       "options": [],
//!       "implementation": "hello_handler"
//!     }
//!   ],
//!   "global_options": []
//! }
//! ```

// Public submodules
pub mod loader;
pub mod schema;
pub mod validator;

// Re-export commonly used types and functions for convenience
#[allow(unused_imports)]
pub use schema::{
    ArgumentDefinition, ArgumentType, CommandDefinition, CommandsConfig, Metadata,
    OptionDefinition, ValidationRule,
};

#[allow(unused_imports)]
pub use loader::{load_config, load_json, load_yaml};

#[allow(unused_imports)]
pub use validator::{validate_argument_types, validate_command, validate_config};

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: Load and validate a complete configuration
    #[test]
    fn test_integration_load_and_validate() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
  prompt_suffix: " > "

commands:
  - name: process
    aliases: [proc, p]
    description: "Process data files"
    required: true
    arguments:
      - name: input
        arg_type: path
        required: true
        description: "Input file path"
        validation:
          - must_exist: true
          - extensions: [csv, tsv]
      - name: output
        arg_type: path
        required: false
        description: "Output file path"
        validation: []
    options:
      - name: verbose
        short: v
        long: verbose
        option_type: bool
        required: false
        description: "Verbose output"
        choices: []
      - name: format
        short: f
        long: format
        option_type: string
        required: false
        default: "json"
        description: "Output format"
        choices: [json, xml, csv]
    implementation: "process_handler"

  - name: help
    aliases: [h]
    description: "Show help information"
    required: false
    arguments: []
    options: []
    implementation: "help_handler"

global_options:
  - name: config
    short: c
    long: config
    option_type: path
    required: false
    description: "Configuration file"
    choices: []
        "#;

        // Load the configuration
        let config = load_yaml(yaml).unwrap();

        // Validate it
        validate_config(&config).unwrap();

        // Verify structure
        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.commands.len(), 2);
        assert_eq!(config.commands[0].name, "process");
        assert_eq!(config.commands[0].arguments.len(), 2);
        assert_eq!(config.commands[0].options.len(), 2);
        assert_eq!(config.global_options.len(), 1);
    }

    /// Test that validation catches errors in loaded config
    #[test]
    fn test_integration_invalid_config() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"

commands:
  - name: cmd1
    aliases: []
    description: "Command 1"
    required: false
    arguments: []
    options: []
    implementation: "handler1"
  - name: cmd1
    aliases: []
    description: "Command 2 with duplicate name"
    required: false
    arguments: []
    options: []
    implementation: "handler2"

global_options: []
        "#;

        let config = load_yaml(yaml).unwrap();
        let result = validate_config(&config);

        // Should fail due to duplicate command name
        assert!(result.is_err());
    }

    /// Test loading from JSON format
    #[test]
    fn test_integration_json_format() {
        let json = r#"
{
  "metadata": {
    "version": "2.0.0",
    "prompt": "myapp",
    "prompt_suffix": " $ "
  },
  "commands": [
    {
      "name": "test",
      "aliases": [],
      "description": "Test command",
      "required": false,
      "arguments": [
        {
          "name": "value",
          "arg_type": "integer",
          "required": true,
          "description": "A test value",
          "validation": [
            {
              "min": 0.0,
              "max": 100.0
            }
          ]
        }
      ],
      "options": [],
      "implementation": "test_handler"
    }
  ],
  "global_options": []
}
        "#;

        let config = load_json(json).unwrap();
        validate_config(&config).unwrap();

        assert_eq!(config.metadata.version, "2.0.0");
        assert_eq!(
            config.commands[0].arguments[0].arg_type,
            ArgumentType::Integer
        );
    }

    /// Test re-exported types are accessible
    #[test]
    fn test_reexports() {
        // This test verifies that re-exported types are accessible
        // from the module root
        let _config = CommandsConfig::minimal();
        let _arg_type = ArgumentType::String;

        // If this compiles, re-exports are working
    }

    /// Test complex validation rules
    #[test]
    fn test_integration_complex_validation() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"

commands:
  - name: analyze
    aliases: []
    description: "Analyze data"
    required: false
    arguments:
      - name: data_file
        arg_type: path
        required: true
        description: "Data file"
        validation:
          - must_exist: true
          - extensions: [dat, bin]
      - name: threshold
        arg_type: float
        required: true
        description: "Analysis threshold"
        validation:
          - min: 0.0
            max: 1.0
    options:
      - name: iterations
        short: i
        long: iterations
        option_type: integer
        required: false
        default: "100"
        description: "Number of iterations"
        choices: []
    implementation: "analyze_handler"

global_options: []
        "#;

        let config = load_yaml(yaml).unwrap();
        let result = validate_config(&config);

        assert!(result.is_ok());

        // Verify validation rules were parsed correctly
        let cmd = &config.commands[0];
        assert_eq!(cmd.arguments[0].validation.len(), 2);
        assert_eq!(cmd.arguments[1].validation.len(), 1);
    }

    /// Test error message quality
    #[test]
    fn test_integration_error_messages() {
        // Test various error conditions to ensure error messages are helpful

        // 1. Invalid YAML syntax
        let bad_yaml = "metadata:\n  version: [unclosed";
        let result = load_yaml(bad_yaml);
        assert!(result.is_err());

        // 2. Type mismatch in validation rules
        let yaml_type_error = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands:
  - name: cmd
    aliases: []
    description: "Test"
    required: false
    arguments:
      - name: count
        arg_type: integer
        required: true
        description: "Count"
        validation:
          - must_exist: true
    options: []
    implementation: "handler"
global_options: []
        "#;

        let config = load_yaml(yaml_type_error).unwrap();
        let result = validate_config(&config);
        assert!(result.is_err());
    }
}
