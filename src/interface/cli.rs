//! CLI (Command-Line Interface) implementation
//!
//! This module provides a simple CLI interface that parses command-line
//! arguments, executes the corresponding command, and exits.
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::interface::CliInterface;
//! use dynamic_cli::prelude::*;
//!
//! # #[derive(Default)]
//! # struct MyContext;
//! # impl ExecutionContext for MyContext {
//! #     fn as_any(&self) -> &dyn std::any::Any { self }
//! #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! # }
//! # fn main() -> dynamic_cli::Result<()> {
//! let registry = CommandRegistry::new();
//! let context = Box::new(MyContext::default());
//!
//! let cli = CliInterface::new(registry, context);
//! cli.run(std::env::args().skip(1).collect())?;
//! # Ok(())
//! # }
//! ```

use crate::context::ExecutionContext;
use crate::error::{display_error, DynamicCliError, Result};
use crate::parser::CliParser;
use crate::registry::CommandRegistry;
use std::process;

/// CLI (Command-Line Interface) handler
///
/// Provides a simple interface for executing commands from command-line arguments.
/// The CLI parses arguments, executes the command, and exits.
///
/// # Architecture
///
/// ```text
/// Command-line args → CliParser → CommandExecutor → Handler
///                                       ↓
///                                  ExecutionContext
/// ```
///
/// # Error Handling
///
/// Errors are displayed to stderr with colored formatting (if enabled)
/// and the process exits with appropriate exit codes:
/// - `0`: Success
/// - `1`: Execution error
/// - `2`: Argument parsing error
/// - `3`: Other errors
pub struct CliInterface {
    /// Command registry containing all available commands
    registry: CommandRegistry,
    
    /// Execution context (owned by the interface)
    context: Box<dyn ExecutionContext>,
}

impl CliInterface {
    /// Create a new CLI interface
    ///
    /// # Arguments
    ///
    /// * `registry` - Command registry with all registered commands
    /// * `context` - Execution context (will be consumed by the interface)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    ///
    /// let cli = CliInterface::new(registry, context);
    /// ```
    pub fn new(registry: CommandRegistry, context: Box<dyn ExecutionContext>) -> Self {
        Self { registry, context }
    }
    
    /// Run the CLI with provided arguments
    ///
    /// Parses the arguments, executes the corresponding command, and handles errors.
    /// This method consumes `self` as the CLI typically runs once and exits.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (typically from `env::args().skip(1)`)
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err(DynamicCliError)` on any error (parsing, validation, execution)
    ///
    /// # Exit Codes
    ///
    /// The caller should handle errors and exit with appropriate codes:
    /// - Parse errors → exit code 2
    /// - Execution errors → exit code 1
    /// - Other errors → exit code 3
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    /// use std::process;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    /// let cli = CliInterface::new(registry, context);
    ///
    /// if let Err(e) = cli.run(std::env::args().skip(1).collect()) {
    ///     eprintln!("Error: {}", e);
    ///     process::exit(1);
    /// }
    /// # }
    /// ```
    pub fn run(mut self, args: Vec<String>) -> Result<()> {
        // Handle empty arguments (show help or error)
        if args.is_empty() {
            return Err(DynamicCliError::Parse(
                crate::error::ParseError::InvalidSyntax {
                    details: "No command specified".to_string(),
                    hint: Some("Try 'help' to see available commands".to_string()),
                }
            ));
        }
        
        // First argument is the command name
        let command_name = &args[0];
        
        // Resolve command name (handles aliases)
        let resolved_name = self.registry.resolve_name(command_name)
            .ok_or_else(|| {
                crate::error::ParseError::unknown_command_with_suggestions(
                    command_name,
                    &self.registry.list_commands()
                        .iter()
                        .map(|cmd| cmd.name.clone())
                        .collect::<Vec<_>>(),
                )
            })?;
        
        // Get command definition
        let definition = self.registry.get_definition(resolved_name)
            .ok_or_else(|| {
                DynamicCliError::Registry(crate::error::RegistryError::MissingHandler {
                    command: resolved_name.to_string(),
                })
            })?;
        
        // Parse arguments using CLI parser
        let parser = CliParser::new(definition);
        let parsed_args = parser.parse(&args[1..])?;
        
        // Get handler and execute command
        let handler = self.registry.get_handler(resolved_name)
            .ok_or_else(|| {
                DynamicCliError::Execution(crate::error::ExecutionError::HandlerNotFound {
                    command: resolved_name.to_string(),
                    implementation: definition.implementation.clone(),
                })
            })?;
        
        handler.execute(&mut *self.context, &parsed_args)?;
        
        Ok(())
    }
    
    /// Run the CLI with automatic error handling and exit
    ///
    /// This is a convenience method that:
    /// 1. Runs the CLI with provided arguments
    /// 2. Handles errors by displaying them to stderr
    /// 3. Exits the process with appropriate exit code
    ///
    /// This method never returns.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    /// let cli = CliInterface::new(registry, context);
    ///
    /// // This will handle errors and exit automatically
    /// cli.run_and_exit(std::env::args().skip(1).collect());
    /// # }
    /// ```
    pub fn run_and_exit(self, args: Vec<String>) -> ! {
        match self.run(args) {
            Ok(()) => process::exit(0),
            Err(e) => {
                display_error(&e);
                
                // Exit with appropriate code based on error type
                let exit_code = match e {
                    DynamicCliError::Parse(_) => 2,
                    DynamicCliError::Validation(_) => 2,
                    DynamicCliError::Execution(_) => 1,
                    _ => 3,
                };
                
                process::exit(exit_code);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, CommandDefinition};
    use std::collections::HashMap;
    
    // Test context
    #[derive(Default)]
    struct TestContext {
        executed_command: Option<String>,
    }
    
    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    
    // Test handler
    struct TestHandler {
        name: String,
    }
    
    impl crate::executor::CommandHandler for TestHandler {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .expect("Failed to downcast context");
            ctx.executed_command = Some(self.name.clone());
            Ok(())
        }
    }
    
    fn create_test_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();
        
        // Create a simple command definition
        let cmd_def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec!["t".to_string()],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test_handler".to_string(),
        };
        
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });
        
        registry.register(cmd_def, handler).expect("Failed to register command");
        
        registry
    }

    #[test]
    fn test_cli_interface_creation() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        
        let _cli = CliInterface::new(registry, context);
        // If this compiles and runs, creation works
    }

    #[test]
    fn test_cli_run_simple_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);
        
        let result = cli.run(vec!["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_run_with_alias() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);
        
        let result = cli.run(vec!["t".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_empty_args() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);
        
        let result = cli.run(vec![]);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::InvalidSyntax { .. }) => {},
            other => panic!("Expected InvalidSyntax error, got: {:?}", other),
        }
    }

    #[test]
    fn test_cli_unknown_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);
        
        let result = cli.run(vec!["unknown".to_string()]);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::UnknownCommand { .. }) => {},
            other => panic!("Expected UnknownCommand error, got: {:?}", other),
        }
    }

    #[test]
    fn test_cli_command_with_args() {
        let mut registry = CommandRegistry::new();
        
        // Command with argument
        let cmd_def = CommandDefinition {
            name: "greet".to_string(),
            aliases: vec![],
            description: "Greet someone".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "name".to_string(),
                    arg_type: ArgumentType::String,
                    required: true,
                    description: "Name to greet".to_string(),
                    validation: vec![],
                }
            ],
            options: vec![],
            implementation: "greet_handler".to_string(),
        };
        
        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(
                &self,
                _context: &mut dyn ExecutionContext,
                args: &HashMap<String, String>,
            ) -> Result<()> {
                assert_eq!(args.get("name"), Some(&"Alice".to_string()));
                Ok(())
            }
        }
        
        registry.register(cmd_def, Box::new(GreetHandler)).unwrap();
        
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);
        
        let result = cli.run(vec!["greet".to_string(), "Alice".to_string()]);
        assert!(result.is_ok());
    }
}
