//! Configuration file loading
//!
//! This module provides functions to load configuration from
//! YAML or JSON files, with automatic format detection.
//!
//! # Supported Formats
//!
//! - YAML (`.yaml`, `.yml`)
//! - JSON (`.json`)
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::config::loader::load_config;
//! use std::path::Path;
//!
//! let config = load_config("commands.yaml").unwrap();
//! println!("Loaded {} commands", config.commands.len());
//! ```

use crate::config::schema::CommandsConfig;
use crate::error::{ConfigError, DynamicCliError, Result};
use std::fs;
use std::path::Path;

/// Load configuration from a file
///
/// Automatically detects the format (YAML or JSON) based on
/// the file extension and parses the content accordingly.
///
/// # Supported Extensions
///
/// - `.yaml`, `.yml` → YAML parser
/// - `.json` → JSON parser
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Returns
///
/// Parsed [`CommandsConfig`] on success
///
/// # Errors
///
/// - [`ConfigError::FileNotFound`] if the file doesn't exist
/// - [`ConfigError::UnsupportedFormat`] if the extension is not recognized
/// - [`ConfigError::YamlParse`] or [`ConfigError::JsonParse`] if parsing fails
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::config::loader::load_config;
///
/// // Load YAML configuration
/// let config = load_config("commands.yaml")?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<CommandsConfig> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(ConfigError::FileNotFound {
            path: path.to_path_buf(),
        }
        .into());
    }

    // Detect format from extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| ConfigError::UnsupportedFormat {
            extension: "<none>".to_string(),
        })?;

    // Read file content
    let content = fs::read_to_string(path).map_err(DynamicCliError::from)?;

    // Parse according to format
    match extension.to_lowercase().as_str() {
        "yaml" | "yml" => load_yaml(&content),
        "json" => load_json(&content),
        other => Err(ConfigError::UnsupportedFormat {
            extension: other.to_string(),
        }
        .into()),
    }
}

/// Load configuration from a YAML string
///
/// Parses YAML content and deserializes it into a [`CommandsConfig`].
/// Provides detailed error messages with line and column information
/// when parsing fails.
///
/// # Arguments
///
/// * `content` - YAML string to parse
///
/// # Returns
///
/// Parsed [`CommandsConfig`] on success
///
/// # Errors
///
/// - [`ConfigError::YamlParse`] if the YAML is invalid or doesn't match the schema
///
/// # Example
///
/// ```
/// use dynamic_cli::config::loader::load_yaml;
///
/// let yaml = r#"
/// metadata:
///   version: "1.0.0"
///   prompt: "test"
/// commands: []
/// global_options: []
/// "#;
///
/// let config = load_yaml(yaml).unwrap();
/// assert_eq!(config.metadata.version, "1.0.0");
/// ```
pub fn load_yaml(content: &str) -> Result<CommandsConfig> {
    serde_yaml::from_str(content).map_err(|e| {
        // Extract position information from error
        ConfigError::yaml_parse_with_location(e).into()
    })
}

/// Load configuration from a JSON string
///
/// Parses JSON content and deserializes it into a [`CommandsConfig`].
/// Provides detailed error messages with line and column information
/// when parsing fails.
///
/// # Arguments
///
/// * `content` - JSON string to parse
///
/// # Returns
///
/// Parsed [`CommandsConfig`] on success
///
/// # Errors
///
/// - [`ConfigError::JsonParse`] if the JSON is invalid or doesn't match the schema
///
/// # Example
///
/// ```
/// use dynamic_cli::config::loader::load_json;
///
/// let json = r#"
/// {
///   "metadata": {
///     "version": "1.0.0",
///     "prompt": "test"
///   },
///   "commands": [],
///   "global_options": []
/// }
/// "#;
///
/// let config = load_json(json).unwrap();
/// assert_eq!(config.metadata.version, "1.0.0");
/// ```
pub fn load_json(content: &str) -> Result<CommandsConfig> {
    serde_json::from_str(content).map_err(|e| {
        // Extract position information from error
        ConfigError::json_parse_with_location(e).into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a temporary file with content
    fn create_temp_file(content: &str, extension: &str) -> NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(extension)
            .tempfile()
            .unwrap();

        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_load_yaml_valid() {
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

        let config = load_yaml(yaml).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.metadata.prompt, "test");
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].name, "hello");
    }

    #[test]
    fn test_load_yaml_invalid_syntax() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands: [
        "#; // Invalid YAML - unclosed array

        let result = load_yaml(yaml);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::YamlParse { .. }) => {}
            other => panic!("Expected YamlParse error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_json_valid() {
        let json = r#"
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "test",
    "prompt_suffix": " > "
  },
  "commands": [
    {
      "name": "hello",
      "aliases": [],
      "description": "Say hello",
      "required": false,
      "arguments": [],
      "options": [],
      "implementation": "hello_handler"
    }
  ],
  "global_options": []
}
        "#;

        let config = load_json(json).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].name, "hello");
    }

    #[test]
    fn test_load_json_invalid_syntax() {
        let json = r#"
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "test"
  },
  "commands": [
        "#; // Invalid JSON - unclosed array

        let result = load_json(json);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::JsonParse { .. }) => {}
            other => panic!("Expected JsonParse error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_config_yaml_file() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands: []
global_options: []
        "#;

        let file = create_temp_file(yaml, ".yaml");
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
    }

    #[test]
    fn test_load_config_yml_extension() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands: []
global_options: []
        "#;

        let file = create_temp_file(yaml, ".yml");
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
    }

    #[test]
    fn test_load_config_json_file() {
        let json = r#"
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "test"
  },
  "commands": [],
  "global_options": []
}
        "#;

        let file = create_temp_file(json, ".json");
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config("nonexistent_file.yaml");

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::FileNotFound { path }) => {
                assert!(path.to_str().unwrap().contains("nonexistent_file.yaml"));
            }
            other => panic!("Expected FileNotFound error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_config_unsupported_extension() {
        let content = "some content";
        let file = create_temp_file(content, ".txt");

        let result = load_config(file.path());

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::UnsupportedFormat { extension }) => {
                assert_eq!(extension, "txt");
            }
            other => panic!("Expected UnsupportedFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_config_no_extension() {
        let content = "some content";

        // Create a file without extension
        let mut file = tempfile::Builder::new()
            .suffix("") // No suffix
            .tempfile()
            .unwrap();

        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();

        // Rename to remove any extension
        let path_without_ext = file.path().with_file_name("configfile");
        std::fs::copy(file.path(), &path_without_ext).unwrap();

        let result = load_config(&path_without_ext);

        // Cleanup
        let _ = std::fs::remove_file(&path_without_ext);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::UnsupportedFormat { .. }) => {}
            other => panic!("Expected UnsupportedFormat error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_yaml_with_complex_structure() {
        let yaml = r#"
metadata:
  version: "2.0.0"
  prompt: "myapp"
  prompt_suffix: " $ "
commands:
  - name: process
    aliases: [proc, p]
    description: "Process data"
    required: true
    arguments:
      - name: input
        arg_type: path
        required: true
        description: "Input file"
        validation:
          - must_exist: true
          - extensions: [csv, tsv]
    options:
      - name: output
        short: o
        long: output
        option_type: path
        required: false
        default: "output.txt"
        description: "Output file"
        choices: []
    implementation: "process_handler"
global_options:
  - name: verbose
    short: v
    long: verbose
    option_type: bool
    required: false
    description: "Verbose output"
    choices: []
        "#;

        let config = load_yaml(yaml).unwrap();

        assert_eq!(config.metadata.version, "2.0.0");
        assert_eq!(config.commands.len(), 1);
        assert_eq!(config.commands[0].arguments.len(), 1);
        assert_eq!(config.commands[0].options.len(), 1);
        assert_eq!(config.global_options.len(), 1);
    }

    #[test]
    fn test_load_json_with_complex_structure() {
        let json = r#"
{
  "metadata": {
    "version": "2.0.0",
    "prompt": "myapp"
  },
  "commands": [
    {
      "name": "process",
      "aliases": ["proc"],
      "description": "Process data",
      "required": true,
      "arguments": [
        {
          "name": "input",
          "arg_type": "path",
          "required": true,
          "description": "Input file",
          "validation": [
            {"must_exist": true},
            {"extensions": ["csv"]}
          ]
        }
      ],
      "options": [],
      "implementation": "process_handler"
    }
  ],
  "global_options": []
}
        "#;

        let config = load_json(json).unwrap();

        assert_eq!(config.metadata.version, "2.0.0");
        assert_eq!(config.commands[0].arguments.len(), 1);
    }

    #[test]
    fn test_error_contains_position_yaml() {
        // YAML with actual syntax error (unclosed array)
        let yaml_syntax_error = "{{{";

        let result = load_yaml(yaml_syntax_error);

        // Should fail due to YAML syntax error
        assert!(result.is_err());

        // Verify it's a YamlParse error
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::YamlParse { .. }) => {
                // Success - we got the expected error type
            }
            other => panic!("Expected YamlParse error, got {:?}", other),
        }
    }

    #[test]
    fn test_case_insensitive_extension() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands: []
global_options: []
        "#;

        // Test with uppercase extension
        let file = create_temp_file(yaml, ".YAML");
        let config = load_config(file.path()).unwrap();

        assert_eq!(config.metadata.version, "1.0.0");
    }
}
