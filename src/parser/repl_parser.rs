//! REPL line parser
//!
//! This module provides the [`ReplParser`] which parses interactive REPL
//! command lines. It works with the [`CommandRegistry`] to resolve command
//! names and aliases, then delegates to [`CliParser`] for argument parsing.
//!
//! # Example
//!
//! ```
//! use dynamic_cli::parser::repl_parser::ReplParser;
//! use dynamic_cli::registry::CommandRegistry;
//! use dynamic_cli::config::schema::{CommandDefinition, ArgumentType};
//! use dynamic_cli::executor::CommandHandler;
//! use dynamic_cli::context::ExecutionContext;
//! use std::collections::HashMap;
//!
//! // Create registry
//! let mut registry = CommandRegistry::new();
//!
//! // Register a command
//! let definition = CommandDefinition {
//!     name: "hello".to_string(),
//!     aliases: vec!["hi".to_string()],
//!     description: "Say hello".to_string(),
//!     required: false,
//!     arguments: vec![],
//!     options: vec![],
//!     implementation: "handler".to_string(),
//! };
//!
//! // Dummy handler for example
//! struct DummyHandler;
//! impl CommandHandler for DummyHandler {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         _args: &HashMap<String, String>,
//!     ) -> dynamic_cli::error::Result<()> {
//!         Ok(())
//!     }
//! }
//!
//! registry.register(definition, Box::new(DummyHandler)).unwrap();
//!
//! // Parse a REPL line
//! let parser = ReplParser::new(&registry);
//! let parsed = parser.parse_line("hi").unwrap();
//! assert_eq!(parsed.command_name, "hello");
//! ```

use crate::error::{ParseError, Result};
use crate::parser::cli_parser::CliParser;
use crate::registry::CommandRegistry;
use std::collections::HashMap;

/// REPL line parser
///
/// Parses interactive command lines in REPL mode. The parser:
/// 1. Splits the line into command name and arguments
/// 2. Resolves the command name (including aliases) via the registry
/// 3. Delegates to [`CliParser`] for argument parsing
///
/// # Lifetime
///
/// Holds a reference to a [`CommandRegistry`] and therefore has a
/// lifetime parameter `'a`.
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::parser::repl_parser::ReplParser;
/// use dynamic_cli::registry::CommandRegistry;
///
/// let registry = CommandRegistry::new();
/// let parser = ReplParser::new(&registry);
///
/// // Parse various command formats
/// let parsed = parser.parse_line("command arg1 arg2").unwrap();
/// let parsed = parser.parse_line("cmd --option value").unwrap();
/// let parsed = parser.parse_line("alias -v").unwrap();
/// ```
pub struct ReplParser<'a> {
    /// Reference to the command registry for name resolution
    registry: &'a CommandRegistry,
}

/// Parsed REPL command
///
/// Contains the resolved command name and parsed arguments.
/// This structure is the output of [`ReplParser::parse_line`].
///
/// # Fields
///
/// - `command_name`: The canonical command name (aliases are resolved)
/// - `arguments`: HashMap of argument/option names to their string values
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::repl_parser::ParsedCommand;
/// use std::collections::HashMap;
///
/// let mut args = HashMap::new();
/// args.insert("input".to_string(), "file.txt".to_string());
///
/// let parsed = ParsedCommand {
///     command_name: "process".to_string(),
///     arguments: args,
/// };
///
/// assert_eq!(parsed.command_name, "process");
/// assert_eq!(parsed.arguments.get("input"), Some(&"file.txt".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCommand {
    /// The canonical command name (after alias resolution)
    pub command_name: String,

    /// Parsed arguments and options
    pub arguments: HashMap<String, String>,
}

impl<'a> ReplParser<'a> {
    /// Create a new REPL parser with the given registry
    ///
    /// # Arguments
    ///
    /// * `registry` - The command registry for resolving command names
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::repl_parser::ReplParser;
    /// use dynamic_cli::registry::CommandRegistry;
    ///
    /// let registry = CommandRegistry::new();
    /// let parser = ReplParser::new(&registry);
    /// ```
    pub fn new(registry: &'a CommandRegistry) -> Self {
        Self { registry }
    }

    /// Parse a REPL command line
    ///
    /// Parses a complete command line as entered in the REPL.
    /// The line is split into tokens, the first token is resolved as
    /// a command name (or alias), and remaining tokens are parsed
    /// as arguments and options.
    ///
    /// # Arguments
    ///
    /// * `line` - The command line to parse
    ///
    /// # Returns
    ///
    /// A [`ParsedCommand`] containing the command name and parsed arguments
    ///
    /// # Errors
    ///
    /// - [`ParseError::UnknownCommand`] if the command is not registered
    /// - [`ParseError::InvalidSyntax`] if the line is empty or malformed
    /// - Any errors from [`CliParser`] during argument parsing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dynamic_cli::parser::repl_parser::ReplParser;
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # let registry = CommandRegistry::new();
    /// let parser = ReplParser::new(&registry);
    ///
    /// // Simple command
    /// let parsed = parser.parse_line("help").unwrap();
    ///
    /// // Command with arguments
    /// let parsed = parser.parse_line("process input.txt output.txt").unwrap();
    ///
    /// // Command with options
    /// let parsed = parser.parse_line("run --verbose --count=10").unwrap();
    /// ```
    pub fn parse_line(&self, line: &str) -> Result<ParsedCommand> {
        // Tokenize the line (respecting quotes)
        let tokens = self.tokenize(line)?;

        if tokens.is_empty() {
            return Err(ParseError::InvalidSyntax {
                details: "Empty command line".to_string(),
                hint: Some("Type a command or 'help' for available commands".to_string()),
            }
            .into());
        }

        // First token is the command name
        let input_name = &tokens[0];

        // Resolve command name (handles aliases)
        let command_name = self
            .registry
            .resolve_name(input_name)
            .ok_or_else(|| {
                // Get list of all available commands for suggestions
                let available: Vec<String> = self
                    .registry
                    .list_commands()
                    .iter()
                    .flat_map(|cmd| {
                        let mut names = vec![cmd.name.clone()];
                        names.extend(cmd.aliases.clone());
                        names
                    })
                    .collect();

                ParseError::unknown_command_with_suggestions(input_name, &available)
            })?
            .to_string();

        // Get command definition for argument parsing
        let definition = self
            .registry
            .get_definition(&command_name)
            .expect("Command definition must exist after resolution");

        // Parse arguments using CliParser
        let remaining_args: Vec<String> = tokens[1..].to_vec();
        let cli_parser = CliParser::new(definition);
        let arguments = cli_parser.parse(&remaining_args)?;

        Ok(ParsedCommand {
            command_name,
            arguments,
        })
    }

    /// Tokenize a command line into arguments
    ///
    /// This function performs simple tokenization by splitting on whitespace
    /// while respecting quoted strings. It handles:
    /// - Single quotes: `'quoted string'`
    /// - Double quotes: `"quoted string"`
    /// - Escaped quotes within quotes: `"say \"hello\""`
    ///
    /// # Arguments
    ///
    /// * `line` - The line to tokenize
    ///
    /// # Returns
    ///
    /// Vector of token strings
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidSyntax`] if quotes are unbalanced
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::parser::repl_parser::ReplParser;
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # let registry = CommandRegistry::new();
    /// # let parser = ReplParser::new(&registry);
    /// // Simple tokens
    /// let tokens = parser.tokenize("cmd arg1 arg2").unwrap();
    /// assert_eq!(tokens, vec!["cmd", "arg1", "arg2"]);
    ///
    /// // Quoted strings
    /// let tokens = parser.tokenize(r#"cmd "hello world""#).unwrap();
    /// assert_eq!(tokens, vec!["cmd", "hello world"]);
    /// ```
    pub fn tokenize(&self, line: &str) -> Result<Vec<String>> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                // Handle quotes
                '"' | '\'' => {
                    if in_quotes && ch == quote_char {
                        // End of quoted string
                        in_quotes = false;
                        quote_char = ' ';
                    } else if !in_quotes {
                        // Start of quoted string
                        in_quotes = true;
                        quote_char = ch;
                    } else {
                        // Quote char inside different quotes
                        current_token.push(ch);
                    }
                }

                // Handle whitespace
                ' ' | '\t' => {
                    if in_quotes {
                        current_token.push(ch);
                    } else if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                }

                // Handle escape sequences
                '\\' => {
                    if let Some(&next_ch) = chars.peek() {
                        if in_quotes && (next_ch == quote_char || next_ch == '\\') {
                            chars.next(); // Consume the escaped character
                            current_token.push(next_ch);
                        } else {
                            current_token.push(ch);
                        }
                    } else {
                        current_token.push(ch);
                    }
                }

                // Regular character
                _ => {
                    current_token.push(ch);
                }
            }
        }

        // Check for unbalanced quotes
        if in_quotes {
            return Err(ParseError::InvalidSyntax {
                details: format!("Unbalanced quote: {}", quote_char),
                hint: Some("Make sure all quotes are properly closed".to_string()),
            }
            .into());
        }

        // Add last token if any
        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, CommandDefinition, OptionDefinition};
    use crate::context::ExecutionContext;
    use crate::executor::CommandHandler;

    // Dummy handler for tests
    struct TestHandler;

    impl CommandHandler for TestHandler {
        fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> crate::error::Result<()> {
            Ok(())
        }
    }

    /// Helper to create a test registry with some commands
    fn create_test_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();

        // Register "hello" command with "hi" alias
        let hello_def = CommandDefinition {
            name: "hello".to_string(),
            aliases: vec!["hi".to_string(), "greet".to_string()],
            description: "Say hello".to_string(),
            required: false,
            arguments: vec![ArgumentDefinition {
                name: "name".to_string(),
                arg_type: ArgumentType::String,
                required: false,
                description: "Name to greet".to_string(),
                validation: vec![],
            }],
            options: vec![OptionDefinition {
                name: "loud".to_string(),
                short: Some("l".to_string()),
                long: Some("loud".to_string()),
                option_type: ArgumentType::Bool,
                required: false,
                default: Some("false".to_string()),
                description: "Loud greeting".to_string(),
                choices: vec![],
            }],
            implementation: "hello_handler".to_string(),
        };

        registry.register(hello_def, Box::new(TestHandler)).unwrap();

        // Register "process" command
        let process_def = CommandDefinition {
            name: "process".to_string(),
            aliases: vec!["proc".to_string()],
            description: "Process files".to_string(),
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
            options: vec![OptionDefinition {
                name: "verbose".to_string(),
                short: Some("v".to_string()),
                long: Some("verbose".to_string()),
                option_type: ArgumentType::Bool,
                required: false,
                default: Some("false".to_string()),
                description: "Verbose output".to_string(),
                choices: vec![],
            }],
            implementation: "process_handler".to_string(),
        };

        registry.register(process_def, Box::new(TestHandler)).unwrap();

        registry
    }

    // ========================================================================
    // Tokenization tests
    // ========================================================================

    #[test]
    fn test_tokenize_simple() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize("hello world").unwrap();
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_multiple_spaces() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize("hello    world   test").unwrap();
        assert_eq!(tokens, vec!["hello", "world", "test"]);
    }

    #[test]
    fn test_tokenize_double_quotes() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize(r#"hello "world test""#).unwrap();
        assert_eq!(tokens, vec!["hello", "world test"]);
    }

    #[test]
    fn test_tokenize_single_quotes() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize("hello 'world test'").unwrap();
        assert_eq!(tokens, vec!["hello", "world test"]);
    }

    #[test]
    fn test_tokenize_escaped_quotes() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize(r#"hello "say \"hi\"""#).unwrap();
        assert_eq!(tokens, vec!["hello", r#"say "hi""#]);
    }

    #[test]
    fn test_tokenize_unbalanced_quotes() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let result = parser.tokenize(r#"hello "world"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_tokenize_empty_line() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_only_spaces() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let tokens = parser.tokenize("    ").unwrap();
        assert!(tokens.is_empty());
    }

    // ========================================================================
    // Command name resolution tests
    // ========================================================================

    #[test]
    fn test_parse_command_by_name() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("hello").unwrap();
        assert_eq!(parsed.command_name, "hello");
    }

    #[test]
    fn test_parse_command_by_alias() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("hi").unwrap();
        assert_eq!(parsed.command_name, "hello");

        let parsed = parser.parse_line("greet").unwrap();
        assert_eq!(parsed.command_name, "hello");
    }

    #[test]
    fn test_parse_unknown_command() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let result = parser.parse_line("unknown");
        assert!(result.is_err());

        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::UnknownCommand { command, .. }) => {
                assert_eq!(command, "unknown");
            }
            other => panic!("Expected UnknownCommand error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_line() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let result = parser.parse_line("");
        assert!(result.is_err());
    }

    // ========================================================================
    // Argument parsing tests
    // ========================================================================

    #[test]
    fn test_parse_command_with_arguments() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("hello Alice").unwrap();
        assert_eq!(parsed.command_name, "hello");
        assert_eq!(parsed.arguments.get("name"), Some(&"Alice".to_string()));
    }

    #[test]
    fn test_parse_command_with_options() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("hello --loud").unwrap();
        assert_eq!(parsed.command_name, "hello");
        assert_eq!(parsed.arguments.get("loud"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_command_with_short_option() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("hello -l").unwrap();
        assert_eq!(parsed.command_name, "hello");
        assert_eq!(parsed.arguments.get("loud"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_command_with_multiple_arguments_and_options() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("process input.txt output.txt --verbose").unwrap();
        assert_eq!(parsed.command_name, "process");
        assert_eq!(parsed.arguments.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(parsed.arguments.get("output"), Some(&"output.txt".to_string()));
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_alias_with_arguments() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("proc input.txt -v").unwrap();
        assert_eq!(parsed.command_name, "process");
        assert_eq!(parsed.arguments.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
    }

    // ========================================================================
    // Quoted argument tests
    // ========================================================================

    #[test]
    fn test_parse_quoted_arguments() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line(r#"hello "Alice Bob""#).unwrap();
        assert_eq!(parsed.command_name, "hello");
        assert_eq!(parsed.arguments.get("name"), Some(&"Alice Bob".to_string()));
    }

    #[test]
    fn test_parse_quoted_paths() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line(r#"process "/path/with spaces/file.txt""#).unwrap();
        assert_eq!(parsed.command_name, "process");
        assert_eq!(
            parsed.arguments.get("input"),
            Some(&"/path/with spaces/file.txt".to_string())
        );
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_parse_complex_command_line() {
        let registry = create_test_registry();
        let parser = ReplParser::new(&registry);

        let parsed = parser
            .parse_line(r#"proc "input file.txt" "output file.txt" -v"#)
            .unwrap();

        assert_eq!(parsed.command_name, "process");
        assert_eq!(parsed.arguments.get("input"), Some(&"input file.txt".to_string()));
        assert_eq!(
            parsed.arguments.get("output"),
            Some(&"output file.txt".to_string())
        );
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parsed_command_debug() {
        let mut args = HashMap::new();
        args.insert("test".to_string(), "value".to_string());

        let parsed = ParsedCommand {
            command_name: "test".to_string(),
            arguments: args,
        };

        // Verify Debug trait works
        let debug_str = format!("{:?}", parsed);
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_parsed_command_clone() {
        let mut args = HashMap::new();
        args.insert("test".to_string(), "value".to_string());

        let parsed = ParsedCommand {
            command_name: "test".to_string(),
            arguments: args,
        };

        let cloned = parsed.clone();
        assert_eq!(parsed, cloned);
    }
}
