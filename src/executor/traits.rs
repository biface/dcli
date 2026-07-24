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
//! Arguments are passed as [`crate::parser::ParsedArgs`] rather than generic
//! types. This design choice:
//! - Maintains object safety
//! - Represents both scalar and repeatable-option values (DD-024)
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
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
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
//!         args: &ParsedArgs,
//!     ) -> Result<()> {
//!         let name = args.get_scalar("name").unwrap_or("World");
//!         println!("Hello, {}!", name);
//!         Ok(())
//!     }
//! }
//! ```

use crate::context::ExecutionContext;
use crate::error::Result;
use crate::parser::ParsedArgs;
use async_trait::async_trait;

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
/// 1. Parser converts user input to [`crate::parser::ParsedArgs`]
/// 2. Validator checks argument constraints
/// 3. `validate()` is called for custom validation (optional)
/// 4. `execute()` is called with validated arguments
///
/// # Example
///
/// ```
/// use dynamic_cli::error::ExecutionError;
/// use dynamic_cli::executor::{CommandHandler, ParsedArgs};
/// use dynamic_cli::context::ExecutionContext;
/// use dynamic_cli::Result;
///
/// struct GreetCommand;
///
/// impl CommandHandler for GreetCommand {
///     fn execute(
///         &self,
///         _context: &mut dyn ExecutionContext,
///         args: &ParsedArgs,
///     ) -> Result<()> {
///         let name = args.get_scalar("name")
///             .ok_or_else(|| {
///                 ExecutionError::CommandFailed(
///                     anyhow::anyhow!("Missing 'name' argument")
///              )
///          })?;
///         
///         let greeting = if let Some(formal) = args.get_scalar("formal") {
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
///     fn validate(&self, args: &ParsedArgs) -> Result<()> {
///         // Custom validation: name must not be empty
///         if let Some(name) = args.get_scalar("name") {
///             if name.trim().is_empty() {
///                 return Err(ExecutionError::CommandFailed(
///                         anyhow::anyhow!("Name cannot be empty")
///                 ).into());
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
    ///   Use `downcast_ref` or `downcast_mut` from the `context` module
    ///   to access your specific context type.
    ///
    /// * `args` - Parsed and validated arguments as name-value pairs.
    ///   All values are strings; type conversion should be done
    ///   within the handler if needed.
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
    /// # use dynamic_cli::error::ExecutionError;
    /// # use dynamic_cli::executor::{CommandHandler, ParsedArgs};
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use dynamic_cli::Result;
    /// #
    /// struct FileCommand;
    ///
    /// impl CommandHandler for FileCommand {
    ///     fn execute(
    ///         &self,
    ///         _context: &mut dyn ExecutionContext,
    ///         args: &ParsedArgs,
    ///     ) -> Result<()> {
    ///         let path = args.get_scalar("path")
    ///             .ok_or_else(|| {
    ///                ExecutionError::CommandFailed(
    ///                       anyhow::anyhow!("Missing path argument")
    ///             )
    ///          })?;
    ///         
    ///         // Perform the actual work
    ///         let content = std::fs::read_to_string(path)
    ///             .map_err(|e| {
    ///                ExecutionError::CommandFailed(anyhow::anyhow!("Failed to read file: {}", e))
    ///         })?;
    ///         
    ///         println!("File contains {} bytes", content.len());
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()>;

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
    /// # use dynamic_cli::executor::{CommandHandler, ParsedArgs};
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use dynamic_cli::error::ExecutionError;
    /// # use dynamic_cli::Result;
    /// #
    /// struct RangeCommand;
    ///
    /// impl CommandHandler for RangeCommand {
    ///     fn execute(
    ///         &self,
    ///         _context: &mut dyn ExecutionContext,
    ///         args: &ParsedArgs,
    ///     ) -> Result<()> {
    ///         // Execution logic here
    ///         Ok(())
    ///     }
    ///     
    ///     fn validate(&self, args: &ParsedArgs) -> Result<()> {
    ///         // Custom validation: ensure min < max
    ///         if let (Some(min), Some(max)) = (args.get_scalar("min"), args.get_scalar("max")) {
    ///             let min_val: f64 = min.parse()
    ///                 .map_err(|_| {
    ///                     ExecutionError::CommandFailed(anyhow::anyhow!("Invalid min value"))
    ///             })?;
    ///             let max_val: f64 = max.parse()
    ///                 .map_err(|_| {ExecutionError::CommandFailed(anyhow::anyhow!("Invalid max value"))})?;
    ///             
    ///             if min_val >= max_val {
    ///                 return Err(ExecutionError::CommandFailed(anyhow::anyhow!("min must be less than max")).into());
    ///             }
    ///         }
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn validate(&self, _args: &ParsedArgs) -> Result<()> {
        Ok(())
    }
}

/// Async counterpart of [`CommandHandler`].
///
/// Additive to `CommandHandler` (see DD-022) — it does not replace it.
/// Implementations use this trait when their command body needs to perform
/// async I/O (network calls, streaming, etc.). The signatures deliberately
/// mirror `CommandHandler` exactly, `execute`/`validate` aside from the
/// `async` keyword, so that migrating a handler from sync to async is a
/// mechanical change.
///
/// # Object Safety
///
/// Made `dyn`-compatible via `#[async_trait]` (which desugars `async fn` to
/// a boxed, pinned future under the hood). Stored as `Box<dyn
/// AsyncCommandHandler>` in the registry, exactly like `CommandHandler` is
/// stored as `Box<dyn CommandHandler>`.
///
/// # Thread Safety
///
/// Same constraint as `CommandHandler`: `Send + Sync` is required so the
/// handler can be shared across the registry and, transitively, across
/// threads if the application needs it.
///
/// # Why a separate trait instead of an async `CommandHandler`?
///
/// Existing sync `CommandHandler` implementations (including downstream
/// consumers) must keep compiling unchanged. See DD-022 for the full
/// rationale, including why `tokio` is not a dependency of `dynamic-cli`
/// itself and why driving the returned future via
/// `futures::executor::block_on` at the dispatch site is safe.
///
/// # Example
///
/// ```
/// use async_trait::async_trait;
/// use dynamic_cli::executor::{AsyncCommandHandler, ParsedArgs};
/// use dynamic_cli::context::ExecutionContext;
/// use dynamic_cli::error::ExecutionError;
/// use dynamic_cli::Result;
///
/// struct FetchCommand;
///
/// #[async_trait]
/// impl AsyncCommandHandler for FetchCommand {
///     async fn execute(
///         &self,
///         _context: &mut dyn ExecutionContext,
///         args: &ParsedArgs,
///     ) -> Result<()> {
///         let url = args.get_scalar("url").ok_or_else(|| {
///             ExecutionError::CommandFailed(anyhow::anyhow!("Missing 'url' argument"))
///         })?;
///         // Real implementations would `.await` an async HTTP call here.
///         println!("Fetching {url}...");
///         Ok(())
///     }
///
///     async fn validate(&self, args: &ParsedArgs) -> Result<()> {
///         if args.get_scalar("url").is_none() {
///             return Err(ExecutionError::CommandFailed(anyhow::anyhow!("url is required")).into());
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait AsyncCommandHandler: Send + Sync {
    /// Async equivalent of [`CommandHandler::execute`]. Same contract:
    /// receives the mutable execution context and the parsed arguments,
    /// returns `Ok(())` on success or a `DynamicCliError` on failure.
    async fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()>;

    /// Async equivalent of [`CommandHandler::validate`]. Same contract and
    /// same default (accepts all arguments) — override only for custom
    /// validation logic.
    async fn validate(&self, _args: &ParsedArgs) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ExecutionError;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Test helper: build a scalar-only `ParsedArgs` from `[(key, value), ...]`.
    fn scalar_args<const N: usize>(pairs: [(&str, &str); N]) -> ParsedArgs {
        let map: HashMap<String, String> = pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        ParsedArgs::from_scalars(map)
    }

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
        fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context).ok_or_else(|| {
                ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
            })?;

            let name = args.get_scalar("name").unwrap_or("World");
            ctx.state = format!("Hello, {}!", name);
            Ok(())
        }
    }

    /// Command with custom validation
    struct ValidatedCommand;

    impl CommandHandler for ValidatedCommand {
        fn execute(&self, _context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
            Ok(())
        }

        fn validate(&self, args: &ParsedArgs) -> Result<()> {
            // Require "count" argument to be present and > 0
            if let Some(count) = args.get_scalar("count") {
                let count_val: i32 = count.parse().map_err(|_| {
                    ExecutionError::CommandFailed(anyhow::anyhow!("count must be an integer"))
                })?;

                if count_val <= 0 {
                    return Err(ExecutionError::CommandFailed(anyhow::anyhow!(
                        "count must be positive"
                    ))
                    .into());
                }
            } else {
                return Err(
                    ExecutionError::CommandFailed(anyhow::anyhow!("count is required")).into(),
                );
            }
            Ok(())
        }
    }

    /// Command that fails during execution
    struct FailingCommand;

    impl CommandHandler for FailingCommand {
        fn execute(&self, _context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
            Err(ExecutionError::CommandFailed(anyhow::anyhow!("Simulated failure")).into())
        }
    }

    /// Command that modifies context
    struct StatefulCommand;

    impl CommandHandler for StatefulCommand {
        fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context).ok_or_else(|| {
                ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
            })?;

            let value = args.get_scalar("value").unwrap_or("default");
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
        let args = scalar_args([("name", "Rust")]);

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, Rust!");
    }

    #[test]
    fn test_execution_without_args() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let args = ParsedArgs::from_scalars(HashMap::new());

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, World!");
    }

    #[test]
    fn test_execution_with_empty_name() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let args = scalar_args([("name", "")]);

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
        let args = scalar_args([("random", "value")]);

        let result = handler.validate(&args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_validation_success() {
        let handler = ValidatedCommand;
        let args = scalar_args([("count", "5")]);

        let result = handler.validate(&args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_validation_missing_arg() {
        let handler = ValidatedCommand;
        let args = ParsedArgs::from_scalars(HashMap::new());

        let result = handler.validate(&args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("required"));
    }

    #[test]
    fn test_custom_validation_invalid_value() {
        let handler = ValidatedCommand;
        let args = scalar_args([("count", "0")]);

        let result = handler.validate(&args);

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("positive"));
    }

    #[test]
    fn test_custom_validation_non_integer() {
        let handler = ValidatedCommand;
        let args = scalar_args([("count", "abc")]);

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
        let args = ParsedArgs::from_scalars(HashMap::new());

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
        let args = ParsedArgs::from_scalars(HashMap::new());

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
        let args = scalar_args([("value", "_modified")]);

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "initial_modified");
    }

    #[test]
    fn test_multiple_executions_preserve_state() {
        let handler = StatefulCommand;
        let mut context = TestContext::default();

        // First execution
        let args1 = scalar_args([("value", "first")]);
        handler.execute(&mut context, &args1).unwrap();
        assert_eq!(context.state, "first");

        // Second execution
        let args2 = scalar_args([("value", "_second")]);
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
        let args = scalar_args([("name", "TraitObject")]);

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, TraitObject!");
    }

    #[test]
    fn test_multiple_trait_objects() {
        // Store multiple handlers as trait objects
        let handlers: Vec<Box<dyn CommandHandler>> =
            vec![Box::new(HelloCommand), Box::new(StatefulCommand)];

        let mut context = TestContext::default();

        // Execute first handler
        let args1 = scalar_args([("name", "First")]);
        handlers[0].execute(&mut context, &args1).unwrap();
        assert_eq!(context.state, "Hello, First!");

        // Execute second handler
        context.state.clear();
        let args2 = scalar_args([("value", "Second")]);
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
            let args = scalar_args([("count", "10")]);
            handler_clone.validate(&args)
        });

        let args = scalar_args([("count", "5")]);
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
        let args = ParsedArgs::from_scalars(HashMap::new());

        // Should use default value
        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "default");
    }

    #[test]
    fn test_args_with_special_characters() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let args = scalar_args([("name", "Hello, 世界! 🌍")]);

        let result = handler.execute(&mut context, &args);

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, Hello, 世界! 🌍!");
    }

    #[test]
    fn test_very_long_argument() {
        let handler = HelloCommand;
        let mut context = TestContext::default();
        let long_name = "x".repeat(10000);
        let args = scalar_args([("name", long_name.as_str())]);

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

        let args1 = scalar_args([("value", "A")]);
        handler1.execute(&mut context, &args1).unwrap();

        let args2 = scalar_args([("value", "B")]);
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

    // ============================================================================
    // AsyncCommandHandler TESTS (DD-022)
    // ============================================================================

    /// Async command that writes to the test context, mirroring `HelloCommand`.
    struct AsyncHelloCommand;

    #[async_trait]
    impl AsyncCommandHandler for AsyncHelloCommand {
        async fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            args: &ParsedArgs,
        ) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context).ok_or_else(|| {
                ExecutionError::CommandFailed(anyhow::anyhow!("Wrong context type"))
            })?;
            let name = args.get_scalar("name").unwrap_or("World");
            ctx.state = format!("Hello, {}!", name);
            Ok(())
        }
    }

    /// Async command with custom validation, mirroring `ValidatedCommand`.
    struct AsyncValidatedCommand;

    #[async_trait]
    impl AsyncCommandHandler for AsyncValidatedCommand {
        async fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &ParsedArgs,
        ) -> Result<()> {
            Ok(())
        }

        async fn validate(&self, args: &ParsedArgs) -> Result<()> {
            if args.get_scalar("count").is_none() {
                return Err(
                    ExecutionError::CommandFailed(anyhow::anyhow!("count is required")).into(),
                );
            }
            Ok(())
        }
    }

    /// Async command that fails during execution, mirroring `FailingCommand`.
    struct AsyncFailingCommand;

    #[async_trait]
    impl AsyncCommandHandler for AsyncFailingCommand {
        async fn execute(
            &self,
            _context: &mut dyn ExecutionContext,
            _args: &ParsedArgs,
        ) -> Result<()> {
            Err(ExecutionError::CommandFailed(anyhow::anyhow!("Simulated async failure")).into())
        }
    }

    #[test]
    fn test_async_basic_execution() {
        let handler = AsyncHelloCommand;
        let mut context = TestContext::default();
        let args = scalar_args([("name", "Rust")]);

        let result = futures::executor::block_on(handler.execute(&mut context, &args));

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, Rust!");
    }

    #[test]
    fn test_async_default_validation_accepts_all() {
        let handler = AsyncHelloCommand;
        let args = scalar_args([("random", "value")]);

        let result = futures::executor::block_on(handler.validate(&args));

        assert!(result.is_ok());
    }

    #[test]
    fn test_async_custom_validation_missing_arg() {
        let handler = AsyncValidatedCommand;
        let args = ParsedArgs::from_scalars(HashMap::new());

        let result = futures::executor::block_on(handler.validate(&args));

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("required"));
    }

    #[test]
    fn test_async_custom_validation_success() {
        let handler = AsyncValidatedCommand;
        let args = scalar_args([("count", "5")]);

        let result = futures::executor::block_on(handler.validate(&args));

        assert!(result.is_ok());
    }

    #[test]
    fn test_async_execution_failure() {
        let handler = AsyncFailingCommand;
        let mut context = TestContext::default();
        let args = ParsedArgs::from_scalars(HashMap::new());

        let result = futures::executor::block_on(handler.execute(&mut context, &args));

        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Simulated async failure"));
    }

    #[test]
    fn test_async_trait_object_usage() {
        // Verify that AsyncCommandHandler can be used as a trait object —
        // the core object-safety guarantee DD-022 depends on.
        let handler: Box<dyn AsyncCommandHandler> = Box::new(AsyncHelloCommand);
        let mut context = TestContext::default();
        let args = scalar_args([("name", "TraitObject")]);

        let result = futures::executor::block_on(handler.execute(&mut context, &args));

        assert!(result.is_ok());
        assert_eq!(context.state, "Hello, TraitObject!");
    }

    #[test]
    fn test_async_send_sync_requirement() {
        // Verifies AsyncCommandHandler is Send + Sync by sharing it across
        // threads via Arc — same pattern as test_send_sync_requirement above.
        let handler: Arc<dyn AsyncCommandHandler> = Arc::new(AsyncHelloCommand);
        let handler_clone = handler.clone();

        let _ = std::thread::spawn(move || {
            let _h = handler_clone;
        });
    }

    #[test]
    fn test_async_object_safety_compile_time() {
        // If this compiles, AsyncCommandHandler is dyn-compatible.
        fn _accepts_trait_object(_: &dyn AsyncCommandHandler) {}
        _accepts_trait_object(&AsyncHelloCommand);
    }
}
