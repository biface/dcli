//! Command-line and REPL parsing
//!
//! This module provides comprehensive parsing functionality for both
//! traditional command-line interfaces (CLI) and interactive REPL mode.
//!
//! # Module Structure
//!
//! The parser module consists of three main components:
//!
//! - [`type_parser`]: Type conversion functions (string → typed values)
//! - [`cli_parser`]: CLI argument parser (Unix-style options)
//! - [`repl_parser`]: REPL line parser (interactive mode)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         User Input                      │
//! │   "process file.txt --verbose"          │
//! └──────────────┬──────────────────────────┘
//!                │
//!                ▼
//! ┌─────────────────────────────────────────┐
//! │       ReplParser (REPL mode)            │
//! │   - Tokenize line                       │
//! │   - Resolve command name via Registry   │
//! │   - Delegate to CliParser               │
//! └──────────────┬──────────────────────────┘
//!                │
//!                ▼
//! ┌─────────────────────────────────────────┐
//! │       CliParser (CLI mode)              │
//! │   - Parse positional arguments          │
//! │   - Parse options (-v, --verbose)       │
//! │   - Apply defaults                      │
//! │   - Use TypeParser for conversion       │
//! └──────────────┬──────────────────────────┘
//!                │
//!                ▼
//! ┌─────────────────────────────────────────┐
//! │       TypeParser                        │
//! │   - Convert strings to typed values     │
//! │   - Validate type constraints           │
//! └──────────────┬──────────────────────────┘
//!                │
//!                ▼
//! ┌─────────────────────────────────────────┐
//! │   HashMap<String, String>               │
//! │   {"input": "file.txt",                 │
//! │    "verbose": "true"}                   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Design Principles
//!
//! ## 1. Separation of Concerns
//!
//! Each parser has a specific responsibility:
//! - **TypeParser**: Handles type conversion only
//! - **CliParser**: Handles CLI syntax (options, arguments)
//! - **ReplParser**: Handles REPL-specific concerns (tokenization, command resolution)
//!
//! ## 2. Composability
//!
//! Parsers compose naturally:
//! - ReplParser uses CliParser for argument parsing
//! - CliParser uses TypeParser for type conversion
//! - Each can be used independently when needed
//!
//! ## 3. Error Clarity
//!
//! All parsers provide detailed error messages with:
//! - Clear descriptions of what went wrong
//! - Suggestions for typos (via Levenshtein distance)
//! - Hints for correct usage
//!
//! # Usage Examples
//!
//! ## CLI Mode (Direct Argument Parsing)
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
//! let args = vec!["input.txt".to_string()];
//! let parsed = parser.parse(&args).unwrap();
//!
//! assert_eq!(parsed.get("input"), Some(&"input.txt".to_string()));
//! ```
//!
//! ## REPL Mode (Interactive Parsing)
//!
//! ```no_run
//! use dynamic_cli::parser::repl_parser::ReplParser;
//! use dynamic_cli::registry::CommandRegistry;
//!
//! let registry = CommandRegistry::new();
//! // ... register commands ...
//!
//! let parser = ReplParser::new(&registry);
//!
//! // Parse user input
//! let line = "process input.txt --verbose";
//! let parsed = parser.parse_line(line).unwrap();
//!
//! println!("Command: {}", parsed.command_name);
//! println!("Arguments: {:?}", parsed.arguments);
//! ```
//!
//! ## Type Parsing (Low-Level)
//!
//! ```
//! use dynamic_cli::parser::type_parser::{parse_integer, parse_bool};
//!
//! let number = parse_integer("42").unwrap();
//! assert_eq!(number, 42);
//!
//! let flag = parse_bool("yes").unwrap();
//! assert_eq!(flag, true);
//! ```
//!
//! # Error Handling
//!
//! All parsing functions return [`Result<T>`] where errors are instances
//! of [`ParseError`]. Common error scenarios:
//!
//! - **Unknown command**: User typed a non-existent command
//!   ```text
//!   Error: Unknown command: 'simulat'
//!   ? Did you mean:
//!     • simulate
//!     • validation
//!   ```
//!
//! - **Type mismatch**: Value cannot be converted to expected type
//!   ```text
//!   Error: Failed to parse count as integer: 'abc'
//!   ```
//!
//! - **Missing argument**: Required argument not provided
//!   ```text
//!   Error: Missing required argument: input for command 'process'
//!   ```
//!
//! # Performance Considerations
//!
//! - **Type parsing**: O(1) for most types, O(n) for string length
//! - **CLI parsing**: O(n) where n = number of arguments
//! - **REPL parsing**: O(m + n) where m = line length (tokenization), n = arguments
//! - **Command resolution**: O(1) via HashMap lookup in registry
//!
//! # Thread Safety
//!
//! All parsers are:
//! - **Stateless**: Can be used concurrently from multiple threads
//! - **Borrowing**: Use references to definitions/registry (no ownership)
//! - **Reusable**: Can parse multiple commands with the same parser instance
//!
//! # Future Extensions
//!
//! Potential enhancements for future versions:
//! - Support for subcommands (e.g., `git commit`)
//! - Environment variable expansion
//! - Glob pattern matching for paths
//! - Command history and auto-completion hints
//! - Streaming parser for very large inputs

use crate::error::Result;

// Public submodules
pub mod cli_parser;
pub mod repl_parser;
pub mod type_parser;

// Re-export commonly used types
pub use cli_parser::CliParser;
pub use repl_parser::{ParsedCommand, ReplParser};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{
        ArgumentDefinition, ArgumentType, CommandDefinition, OptionDefinition,
    };
    use crate::context::ExecutionContext;
    use crate::executor::CommandHandler;
    use crate::registry::CommandRegistry;
    use std::collections::HashMap;

    // Dummy handler for integration tests
    struct IntegrationTestHandler;

    impl CommandHandler for IntegrationTestHandler {
        fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            Ok(())
        }
    }

    /// Helper to create a comprehensive test command
    fn create_comprehensive_command() -> CommandDefinition {
        CommandDefinition {
            name: "analyze".to_string(),
            aliases: vec!["analyse".to_string(), "check".to_string()],
            description: "Analyze data files".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "input".to_string(),
                    arg_type: ArgumentType::Path,
                    required: true,
                    description: "Input data file".to_string(),
                    validation: vec![],
                },
                ArgumentDefinition {
                    name: "output".to_string(),
                    arg_type: ArgumentType::Path,
                    required: false,
                    description: "Output report file".to_string(),
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
                    description: "Enable verbose output".to_string(),
                    choices: vec![],
                },
                OptionDefinition {
                    name: "iterations".to_string(),
                    short: Some("i".to_string()),
                    long: Some("iterations".to_string()),
                    option_type: ArgumentType::Integer,
                    required: false,
                    default: Some("100".to_string()),
                    description: "Number of iterations".to_string(),
                    choices: vec![],
                },
                OptionDefinition {
                    name: "threshold".to_string(),
                    short: Some("t".to_string()),
                    long: Some("threshold".to_string()),
                    option_type: ArgumentType::Float,
                    required: false,
                    default: Some("0.5".to_string()),
                    description: "Analysis threshold".to_string(),
                    choices: vec![],
                },
            ],
            implementation: "analyze_handler".to_string(),
        }
    }

    // ========================================================================
    // Integration tests: CLI Parser
    // ========================================================================

    #[test]
    fn test_cli_parser_integration_minimal() {
        let definition = create_comprehensive_command();
        let parser = CliParser::new(&definition);

        let args = vec!["data.csv".to_string()];
        let result = parser.parse(&args).unwrap();

        // Required argument
        assert_eq!(result.get("input"), Some(&"data.csv".to_string()));

        // Defaults should be applied
        assert_eq!(result.get("verbose"), Some(&"false".to_string()));
        assert_eq!(result.get("iterations"), Some(&"100".to_string()));
        assert_eq!(result.get("threshold"), Some(&"0.5".to_string()));
    }

    #[test]
    fn test_cli_parser_integration_full() {
        let definition = create_comprehensive_command();
        let parser = CliParser::new(&definition);

        let args = vec![
            "data.csv".to_string(),
            "report.txt".to_string(),
            "--verbose".to_string(),
            "--iterations=200".to_string(),
            "-t".to_string(),
            "0.75".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"data.csv".to_string()));
        assert_eq!(result.get("output"), Some(&"report.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("iterations"), Some(&"200".to_string()));
        assert_eq!(result.get("threshold"), Some(&"0.75".to_string()));
    }

    #[test]
    fn test_cli_parser_integration_mixed_options() {
        let definition = create_comprehensive_command();
        let parser = CliParser::new(&definition);

        // Options can be interspersed with positional arguments
        let args = vec![
            "--verbose".to_string(),
            "data.csv".to_string(),
            "-i200".to_string(),
            "report.txt".to_string(),
            "--threshold".to_string(),
            "0.9".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"data.csv".to_string()));
        assert_eq!(result.get("output"), Some(&"report.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("iterations"), Some(&"200".to_string()));
        assert_eq!(result.get("threshold"), Some(&"0.9".to_string()));
    }

    // ========================================================================
    // Integration tests: REPL Parser
    // ========================================================================

    #[test]
    fn test_repl_parser_integration_simple() {
        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        let parsed = parser.parse_line("analyze data.csv").unwrap();
        assert_eq!(parsed.command_name, "analyze");
        assert_eq!(parsed.arguments.get("input"), Some(&"data.csv".to_string()));
    }

    #[test]
    fn test_repl_parser_integration_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        // Use alias instead of command name
        let parsed = parser.parse_line("check data.csv --verbose").unwrap();
        assert_eq!(parsed.command_name, "analyze"); // Resolves to canonical name
        assert_eq!(parsed.arguments.get("input"), Some(&"data.csv".to_string()));
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_repl_parser_integration_quoted_paths() {
        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        let parsed = parser
            .parse_line(r#"analyze "/path/with spaces/data.csv" "output report.txt""#)
            .unwrap();

        assert_eq!(
            parsed.arguments.get("input"),
            Some(&"/path/with spaces/data.csv".to_string())
        );
        assert_eq!(
            parsed.arguments.get("output"),
            Some(&"output report.txt".to_string())
        );
    }

    #[test]
    fn test_repl_parser_integration_complex() {
        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        let parsed = parser
            .parse_line(r#"analyse "data file.csv" report.txt -v --iterations=500 -t 0.95"#)
            .unwrap();

        assert_eq!(parsed.command_name, "analyze");
        assert_eq!(
            parsed.arguments.get("input"),
            Some(&"data file.csv".to_string())
        );
        assert_eq!(
            parsed.arguments.get("output"),
            Some(&"report.txt".to_string())
        );
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
        assert_eq!(parsed.arguments.get("iterations"), Some(&"500".to_string()));
        assert_eq!(parsed.arguments.get("threshold"), Some(&"0.95".to_string()));
    }

    // ========================================================================
    // Integration tests: Type Parser
    // ========================================================================

    #[test]
    fn test_type_parser_integration_all_types() {
        use type_parser::parse_value;

        // Test all argument types
        assert!(parse_value("hello", ArgumentType::String).is_ok());
        assert!(parse_value("42", ArgumentType::Integer).is_ok());
        assert!(parse_value("3.14", ArgumentType::Float).is_ok());
        assert!(parse_value("true", ArgumentType::Bool).is_ok());
        assert!(parse_value("/path/to/file", ArgumentType::Path).is_ok());
    }

    #[test]
    fn test_type_parser_integration_error_propagation() {
        let definition = create_comprehensive_command();
        let parser = CliParser::new(&definition);

        // Invalid integer should fail
        let args = vec![
            "data.csv".to_string(),
            "--iterations".to_string(),
            "not_a_number".to_string(),
        ];

        let result = parser.parse(&args);
        assert!(result.is_err());
    }

    // ========================================================================
    // Integration tests: End-to-End Workflows
    // ========================================================================

    #[test]
    fn test_workflow_cli_to_execution() {
        // Simulate: User provides CLI args → Parser → Handler could execute

        let definition = create_comprehensive_command();
        let parser = CliParser::new(&definition);

        let args = vec!["data.csv".to_string(), "-v".to_string()];
        let parsed = parser.parse(&args).unwrap();

        // Verify parsed data is ready for execution
        assert!(parsed.contains_key("input"));
        assert!(parsed.contains_key("verbose"));
        assert_eq!(parsed.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_workflow_repl_to_execution() {
        // Simulate: User types in REPL → Parser → Handler could execute

        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        let line = "analyze data.csv --verbose --iterations=1000";
        let parsed = parser.parse_line(line).unwrap();

        // Verify parsed command is ready for execution
        assert_eq!(parsed.command_name, "analyze");
        assert!(parsed.arguments.contains_key("input"));
        assert_eq!(parsed.arguments.get("verbose"), Some(&"true".to_string()));
        assert_eq!(
            parsed.arguments.get("iterations"),
            Some(&"1000".to_string())
        );
    }

    #[test]
    fn test_workflow_typo_suggestions() {
        let mut registry = CommandRegistry::new();
        let definition = create_comprehensive_command();
        registry
            .register(definition, Box::new(IntegrationTestHandler))
            .unwrap();

        let parser = ReplParser::new(&registry);

        // User makes a typo
        let result = parser.parse_line("analyz data.csv");

        assert!(result.is_err());

        // Error should contain suggestions
        let error = result.unwrap_err();
        let error_msg = format!("{}", error);
        assert!(error_msg.contains("Unknown command"));
    }

    // ========================================================================
    // Re-export verification tests
    // ========================================================================

    #[test]
    fn test_reexports_accessible() {
        // Verify that re-exported types are accessible from module root

        let definition = create_comprehensive_command();

        // CliParser should be accessible
        let _cli_parser = CliParser::new(&definition);

        // ReplParser should be accessible (needs registry)
        let registry = CommandRegistry::new();
        let _repl_parser = ReplParser::new(&registry);

        // ParsedCommand should be accessible
        let _parsed = ParsedCommand {
            command_name: "test".to_string(),
            arguments: HashMap::new(),
        };
    }
}
