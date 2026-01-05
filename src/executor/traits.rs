//! Command handler trait and related types
//!
//! This module defines the core trait that all command implementations must implement.
//! The trait is designed to be object-safe, meaning it can be used as a trait object
//! (`&dyn CommandHandler`), which is critical for dynamic command registration.
//!
//! # Design Principles
//!
//! ## Object Safety
//!
//! The `CommandHandler` trait is intentionally kept simple and object-safe:
//! - No generic methods (would prevent trait object usage)
//! - No associated types with type parameters
//! - All methods use concrete types or trait objects
//!
//! This allows the registry to store handlers as `Box<dyn CommandHandler>`,
//! enabling dynamic command registration at runtime.
//!
//! ## Simple Type Signatures
//!
//! Arguments are passed as `HashMap<String, String>` rather than generic types.
//! This design choice:
//! - Maintains object safety
//! - Provides flexibility in argument types
//! - Delegates type parsing to the parser module
//!
//! ## Thread Safety
//!
//! All handlers must be `Send + Sync` to support:
//! - Shared access across threads
//! - Potential async execution in the future
//! - Safe usage in multi-threaded contexts
//!
//! # Example
//!
//! ```
//! use std::collections::HashMap;
//! use dynamic_cli::executor::CommandHandler;
//! use dynamic_cli::context::ExecutionContext;
//! use dynamic_cli::Result;
//!
//! // Define a simple command handler
//! struct HelloCommand;
//!
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &HashMap<String, String>,
//!     ) -> Result<()> {
//!         let name = args.get("name").map(|s| s.as_str()).unwrap_or("World");
//!         println!("Hello, {}!", name);
//!         Ok(())
//!     }
//! }
//! ```

use crate::context::ExecutionContext;
use crate::error::Result;
use std::collections::HashMap;

/// Trait for command implementations
///
/// Each command in the CLI/REPL application must implement this trait.
/// The trait is designed to be object-safe, allowing commands to be
/// stored and invoked dynamically through trait objects.
///
/// # Object Safety
///
/// This trait is intentionally object-safe (can be used as `dyn CommandHandler`).
/// **Do not add methods with generic type parameters**, as this would break
/// object safety and prevent dynamic dispatch.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow:
/// - Sharing command handlers across threads
/// - Safe concurrent access to the command registry
/// - Future async execution support
///
/// # Execution Flow
///
/// 1. Parser converts user input to `HashMap<String, String>`
/// 2. Validator checks argument constraints
/// 3. `validate()` is called for custom validation (optional)
/// 4. `execute()` is called with validated arguments
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use dynamic_cli::executor::CommandHandler;
/// use dynamic_cli::context::ExecutionContext;
/// use dynamic_cli::Result;
///
/// struct GreetCommand;
///
/// impl CommandHandler for GreetCommand {
///     fn execute(
///         &self,
///         _context: &mut dyn ExecutionContext,
///         args: &HashMap<String, String>,
///     ) -> Result<()> {
///         let name = args.get("name")
///             .ok_or_else(|| anyhow::anyhow!("Missing 'name' argument"))?;
///         
///         let greeting = if let Some(formal) = args.get("formal") {
///             if formal == "true" {
///                 format!("Good day, {}.", name)
///             } else {
///                 format!("Hi, {}!", name)
///             }
///         } else {
///             format!("Hello, {}!", name)
///         };
///         
///         println!("{}", greeting);
///         Ok(())
///     }
///     
///     fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
///         // Custom validation: name must not be empty
///         if let Some(name) = args.get("name") {
///             if name.trim().is_empty() {
///                 return Err(anyhow::anyhow!("Name cannot be empty").into());
///             }
///         }
///         Ok(())
///     }
/// }
/// ```
pub trait CommandHandler: Send + Sync {
    /// Execute the command with the given context and arguments
    ///
    /// This is the main entry point for command execution. It receives:
    /// - A mutable reference to the execution context (for shared state)
    /// - A map of argument names to their string values
    ///
    /// # Arguments
    ///
    /// * `context` - Mutable execution context for sharing state between commands.
    ///               Use `downcast_ref` or `downcast_mut` from the `context` module
    ///               to access your specific context type.
    ///
    /// * `args` - Parsed and validated arguments as name-value pairs.
    ///           All values are strings; type conversion should be done
    ///           within the handler if needed.
    ///
    /// # Returns
    ///
    /// - `Ok(())` if execution succeeds
    /// - `Err(DynamicCliError)` if execution fails
    ///
    /// # Errors
    ///
    /// Implementations should return errors for:
    /// - Invalid argument values (caught by validate, but can be rechecked)
    /// - Execution failures (I/O errors, computation errors, etc.)
    /// - Invalid context state
    ///
    /// Use `ExecutionError::CommandFailed` to wrap application-specific errors:
    /// ```ignore
    /// Err(ExecutionError::CommandFailed(anyhow::anyhow!("Details")).into())
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use dynamic_cli::Result;
    /// #
    /// struct FileCommand;
    ///
    /// impl CommandHandler for FileCommand {
    ///     fn execute(
    ///         &self,
    ///         _context: &mut dyn ExecutionContext,
    ///         args: &HashMap<String, String>,
    ///     ) -> Result<()> {
    ///         let path = args.get("path")
    ///             .ok_or_else(|| anyhow::anyhow!("Missing path argument"))?;
    ///         
    ///         // Perform the actual work
    ///         let content = std::fs::read_to_string(path)
    ///             .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
    ///         
    ///         println!("File contains {} bytes", content.len());
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()>;

    /// Optional custom validation for arguments
    ///
    /// This method is called after the standard validation (type checking,
    /// required arguments, etc.) but before execution. It allows commands
    /// to implement custom validation logic.
    ///
    /// # Default Implementation
    ///
    /// The default implementation accepts all arguments (returns `Ok(())`).
    /// Override this method only if you need custom validation.
    ///
    /// # Arguments
    ///
    /// * `args` - The arguments to validate
    ///
    /// # Returns
    ///
    /// - `Ok(())` if validation succeeds
    /// - `Err(DynamicCliError)` if validation fails
    ///
    /// # Example
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use dynamic_cli::Result;
    /// #
    /// struct RangeCommand;
    ///
    /// impl CommandHandler for RangeCommand {
    ///     fn execute(
    ///         &self,
    ///         _context: &mut dyn ExecutionContext,
    ///         args: &HashMap<String, String>,
    ///     ) -> Result<()> {
    ///         // Execution logic here
    ///         Ok(())
    ///     }
    ///     
    ///     fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
    ///         // Custom validation: ensure min < max
    ///         if let (Some(min), Some(max)) = (args.get("min"), args.get("max")) {
    ///             let min_val: f64 = min.parse()
    ///                 .map_err(|_| anyhow::anyhow!("Invalid min value"))?;
    ///             let max_val: f64 = max.parse()
    ///                 .map_err(|_| anyhow::anyhow!("Invalid max value"))?;
    ///             
    ///             if min_val >= max_val {
    ///                 return Err(anyhow::anyhow!("min must be less than max").into());
    ///             }
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn validate(&self, _args: &HashMap<String, String>) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ExecutionError;
    use std::any::Any;
    use std::sync::{Arc, Mutex};

    // ============================================================================
    // TEST FIXTURES
    // ============================================================================

    /// Simple test context for unit tests
    #[derive(Default)]
    struct TestContext {
        state: String,
    }

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// Simple command that prints to context
    struct HelloCommand;

    impl CommandHandler for HelloCommand {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &HashMap<String, String>,
        ) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .ok_or_else(|| ExecutionError::CommandFailed(
                    anyhow::anyhow!("Wrong context type")
                ))?;

            let name = args.get("name").map(|s| s.as_str()).unwrap_or("World");
            ctx.state = format!("Hello, {}!", name);
            Ok(())
        }
    }

    /// Command with custom validation
    struct ValidatedCommand;

    impl CommandHandler for ValidatedCommand {
        fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            Ok(())
        }

        fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
            // Require "count" argument to be present and > 0
            if let Some(count) = args.get("count") {
                let count_val: i32 = count
                    .parse()
                    .map_err(|_| ExecutionError::CommandFailed(
                        anyhow::anyhow!("count must be an integer")
                    ))?;

                if count_val <= 0 {
                    return Err(ExecutionError::CommandFailed(
                        anyhow::anyhow!("count must be positive")
                    ).into());
                }
            } else {
                return Err(ExecutionError::CommandFailed(
                    anyhow::anyhow!("count is required")
                ).into());
            }
            Ok(())
        }
    }

    /// Command that fails during execution
    struct FailingCommand;

    impl CommandHandler for FailingCommand {
        fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            Err(ExecutionError::CommandFailed(
                anyhow::anyhow!("Simulated failure")
            ).into())
        }
    }

    /// Command that modifies context
    struct StatefulCommand;

    impl CommandHandler for StatefulCommand {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &HashMap<String, String>,
        ) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .ok_or_else(|| ExecutionError::CommandFailed(
                    anyhow::anyhow!("Wrong context type")
                ))?;

            let value = args.get("value").map(|s| s.as_str()).unwrap_or("default");
            ctx.state.push_str(value);
            Ok(())
        }
    }

    // ============================================================================
    // BASIC FUNCTIONALITY TESTS
    // ============================================================================

    #[test]
    fn test_basic_execution() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Rust".to_string());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, Rust!");
    }

    #[test]
    fn test_execution_without_args() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let args = HashMap::new();

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, World!");
    }

    #[test]
    fn test_execution_with_empty_name() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "".to_string());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, !");
    }

    // ============================================================================
    // VALIDATION TESTS
    // ============================================================================

    #[test]
    fn test_default_validation_accepts_all() {
        let handler = HelloCommand;
        let mut args = HashMap::new();
        args.insert("random".to_string(), "value".to_string());

        let result = handler.validate(&args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_validation_success() {
        let handler = ValidatedCommand;
        let mut args = HashMap::new();
        args.insert("count".to_string(), "5".to_string());

        let result = handler.validate(&args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_validation_missing_arg() {
        let handler = ValidatedCommand;
        let args = HashMap::new();

        let result = handler.validate(&args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("required"));
    }

    #[test]
    fn test_custom_validation_invalid_value() {
        let handler = ValidatedCommand;
        let mut args = HashMap::new();
        args.insert("count".to_string(), "0".to_string());

        let result = handler.validate(&args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("positive"));
    }

    #[test]
    fn test_custom_validation_non_integer() {
        let handler = ValidatedCommand;
        let mut args = HashMap::new();
        args.insert("count".to_string(), "abc".to_string());

        let result = handler.validate(&args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("integer"));
    }

    // ============================================================================
    // ERROR HANDLING TESTS
    // ============================================================================

    #[test]
    fn test_execution_failure() {
        let handler = FailingCommand;
        let mut context = TestContext::default();
        let args = HashMap::new();

        let result = handler.execute(&mut context, &args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Simulated failure"));
    }

    #[test]
    fn test_context_downcast_failure() {
        // Use a different context type to trigger downcast failure
        #[derive(Default)]
        struct WrongContext;

        impl ExecutionContext for WrongContext {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let handler = HelloCommand;
        let mut wrong_context = WrongContext::default();
        let args = HashMap::new();

        let result = handler.execute(&mut wrong_context, &args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Wrong context type"));
    }

    // ============================================================================
    // STATE MODIFICATION TESTS
    // ============================================================================

    #[test]
    fn test_context_state_modification() {
        let handler = StatefulCommand;
        let mut context = TestContext::default();
        context.state = "initial".to_string();
        let mut args = HashMap::new();
        args.insert("value".to_string(), "_modified".to_string());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "initial_modified");
    }

    #[test]
    fn test_multiple_executions_preserve_state() {
        let handler = StatefulCommand;
        let mut context = TestContext::default();

        // First execution
        let mut args1 = HashMap::new();
        args1.insert("value".to_string(), "first".to_string());
        handler.execute(&mut context, &args1).unwrap();
        assert_eq!(context.state, "first");

        // Second execution
        let mut args2 = HashMap::new();
        args2.insert("value".to_string(), "_second".to_string());
        handler.execute(&mut context, &args2).unwrap();
        assert_eq!(context.state, "first_second");
    }

    // ============================================================================
    // TRAIT OBJECT TESTS
    // ============================================================================

    #[test]
    fn test_trait_object_usage() {
        // Verify that CommandHandler can be used as a trait object
        let handler: Box<dyn CommandHandler> = Box::new(HelloCommand);
        let mut context = TestContext::default();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "TraitObject".to_string());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, TraitObject!");
    }

    #[test]
    fn test_multiple_trait_objects() {
        // Store multiple handlers as trait objects
        let handlers: Vec<Box<dyn CommandHandler>> = vec![
            Box::new(HelloCommand),
            Box::new(StatefulCommand),
        ];

        let mut context = TestContext::default();

        // Execute first handler
        let mut args1 = HashMap::new();
        args1.insert("name".to_string(), "First".to_string());
        handlers[0].execute(&mut context, &args1).unwrap();
        assert_eq!(context.state, "Hello, First!");

        // Execute second handler
        context.state.clear();
        let mut args2 = HashMap::new();
        args2.insert("value".to_string(), "Second".to_string());
        handlers[1].execute(&mut context, &args2).unwrap();
        assert_eq!(context.state, "Second");
    }

    // ============================================================================
    // THREAD SAFETY TESTS
    // ============================================================================

    #[test]
    fn test_send_sync_requirement() {
        // This test verifies that CommandHandler is Send + Sync
        // by using it in a multi-threaded context
        let handler: Arc<dyn CommandHandler> = Arc::new(HelloCommand);

        // Clone the Arc to simulate sharing across threads
        let handler_clone = handler.clone();

        // This compilation test ensures Send + Sync are satisfied
        let _ = std::thread::spawn(move || {
            let _h = handler_clone;
        });
    }

    #[test]
    fn test_concurrent_validation() {
        // Test that validation can be called from multiple threads
        let handler = Arc::new(ValidatedCommand);
        let handler_clone = handler.clone();

        let handle = std::thread::spawn(move || {
            let mut args = HashMap::new();
            args.insert("count".to_string(), "10".to_string());
            handler_clone.validate(&args)
        });

        let mut args = HashMap::new();
        args.insert("count".to_string(), "5".to_string());
        let result1 = handler.validate(&args);

        let result2 = handle.join().unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    // ============================================================================
    // EDGE CASES
    // ============================================================================

    #[test]
    fn test_empty_args() {
        let handler = StatefulCommand;
        let mut context = TestContext::default();
        let args = HashMap::new();

        // Should use default value
        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "default");
    }

    #[test]
    fn test_args_with_special_characters() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Hello, 世界! 🌍".to_string());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, Hello, 世界! 🌍!");
    }

    #[test]
    fn test_very_long_argument() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let mut args = HashMap::new();
        let long_name = "x".repeat(10000);
        args.insert("name".to_string(), long_name.clone());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert!(context.state.contains(&long_name));
    }

    // ============================================================================
    // SHARED STATE TESTS
    // ============================================================================

    #[test]
    fn test_shared_mutable_context() {
        // Test that context can be safely modified by multiple commands
        let handler1 = StatefulCommand;
        let handler2 = StatefulCommand;
        let mut context = TestContext::default();

        let mut args1 = HashMap::new();
        args1.insert("value".to_string(), "A".to_string());
        handler1.execute(&mut context, &args1).unwrap();

        let mut args2 = HashMap::new();
        args2.insert("value".to_string(), "B".to_string());
        handler2.execute(&mut context, &args2).unwrap();

        assert_eq!(context.state, "AB");
    }

    // Test to ensure the trait is indeed object-safe at compile time
    #[test]
    fn test_object_safety_compile_time() {
        // This function signature requires CommandHandler to be object-safe
        fn _accepts_trait_object(_: &dyn CommandHandler) {}

        // If this compiles, the trait is object-safe
        let handler = HelloCommand;
        _accepts_trait_object(&handler);
    }

    // Test that demonstrates why we can't have generic methods
    // (This is a documentation test, not an actual test that runs)
    /// ```compile_fail
    /// use dynamic_cli::executor::CommandHandler;
    /// 
    /// trait BrokenHandler: CommandHandler {
    ///     fn generic_method<T>(&self, value: T);
    /// }
    /// 
    /// // This would fail because trait objects can't have generic methods
    /// fn use_as_trait_object(handler: &dyn BrokenHandler) {
    ///     // Cannot call generic_method on trait object
    /// }
    /// ```
    #[allow(dead_code)]
    fn test_no_generic_methods_documentation() {}
}
