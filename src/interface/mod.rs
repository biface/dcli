//! User interface module
//!
//! This module provides two main interfaces for interacting with the CLI framework:
//!
//! - [`CliInterface`]: One-shot command execution from command-line arguments
//! - [`ReplInterface`]: Interactive REPL (Read-Eval-Print Loop) with history
//!
//! # Overview
//!
//! The `interface` module is the user-facing layer of the framework. It handles:
//! - Parsing user input (CLI args or REPL lines)
//! - Executing commands through the registry
//! - Displaying results and errors
//! - Managing command history (REPL only)
//!
//! # Choosing an Interface
//!
//! ## CLI Interface
//!
//! Use [`CliInterface`] when:
//! - Running single commands from scripts
//! - Building traditional CLI tools
//! - No interaction is needed
//! - Each invocation is independent
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
//!
//! ## REPL Interface
//!
//! Use [`ReplInterface`] when:
//! - Building interactive tools
//! - Users need to run multiple commands
//! - Context/state is preserved between commands
//! - Command history and line editing are desired
//!
//! ```no_run
//! use dynamic_cli::interface::ReplInterface;
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
//! let repl = ReplInterface::new(registry, context, "myapp".to_string())?;
//! repl.run()?; // Enters interactive loop
//! # Ok(())
//! # }
//! ```
//!
//! # Architecture
//!
//! Both interfaces follow the same flow:
//!
//! ```text
//! User Input → Parser → Validator → Executor → Handler
//!                                        ↓
//!                                  ExecutionContext
//! ```
//!
//! **Key differences**:
//!
//! | Aspect | CLI | REPL |
//! |--------|-----|------|
//! | Input | Command-line args | Interactive lines |
//! | Parser | [`CliParser`] | [`ReplParser`] |
//! | History | None | Persistent to disk |
//! | Errors | Exit process | Display and continue |
//! | Lifecycle | One command, exit | Loop until user quits |
//!
//! # Error Handling
//!
//! ## CLI Interface
//!
//! Errors cause the process to exit with specific codes:
//! - `0`: Success
//! - `1`: Execution error
//! - `2`: Parse/validation error
//! - `3`: Other errors
//!
//! ## REPL Interface
//!
//! Errors are displayed but the REPL continues:
//! - Parse errors → show suggestions, continue
//! - Validation errors → explain issue, continue
//! - Execution errors → display error, continue
//! - Critical errors → exit REPL
//!
//! # Examples
//!
//! ## Complete CLI Application
//!
//! ```no_run
//! use dynamic_cli::prelude::*;
//! use dynamic_cli::config::loader::load_config;
//! use dynamic_cli::interface::CliInterface;
//! use std::collections::HashMap;
//!
//! // Define context
//! #[derive(Default)]
//! struct AppContext {
//!     data: Vec<String>,
//! }
//!
//! impl ExecutionContext for AppContext {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! }
//!
//! // Define handler
//! struct AddCommand;
//!
//! impl CommandHandler for AddCommand {
//!     fn execute(
//!         &self,
//!         context: &mut dyn ExecutionContext,
//!         args: &HashMap<String, String>,
//!     ) -> dynamic_cli::Result<()> {
//!         let ctx = dynamic_cli::context::downcast_mut::<AppContext>(context).unwrap();
//!         let item = args.get("item").unwrap();
//!         ctx.data.push(item.clone());
//!         println!("Added: {}", item);
//!         Ok(())
//!     }
//! }
//!
//! fn main() -> dynamic_cli::Result<()> {
//!     // Load configuration
//!     let config = load_config("commands.yaml")?;
//!     
//!     // Build registry
//!     let mut registry = CommandRegistry::new();
//!     registry.register(
//!         config.commands[0].clone(),
//!         Box::new(AddCommand),
//!     )?;
//!     
//!     // Create and run CLI
//!     let context = Box::new(AppContext::default());
//!     let cli = CliInterface::new(registry, context);
//!     cli.run(std::env::args().skip(1).collect())
//! }
//! ```
//!
//! ## Complete REPL Application
//!
//! ```no_run
//! use dynamic_cli::prelude::*;
//! use dynamic_cli::config::loader::load_config;
//! use dynamic_cli::interface::ReplInterface;
//! use std::collections::HashMap;
//!
//! // Same context and handler as above
//! # #[derive(Default)]
//! # struct AppContext { data: Vec<String> }
//! # impl ExecutionContext for AppContext {
//! #     fn as_any(&self) -> &dyn std::any::Any { self }
//! #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! # }
//! # struct AddCommand;
//! # impl CommandHandler for AddCommand {
//! #     fn execute(&self, context: &mut dyn ExecutionContext, args: &HashMap<String, String>) -> dynamic_cli::Result<()> {
//! #         let ctx = dynamic_cli::context::downcast_mut::<AppContext>(context).unwrap();
//! #         let item = args.get("item").unwrap();
//! #         ctx.data.push(item.clone());
//! #         println!("Added: {}", item);
//! #         Ok(())
//! #     }
//! # }
//!
//! fn main() -> dynamic_cli::Result<()> {
//!     // Load configuration
//!     let config = load_config("commands.yaml")?;
//!     
//!     // Build registry
//!     let mut registry = CommandRegistry::new();
//!     registry.register(
//!         config.commands[0].clone(),
//!         Box::new(AddCommand),
//!     )?;
//!     
//!     // Create and run REPL
//!     let context = Box::new(AppContext::default());
//!     let repl = ReplInterface::new(registry, context, "myapp".to_string())?;
//!     repl.run() // Interactive loop
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`cli`]: CLI interface implementation
//! - [`repl`]: REPL interface implementation
//!
//! [`CliParser`]: crate::parser::CliParser
//! [`ReplParser`]: crate::parser::ReplParser

pub mod cli;
pub mod repl;

// Re-export main types for convenience
pub use cli::CliInterface;
pub use repl::ReplInterface;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::CommandDefinition;
    use crate::prelude::*;
    use std::collections::HashMap;

    // Test context
    #[derive(Default)]
    struct TestContext;

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // Test handler
    struct TestHandler;

    impl CommandHandler for TestHandler {
        fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> crate::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_module_imports() {
        // Verify that types are re-exported
        let _: Option<CliInterface> = None;
        let _: Option<ReplInterface> = None;
    }

    #[test]
    fn test_cli_interface_accessible() {
        let mut registry = CommandRegistry::new();

        let cmd_def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec![],
            description: "Test".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test".to_string(),
        };

        registry.register(cmd_def, Box::new(TestHandler)).unwrap();

        let context = Box::new(TestContext::default());
        let _cli = CliInterface::new(registry, context);
    }

    #[test]
    fn test_repl_interface_accessible() {
        let mut registry = CommandRegistry::new();

        let cmd_def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec![],
            description: "Test".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test".to_string(),
        };

        registry.register(cmd_def, Box::new(TestHandler)).unwrap();

        let context = Box::new(TestContext::default());
        let _repl = ReplInterface::new(registry, context, "test".to_string());
    }
}
