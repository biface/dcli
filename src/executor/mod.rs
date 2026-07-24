//! Command execution module
//!
//! This module provides the core functionality for executing commands in the
//! dynamic-cli framework. It defines the [`CommandHandler`] trait that all
//! command implementations must satisfy.
//!
//! # Module Organization
//!
//! - [`traits`]: Core trait definitions (`CommandHandler`)
//! - `command_executor` (future): Executor logic for running commands
//!
//! # Architecture
//!
//! The execution flow in dynamic-cli follows this pattern:
//!
//! ```text
//! User Input → Parser → Validator → Executor → Command Handler
//!                                      ↓
//!                                  Context
//! ```
//!
//! 1. **Parser**: Converts raw input to structured arguments
//! 2. **Validator**: Checks argument types and constraints
//! 3. **Executor**: Looks up and invokes the appropriate handler
//! 4. **Handler**: Executes the command logic with access to context
//!
//! # Design Philosophy
//!
//! ## Object Safety
//!
//! The module is designed around object-safe traits to enable dynamic dispatch.
//! This allows:
//! - Runtime registration of commands
//! - Storing heterogeneous handlers in collections
//! - Plugin-style architecture where handlers are loaded dynamically
//!
//! ## Thread Safety
//!
//! All types are `Send + Sync` to support:
//! - Multi-threaded CLI applications
//! - Concurrent command execution (future enhancement)
//! - Safe shared access to the command registry
//!
//! ## Simplicity
//!
//! The API is intentionally kept simple:
//! - Arguments are passed as [`crate::parser::ParsedArgs`]
//! - Context is accessed through trait objects
//! - Error handling uses the framework's standard `Result` type
//!
//! # Quick Start
//!
//! ```
//! use dynamic_cli::error::ExecutionError;
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::Result;
//! use std::collections::HashMap;
//!
//! // 1. Define your context
//! #[derive(Default)]
//! struct AppContext {
//!     counter: i32,
//! }
//!
//! impl ExecutionContext for AppContext {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! }
//!
//! // 2. Implement a command handler
//! struct IncrementCommand;
//!
//! impl CommandHandler for IncrementCommand {
//!     fn execute(
//!         &self,
//!         context: &mut dyn ExecutionContext,
//!         args: &ParsedArgs,
//!     ) -> Result<()> {
//!         let ctx = dynamic_cli::context::downcast_mut::<AppContext>(context)
//!             .ok_or_else(|| ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type")))?;
//!         
//!         let amount: i32 = args.get_scalar("amount")
//!             .and_then(|s| s.parse().ok())
//!             .unwrap_or(1);
//!         
//!         ctx.counter += amount;
//!         println!("Counter is now: {}", ctx.counter);
//!         Ok(())
//!     }
//! }
//!
//! // 3. Use the handler
//! # fn main() -> Result<()> {
//! let handler = IncrementCommand;
//! let mut context = AppContext::default();
//! let mut raw_args = HashMap::new();
//! raw_args.insert("amount".to_string(), "5".to_string());
//! let args = ParsedArgs::from_scalars(raw_args);
//!
//! handler.execute(&mut context, &args)?;
//! assert_eq!(context.counter, 5);
//! # Ok(())
//! # }
//! ```
//!
//! # Examples
//!
//! ## Basic Command
//!
//! ```
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::Result;
//!
//! struct EchoCommand;
//!
//! impl CommandHandler for EchoCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &ParsedArgs,
//!     ) -> Result<()> {
//!         if let Some(message) = args.get_scalar("message") {
//!             println!("{}", message);
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Command with Validation
//!
//! ```
//! use dynamic_cli::error::ExecutionError;
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::DynamicCliError::Execution;
//! use dynamic_cli::Result;
//!
//! struct DivideCommand;
//!
//! impl CommandHandler for DivideCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &ParsedArgs,
//!     ) -> dynamic_cli::Result<()> {
//!         let denom = args.get_scalar("denominator")
//!             .ok_or_else(|| {
//!                 ExecutionError::CommandFailed(
//!                     anyhow::anyhow!("Missing Denominator"))})?;
//!
//!         let value: f64 = denom.parse()
//!             .map_err(|_| {
//!                 ExecutionError::CommandFailed(
//!                     anyhow::anyhow!("Invalid Denominator"))})?;
//!
//!         if value == 0.0 {
//!             return Err(ExecutionError::CommandFailed(
//!                     anyhow::anyhow!("Cannot divide by zero")).into());
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Stateful Command
//!
//! ```
//! use dynamic_cli::error::ExecutionError;
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::Result;
//!
//! #[derive(Default)]
//! struct FileContext {
//!     current_file: Option<String>,
//! }
//!
//! impl ExecutionContext for FileContext {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! }
//!
//! struct OpenCommand;
//!
//! impl CommandHandler for OpenCommand {
//!     fn execute(
//!         &self,
//!         context: &mut dyn ExecutionContext,
//!         args: &ParsedArgs,
//!     ) -> Result<()> {
//!         let ctx = dynamic_cli::context::downcast_mut::<FileContext>(context)
//!             .ok_or_else(|| ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type")))?;
//!         
//!         let filename = args.get_scalar("file")
//!             .ok_or_else(|| { ExecutionError::CommandFailed(anyhow::anyhow!("Missing file argument"))})?;
//!         
//!         ctx.current_file = Some(filename.to_string());
//!         println!("Opened: {}", filename);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! # Advanced Usage
//!
//! ## Dynamic Command Registration
//!
//! Commands can be registered dynamically at runtime using trait objects:
//!
//! ```
//! use std::collections::HashMap;
//! use dynamic_cli::executor::CommandHandler;
//! # use dynamic_cli::context::ExecutionContext;
//! # use dynamic_cli::Result;
//!
//! // Store commands in a registry
//! struct CommandRegistry {
//!     handlers: HashMap<String, Box<dyn CommandHandler>>,
//! }
//!
//! impl CommandRegistry {
//!     fn new() -> Self {
//!         Self {
//!             handlers: HashMap::new(),
//!         }
//!     }
//!     
//!     fn register(&mut self, name: String, handler: Box<dyn CommandHandler>) {
//!         self.handlers.insert(name, handler);
//!     }
//!     
//!     fn get(&self, name: &str) -> Option<&Box<dyn CommandHandler>> {
//!         self.handlers.get(name)
//!     }
//! }
//! ```
//!
//! ## Error Handling Pattern
//!
//! ```
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::error::ExecutionError;
//! use dynamic_cli::Result;
//!
//! struct FileCommand;
//!
//! impl CommandHandler for FileCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &ParsedArgs,
//!     ) -> Result<()> {
//!         let path = args.get_scalar("path")
//!             .ok_or_else(|| { ExecutionError::CommandFailed(anyhow::anyhow!("Missing path argument"))})?;
//!         
//!         // Wrap application errors in ExecutionError
//!         std::fs::read_to_string(path)
//!             .map_err(|e| ExecutionError::CommandFailed(
//!                 anyhow::anyhow!("Failed to read file: {}", e)
//!             ))?;
//!         
//!         Ok(())
//!     }
//! }
//! ```

// Public submodules
pub mod traits;

// Public re-exports for convenience
pub use crate::parser::ParsedArgs;
pub use traits::{AsyncCommandHandler, CommandHandler};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;
    use crate::error::ExecutionError;
    use std::any::Any;
    use std::collections::HashMap;

    // ============================================================================
    // INTEGRATION TEST FIXTURES
    // ============================================================================

    /// Test context for integration tests
    #[derive(Default)]
    struct IntegrationContext {
        log: Vec<String>,
        state: HashMap<String, String>,
    }

    impl ExecutionContext for IntegrationContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// Command that logs its execution
    struct LogCommand;

    impl CommandHandler for LogCommand {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            let ctx =
                crate::context::downcast_mut::<IntegrationContext>(context).ok_or_else(|| {
                    ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
                })?;

            let message = args.get_scalar("message").unwrap_or("default message");
            ctx.log.push(message.to_string());
            Ok(())
        }
    }

    /// Command that sets state
    struct SetCommand;

    impl CommandHandler for SetCommand {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            let ctx =
                crate::context::downcast_mut::<IntegrationContext>(context).ok_or_else(|| {
                    ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
                })?;

            if let (Some(key), Some(value)) = (args.get_scalar("key"), args.get_scalar("value")) {
                ctx.state.insert(key.to_string(), value.to_string());
            }
            Ok(())
        }
    }

    /// Command that reads state
    struct GetCommand;

    impl CommandHandler for GetCommand {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            let ctx =
                crate::context::downcast_mut::<IntegrationContext>(context).ok_or_else(|| {
                    ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
                })?;

            if let Some(key) = args.get_scalar("key") {
                if let Some(value) = ctx.state.get(key) {
                    ctx.log.push(format!("{} = {}", key, value));
                } else {
                    ctx.log.push(format!("{} not found", key));
                }
            }
            Ok(())
        }
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    #[test]
    fn test_module_reexports() {
        // Verify that CommandHandler is accessible from module root
        fn _accepts_handler(_: &dyn CommandHandler) {}

        let handler = LogCommand;
        _accepts_handler(&handler);
    }

    #[test]
    fn test_command_sequence() {
        // Test executing multiple commands in sequence
        let mut context = IntegrationContext::default();

        // Execute first command
        let log_cmd = LogCommand;
        let mut args1 = HashMap::new();
        args1.insert("message".to_string(), "First".to_string());
        let args1 = ParsedArgs::from_scalars(args1);
        log_cmd.execute(&mut context, &args1).unwrap();

        // Execute second command
        let mut args2 = HashMap::new();
        args2.insert("message".to_string(), "Second".to_string());
        let args2 = ParsedArgs::from_scalars(args2);
        log_cmd.execute(&mut context, &args2).unwrap();

        // Verify both commands executed
        assert_eq!(context.log.len(), 2);
        assert_eq!(context.log[0], "First");
        assert_eq!(context.log[1], "Second");
    }

    #[test]
    fn test_stateful_workflow() {
        // Test a complete workflow with state management
        let mut context = IntegrationContext::default();

        // Set some values
        let set_cmd = SetCommand;
        let mut args1 = HashMap::new();
        args1.insert("key".to_string(), "name".to_string());
        args1.insert("value".to_string(), "Alice".to_string());
        let args1 = ParsedArgs::from_scalars(args1);
        set_cmd.execute(&mut context, &args1).unwrap();

        let mut args2 = HashMap::new();
        args2.insert("key".to_string(), "age".to_string());
        args2.insert("value".to_string(), "30".to_string());
        let args2 = ParsedArgs::from_scalars(args2);
        set_cmd.execute(&mut context, &args2).unwrap();

        // Retrieve values
        let get_cmd = GetCommand;
        let mut args3 = HashMap::new();
        args3.insert("key".to_string(), "name".to_string());
        let args3 = ParsedArgs::from_scalars(args3);
        get_cmd.execute(&mut context, &args3).unwrap();

        let mut args4 = HashMap::new();
        args4.insert("key".to_string(), "age".to_string());
        let args4 = ParsedArgs::from_scalars(args4);
        get_cmd.execute(&mut context, &args4).unwrap();

        // Verify workflow
        assert_eq!(context.state.len(), 2);
        assert_eq!(context.state.get("name"), Some(&"Alice".to_string()));
        assert_eq!(context.state.get("age"), Some(&"30".to_string()));
        assert_eq!(context.log.len(), 2);
        assert_eq!(context.log[0], "name = Alice");
        assert_eq!(context.log[1], "age = 30");
    }

    #[test]
    fn test_heterogeneous_handler_collection() {
        // Test storing different handler types in a collection
        let handlers: Vec<Box<dyn CommandHandler>> = vec![
            Box::new(LogCommand),
            Box::new(SetCommand),
            Box::new(GetCommand),
        ];

        // Verify we can store different handlers
        assert_eq!(handlers.len(), 3);

        // Execute each handler
        let mut context = IntegrationContext::default();

        let mut args1 = HashMap::new();
        args1.insert("message".to_string(), "test".to_string());
        let args1 = ParsedArgs::from_scalars(args1);
        handlers[0].execute(&mut context, &args1).unwrap();

        let mut args2 = HashMap::new();
        args2.insert("key".to_string(), "k".to_string());
        args2.insert("value".to_string(), "v".to_string());
        let args2 = ParsedArgs::from_scalars(args2);
        handlers[1].execute(&mut context, &args2).unwrap();

        let mut args3 = HashMap::new();
        args3.insert("key".to_string(), "k".to_string());
        let args3 = ParsedArgs::from_scalars(args3);
        handlers[2].execute(&mut context, &args3).unwrap();

        assert_eq!(context.log.len(), 2);
        assert_eq!(context.state.len(), 1);
    }

    #[test]
    fn test_context_isolation_between_commands() {
        // Verify that context is shared correctly
        let mut context = IntegrationContext::default();

        let set_cmd = SetCommand;
        let mut args = HashMap::new();
        args.insert("key".to_string(), "shared".to_string());
        args.insert("value".to_string(), "value".to_string());
        let args = ParsedArgs::from_scalars(args);
        set_cmd.execute(&mut context, &args).unwrap();

        // Another command can see the state
        let get_cmd = GetCommand;
        let mut args2 = HashMap::new();
        args2.insert("key".to_string(), "shared".to_string());
        let args2 = ParsedArgs::from_scalars(args2);
        get_cmd.execute(&mut context, &args2).unwrap();

        assert!(context.log[0].contains("shared = value"));
    }

    #[test]
    fn test_error_propagation() {
        // Test that errors propagate correctly through the handler chain
        struct FailingCommand;

        impl CommandHandler for FailingCommand {
            fn execute(
                &self,
                _context: &mut dyn ExecutionContext,
                _args: &ParsedArgs,
            ) -> crate::error::Result<()> {
                Err(ExecutionError::CommandFailed(anyhow::anyhow!("Intentional failure")).into())
            }
        }

        let handler = FailingCommand;
        let mut context = IntegrationContext::default();
        let args = HashMap::new();
        let args = ParsedArgs::from_scalars(args);

        let result = handler.execute(&mut context, &args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Intentional failure"));
    }

    #[test]
    fn test_validation_before_execution() {
        // Test the intended workflow: validate then execute
        struct ValidatingCommand;

        impl CommandHandler for ValidatingCommand {
            fn execute(
                &self,
                context: &mut dyn ExecutionContext,
                _args: &ParsedArgs,
            ) -> crate::error::Result<()> {
                let ctx = crate::context::downcast_mut::<IntegrationContext>(context).ok_or_else(
                    || ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type")),
                )?;
                ctx.log.push("executed".to_string());
                Ok(())
            }

            fn validate(&self, args: &ParsedArgs) -> crate::error::Result<()> {
                if args.get_scalar("required").is_none() {
                    return Err(ExecutionError::CommandFailed(anyhow::anyhow!(
                        "Missing required argument"
                    ))
                    .into());
                }
                Ok(())
            }
        }

        let handler = ValidatingCommand;
        let mut context = IntegrationContext::default();

        // Test validation failure
        let args_invalid = HashMap::new();
        let args_invalid = ParsedArgs::from_scalars(args_invalid);
        assert!(handler.validate(&args_invalid).is_err());
        assert!(handler.execute(&mut context, &args_invalid).is_ok()); // Execute would work

        // Test validation success
        let mut args_valid = HashMap::new();
        args_valid.insert("required".to_string(), "value".to_string());
        let args_valid = ParsedArgs::from_scalars(args_valid);
        assert!(handler.validate(&args_valid).is_ok());
        assert!(handler.execute(&mut context, &args_valid).is_ok());
    }

    #[test]
    fn test_command_handler_documentation_example() {
        // Test the example from module documentation
        #[derive(Default)]
        struct AppContext {
            counter: i32,
        }

        impl ExecutionContext for AppContext {
            fn as_any(&self) -> &dyn Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        struct IncrementCommand;

        impl CommandHandler for IncrementCommand {
            fn execute(
                &self,
                context: &mut dyn ExecutionContext,
                args: &ParsedArgs,
            ) -> crate::error::Result<()> {
                let ctx = crate::context::downcast_mut::<AppContext>(context).ok_or_else(|| {
                    ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
                })?;

                let amount: i32 = args
                    .get_scalar("amount")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                ctx.counter += amount;
                Ok(())
            }
        }

        let handler = IncrementCommand;
        let mut context = AppContext::default();
        let mut args = HashMap::new();
        args.insert("amount".to_string(), "5".to_string());
        let args = ParsedArgs::from_scalars(args);

        handler.execute(&mut context, &args).unwrap();
        assert_eq!(context.counter, 5);
    }

    #[test]
    fn test_complex_multi_step_workflow() {
        // Test a more complex workflow simulating real usage
        let mut context = IntegrationContext::default();

        // Step 1: Initialize some state
        let set_cmd = SetCommand;
        let mut args1 = HashMap::new();
        args1.insert("key".to_string(), "initialized".to_string());
        args1.insert("value".to_string(), "true".to_string());
        let args1 = ParsedArgs::from_scalars(args1);
        set_cmd.execute(&mut context, &args1).unwrap();

        // Step 2: Log the initialization
        let log_cmd = LogCommand;
        let mut args2 = HashMap::new();
        args2.insert("message".to_string(), "System initialized".to_string());
        let args2 = ParsedArgs::from_scalars(args2);
        log_cmd.execute(&mut context, &args2).unwrap();

        // Step 3: Set more state
        let mut args3 = HashMap::new();
        args3.insert("key".to_string(), "user".to_string());
        args3.insert("value".to_string(), "admin".to_string());
        let args3 = ParsedArgs::from_scalars(args3);
        set_cmd.execute(&mut context, &args3).unwrap();

        // Step 4: Query state
        let get_cmd = GetCommand;
        let mut args4 = HashMap::new();
        args4.insert("key".to_string(), "user".to_string());
        let args4 = ParsedArgs::from_scalars(args4);
        get_cmd.execute(&mut context, &args4).unwrap();

        // Verify the complete workflow
        assert_eq!(context.state.len(), 2);
        assert_eq!(context.log.len(), 2);
        assert_eq!(context.log[0], "System initialized");
        assert_eq!(context.log[1], "user = admin");
    }
}
