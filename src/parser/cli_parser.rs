//! CLI argument parser
//!
//! This module provides the [`CliParser`] which parses Unix-style command-line
//! arguments into a structured HashMap. It handles:
//! - Positional arguments
//! - Short options (`-v`)
//! - Long options (`--verbose`)
//! - Options with values (`-o file.txt`, `--output=file.txt`)
//! - Type conversion and validation
//!
//! # Example
//!
//! ```
//! use dynamic_cli::parser::cli_parser::CliParser;
//! use dynamic_cli::config::schema::{CommandDefinition, ArgumentDefinition, ArgumentType};
//!
//! let definition = CommandDefinition {
//!     name: "process".to_string(),
//!     aliases: vec![],
//!     description: "Process files".to_string(),
//!     required: false,
//!     arguments: vec![
//!         ArgumentDefinition {
//!             name: "input".to_string(),
//!             arg_type: ArgumentType::Path,
//!             required: true,
//!             description: "Input file".to_string(),
//!             validation: vec![],
//!         }
//!     ],
//!     options: vec![],
//!     implementation: "handler".to_string(),
//! };
//!
//! let parser = CliParser::new(&definition);
//! let args = vec!["file.txt".to_string()];
//! let parsed = parser.parse(&args).unwrap();
//!
//! assert_eq!(parsed.get("input"), Some(&"file.txt".to_string()));
//! ```

use crate::config::schema::{ArgumentDefinition, CommandDefinition, OptionDefinition};
use crate::error::{ParseError, Result};
use crate::parser::type_parser;
use std::collections::HashMap;

/// CLI argument parser
///
/// Parses command-line arguments according to a [`CommandDefinition`].
/// The parser handles both positional arguments and named options
/// with type conversion and validation.
///
/// # Lifetime
///
/// The parser holds a reference to a [`CommandDefinition`] and therefore
/// has a lifetime parameter `'a` that must outlive the parser.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::cli_parser::CliParser;
/// use dynamic_cli::config::schema::{
///     CommandDefinition, OptionDefinition, ArgumentType
/// };
///
/// let definition = CommandDefinition {
///     name: "test".to_string(),
///     aliases: vec![],
///     description: "Test command".to_string(),
///     required: false,
///     arguments: vec![],
///     options: vec![
///         OptionDefinition {
///             name: "verbose".to_string(),
///             short: Some("v".to_string()),
///             long: Some("verbose".to_string()),
///             option_type: ArgumentType::Bool,
///             required: false,
///             default: Some("false".to_string()),
///             description: "Verbose output".to_string(),
///             choices: vec![],
///         }
///     ],
///     implementation: "handler".to_string(),
/// };
///
/// let parser = CliParser::new(&definition);
/// let args = vec!["-v".to_string()];
/// let parsed = parser.parse(&args).unwrap();
///
/// assert_eq!(parsed.get("verbose"), Some(&"true".to_string()));
/// ```
pub struct CliParser<'a> {
    /// The command definition that specifies expected arguments and options
    definition: &'a CommandDefinition,
}

impl<'a> CliParser<'a> {
    /// Create a new CLI parser for the given command definition
    ///
    /// # Arguments
    ///
    /// * `definition` - The command definition specifying expected arguments
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::cli_parser::CliParser;
    /// use dynamic_cli::config::schema::CommandDefinition;
    ///
    /// # let definition = CommandDefinition {
    /// #     name: "test".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// # };
    /// let parser = CliParser::new(&definition);
    /// ```
    pub fn new(definition: &'a CommandDefinition) -> Self {
        Self { definition }
    }

    /// Parse command-line arguments into a HashMap
    ///
    /// Parses the provided arguments according to the command definition.
    /// Positional arguments are matched in order, and options are matched
    /// by their short or long forms.
    ///
    /// # Arguments
    ///
    /// * `args` - Slice of argument strings (excluding the command name)
    ///
    /// # Returns
    ///
    /// A HashMap mapping argument/option names to their string values.
    /// All values are stored as strings after type validation.
    ///
    /// # Errors
    ///
    /// - [`ParseError::MissingArgument`] if required arguments are missing
    /// - [`ParseError::MissingOption`] if required options are missing
    /// - [`ParseError::UnknownOption`] if an unrecognized option is provided
    /// - [`ParseError::TypeParseError`] if a value cannot be converted to its expected type
    /// - [`ParseError::TooManyArguments`] if more positional arguments than expected
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::cli_parser::CliParser;
    /// use dynamic_cli::config::schema::{
    ///     CommandDefinition, ArgumentDefinition, ArgumentType
    /// };
    ///
    /// let definition = CommandDefinition {
    ///     name: "greet".to_string(),
    ///     aliases: vec![],
    ///     description: "Greet someone".to_string(),
    ///     required: false,
    ///     arguments: vec![
    ///         ArgumentDefinition {
    ///             name: "name".to_string(),
    ///             arg_type: ArgumentType::String,
    ///             required: true,
    ///             description: "Name".to_string(),
    ///             validation: vec![],
    ///         }
    ///     ],
    ///     options: vec![],
    ///     implementation: "handler".to_string(),
    /// };
    ///
    /// let parser = CliParser::new(&definition);
    /// let result = parser.parse(&["Alice".to_string()]).unwrap();
    /// assert_eq!(result.get("name"), Some(&"Alice".to_string()));
    /// ```
    pub fn parse(&self, args: &[String]) -> Result<HashMap<String, String>> {
        let mut result = HashMap::new();
        let mut positional_index = 0;
        let mut i = 0;

        // Parse arguments
        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with("--") {
                // Long option
                self.parse_long_option(arg, args, &mut i, &mut result)?;
            } else if arg.starts_with('-') && arg.len() > 1 {
                // Short option (ensure it's not just a negative number)
                if arg
                    .chars()
                    .nth(1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    // This is a negative number, treat as positional
                    self.parse_positional_argument(arg, positional_index, &mut result)?;
                    positional_index += 1;
                } else {
                    self.parse_short_option(arg, args, &mut i, &mut result)?;
                }
            } else {
                // Positional argument
                self.parse_positional_argument(arg, positional_index, &mut result)?;
                positional_index += 1;
            }

            i += 1;
        }

        // Apply defaults for missing optional options
        self.apply_defaults(&mut result)?;

        // Validate all required arguments are present
        self.validate_required_arguments(&result)?;
        self.validate_required_options(&result)?;

        Ok(result)
    }

    /// Parse a long option (--option or --option=value)
    fn parse_long_option(
        &self,
        arg: &str,
        args: &[String],
        index: &mut usize,
        result: &mut HashMap<String, String>,
    ) -> Result<()> {
        let arg_without_dashes = &arg[2..];

        // Check for --option=value format
        if let Some(eq_pos) = arg_without_dashes.find('=') {
            let option_name = &arg_without_dashes[..eq_pos];
            let value = &arg_without_dashes[eq_pos + 1..];

            let option = self.find_option_by_long(option_name)?;
            let parsed_value = type_parser::parse_value(value, option.option_type)?;
            result.insert(option.name.clone(), parsed_value);
        } else {
            // --option format (value might be next arg)
            let option = self.find_option_by_long(arg_without_dashes)?;

            // For boolean options, presence means true
            if matches!(
                option.option_type,
                crate::config::schema::ArgumentType::Bool
            ) {
                result.insert(option.name.clone(), "true".to_string());
            } else {
                // Non-boolean: expect value in next argument
                *index += 1;
                if *index >= args.len() {
                    return Err(ParseError::InvalidSyntax {
                        details: format!(
                            "Option --{} requires a value",
                            option.long.as_ref().unwrap()
                        ),
                        hint: Some(format!(
                            "Usage: --{}=<value> or --{} <value>",
                            option.long.as_ref().unwrap(),
                            option.long.as_ref().unwrap()
                        )),
                    }
                    .into());
                }

                let value = &args[*index];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), parsed_value);
            }
        }

        Ok(())
    }

    /// Parse a short option (-o or -o value)
    fn parse_short_option(
        &self,
        arg: &str,
        args: &[String],
        index: &mut usize,
        result: &mut HashMap<String, String>,
    ) -> Result<()> {
        let short_flag = &arg[1..2];
        let option = self.find_option_by_short(short_flag)?;

        // For boolean options, presence means true
        if matches!(
            option.option_type,
            crate::config::schema::ArgumentType::Bool
        ) {
            result.insert(option.name.clone(), "true".to_string());
        } else {
            // Check if value is attached (e.g., -ovalue)
            if arg.len() > 2 {
                let value = &arg[2..];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), parsed_value);
            } else {
                // Value is next argument
                *index += 1;
                if *index >= args.len() {
                    return Err(ParseError::InvalidSyntax {
                        details: format!("Option -{} requires a value", short_flag),
                        hint: Some(format!(
                            "Usage: -{}<value> or -{} <value>",
                            short_flag, short_flag
                        )),
                    }
                    .into());
                }

                let value = &args[*index];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), parsed_value);
            }
        }

        Ok(())
    }

    /// Parse a positional argument
    fn parse_positional_argument(
        &self,
        value: &str,
        index: usize,
        result: &mut HashMap<String, String>,
    ) -> Result<()> {
        if index >= self.definition.arguments.len() {
            return Err(ParseError::TooManyArguments {
                command: self.definition.name.clone(),
                expected: self.definition.arguments.len(),
                got: index + 1,
            }
            .into());
        }

        let arg_def = &self.definition.arguments[index];
        let parsed_value = type_parser::parse_value(value, arg_def.arg_type)?;
        result.insert(arg_def.name.clone(), parsed_value);

        Ok(())
    }

    /// Apply default values for options not provided
    fn apply_defaults(&self, result: &mut HashMap<String, String>) -> Result<()> {
        for option in &self.definition.options {
            if !result.contains_key(&option.name) {
                if let Some(ref default) = option.default {
                    // Validate the default value
                    let parsed_default = type_parser::parse_value(default, option.option_type)?;
                    result.insert(option.name.clone(), parsed_default);
                }
            }
        }
        Ok(())
    }

    /// Validate that all required arguments are present
    fn validate_required_arguments(&self, result: &HashMap<String, String>) -> Result<()> {
        for arg in &self.definition.arguments {
            if arg.required && !result.contains_key(&arg.name) {
                return Err(ParseError::MissingArgument {
                    argument: arg.name.clone(),
                    command: self.definition.name.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Validate that all required options are present
    fn validate_required_options(&self, result: &HashMap<String, String>) -> Result<()> {
        for option in &self.definition.options {
            if option.required && !result.contains_key(&option.name) {
                return Err(ParseError::MissingOption {
                    option: option
                        .long
                        .clone()
                        .or(option.short.clone())
                        .unwrap_or_default(),
                    command: self.definition.name.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    /// Find an option by its long form
    fn find_option_by_long(&self, long: &str) -> Result<&OptionDefinition> {
        self.definition
            .options
            .iter()
            .find(|opt| opt.long.as_deref() == Some(long))
            .ok_or_else(|| {
                let available: Vec<String> = self
                    .definition
                    .options
                    .iter()
                    .filter_map(|o| o.long.clone())
                    .collect();
                ParseError::unknown_option_with_suggestions(
                    &format!("--{}", long),
                    &self.definition.name,
                    &available,
                )
                .into()
            })
    }

    /// Find an option by its short form
    fn find_option_by_short(&self, short: &str) -> Result<&OptionDefinition> {
        self.definition
            .options
            .iter()
            .find(|opt| opt.short.as_deref() == Some(short))
            .ok_or_else(|| {
                let available: Vec<String> = self
                    .definition
                    .options
                    .iter()
                    .filter_map(|o| o.short.clone())
                    .collect();
                ParseError::unknown_option_with_suggestions(
                    &format!("-{}", short),
                    &self.definition.name,
                    &available,
                )
                .into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentType, OptionDefinition};

    /// Helper to create a test command definition
    fn create_test_definition() -> CommandDefinition {
        CommandDefinition {
            name: "test".to_string(),
            aliases: vec![],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "input".to_string(),
                    arg_type: ArgumentType::Path,
                    required: true,
                    description: "Input file".to_string(),
                    validation: vec![],
                },
                ArgumentDefinition {
                    name: "output".to_string(),
                    arg_type: ArgumentType::Path,
                    required: false,
                    description: "Output file".to_string(),
                    validation: vec![],
                },
            ],
            options: vec![
                OptionDefinition {
                    name: "verbose".to_string(),
                    short: Some("v".to_string()),
                    long: Some("verbose".to_string()),
                    option_type: ArgumentType::Bool,
                    required: false,
                    default: Some("false".to_string()),
                    description: "Verbose output".to_string(),
                    choices: vec![],
                },
                OptionDefinition {
                    name: "count".to_string(),
                    short: Some("c".to_string()),
                    long: Some("count".to_string()),
                    option_type: ArgumentType::Integer,
                    required: false,
                    default: Some("10".to_string()),
                    description: "Count".to_string(),
                    choices: vec![],
                },
            ],
            implementation: "handler".to_string(),
        }
    }

    // ========================================================================
    // Positional arguments tests
    // ========================================================================

    #[test]
    fn test_parse_single_positional_argument() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
    }

    #[test]
    fn test_parse_multiple_positional_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "output.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
    }

    #[test]
    fn test_parse_missing_required_argument() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args: Vec<String> = vec![];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::MissingArgument {
                argument, ..
            }) => {
                assert_eq!(argument, "input");
            }
            other => panic!("Expected MissingArgument error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_too_many_positional_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "extra.txt".to_string(),
        ];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::TooManyArguments { .. }) => {}
            other => panic!("Expected TooManyArguments error, got {:?}", other),
        }
    }

    // ========================================================================
    // Long options tests
    // ========================================================================

    #[test]
    fn test_parse_long_boolean_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--verbose".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_long_option_with_equals() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--count=42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_long_option_with_space() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "--count".to_string(),
            "42".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_unknown_long_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--unknown".to_string()];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::UnknownOption { .. }) => {}
            other => panic!("Expected UnknownOption error, got {:?}", other),
        }
    }

    // ========================================================================
    // Short options tests
    // ========================================================================

    #[test]
    fn test_parse_short_boolean_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-v".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_short_option_with_space() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-c".to_string(), "42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_short_option_attached_value() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-c42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_negative_number_as_positional() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // -123 should be treated as a positional argument, not an option
        let args = vec!["-123".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"-123".to_string()));
    }

    // ========================================================================
    // Default values tests
    // ========================================================================

    #[test]
    fn test_apply_default_values() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        // Default values should be applied
        assert_eq!(result.get("verbose"), Some(&"false".to_string()));
        assert_eq!(result.get("count"), Some(&"10".to_string()));
    }

    #[test]
    fn test_override_default_values() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "-v".to_string(),
            "-c".to_string(),
            "5".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        // Provided values should override defaults
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
    }

    // ========================================================================
    // Type conversion tests
    // ========================================================================

    #[test]
    fn test_type_conversion_error() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // "abc" cannot be parsed as integer
        let args = vec![
            "input.txt".to_string(),
            "--count".to_string(),
            "abc".to_string(),
        ];
        let result = parser.parse(&args);

        assert!(result.is_err());
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_parse_complex_command_line() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "--verbose".to_string(),
            "--count=100".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"100".to_string()));
    }

    #[test]
    fn test_parse_mixed_options_and_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // Options can be interspersed with positional arguments
        let args = vec![
            "--verbose".to_string(),
            "input.txt".to_string(),
            "-c".to_string(),
            "50".to_string(),
            "output.txt".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"50".to_string()));
    }
}
