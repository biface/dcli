//! Execution context module
//!
//! This module provides traits and utilities for managing execution context
//! in CLI/REPL applications built with dynamic-cli.
//!
//! # Overview
//!
//! The execution context is shared state that persists across command executions.
//! Each application defines its own context type that implements the
//! [`ExecutionContext`] trait.
//!
//! # Key Concepts
//!
//! ## Execution Context
//!
//! The context holds application-specific state that commands can read and modify:
//!
//! ```
//! use dynamic_cli::context::ExecutionContext;
//! use std::any::Any;
//!
//! #[derive(Default)]
//! struct AppContext {
//!     session_id: String,
//!     user_data: Vec<String>,
//!     settings: std::collections::HashMap<String, String>,
//! }
//!
//! impl ExecutionContext for AppContext {
//!     fn as_any(&self) -> &dyn Any {
//!         self
//!     }
//!
//!     fn as_any_mut(&mut self) -> &mut dyn Any {
//!         self
//!     }
//! }
//! ```
//!
//! ## Type-Safe Downcasting
//!
//! Since the framework works with trait objects, commands must downcast
//! the context to access their specific type:
//!
//! ```
//! use dynamic_cli::context::{ExecutionContext, downcast_mut};
//! # use std::any::Any;
//! # #[derive(Default)]
//! # struct AppContext { counter: u32 }
//! # impl ExecutionContext for AppContext {
//! #     fn as_any(&self) -> &dyn Any { self }
//! #     fn as_any_mut(&mut self) -> &mut dyn Any { self }
//! # }
//!
//! fn my_command(context: &mut dyn ExecutionContext) -> Result<(), String> {
//!     // Downcast to concrete type
//!     let app_ctx = downcast_mut::<AppContext>(context)
//!         .ok_or("Invalid context type")?;
//!
//!     // Use the context
//!     app_ctx.counter += 1;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Thread Safety
//!
//! All contexts must be `Send + Sync` to support:
//! - Multi-threaded command execution
//! - Async/await patterns
//! - Future framework extensibility
//!
//! # Common Patterns
//!
//! ## Stateless Context
//!
//! For simple applications that don't need state:
//!
//! ```
//! use dynamic_cli::context::ExecutionContext;
//! use std::any::Any;
//!
//! #[derive(Default)]
//! struct EmptyContext;
//!
//! impl ExecutionContext for EmptyContext {
//!     fn as_any(&self) -> &dyn Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn Any { self }
//! }
//! ```
//!
//! ## Stateful Context
//!
//! For applications that maintain state:
//!
//! ```
//! use dynamic_cli::context::ExecutionContext;
//! use std::any::Any;
//! use std::collections::HashMap;
//!
//! struct DatabaseContext {
//!     connection_pool: Vec<String>, // Simplified example
//!     cache: HashMap<String, String>,
//!     transaction_count: u64,
//! }
//!
//! impl Default for DatabaseContext {
//!     fn default() -> Self {
//!         Self {
//!             connection_pool: vec!["conn1".to_string()],
//!             cache: HashMap::new(),
//!             transaction_count: 0,
//!         }
//!     }
//! }
//!
//! impl ExecutionContext for DatabaseContext {
//!     fn as_any(&self) -> &dyn Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn Any { self }
//! }
//! ```
//!
//! ## Error Handling in Commands
//!
//! Best practice for handling downcast failures:
//!
//! ```
//! use dynamic_cli::context::{ExecutionContext, downcast_mut};
//! # use std::any::Any;
//! # struct MyContext { value: i32 }
//! # impl ExecutionContext for MyContext {
//! #     fn as_any(&self) -> &dyn Any { self }
//! #     fn as_any_mut(&mut self) -> &mut dyn Any { self }
//! # }
//!
//! fn robust_handler(
//!     context: &mut dyn ExecutionContext
//! ) -> Result<(), Box<dyn std::error::Error>> {
//!     let ctx = downcast_mut::<MyContext>(context)
//!         .ok_or("Context type mismatch: expected MyContext")?;
//!
//!     ctx.value += 1;
//!     Ok(())
//! }
//! ```
//!
//! # Architecture Notes
//!
//! ## Why Use Trait Objects?
//!
//! The framework uses `dyn ExecutionContext` because:
//! 1. Each application defines its own context type
//! 2. The framework can't know concrete types at compile time
//! 3. This provides maximum flexibility for users
//!
//! ## Why Require Send + Sync?
//!
//! Thread safety bounds enable:
//! - Sharing contexts across threads
//! - Compatibility with async runtimes (tokio, async-std)
//! - Future features like parallel command execution
//!
//! ## Performance Considerations
//!
//! - Downcasting has minimal overhead (type ID comparison)
//! - Context access is not on the hot path for most commands
//! - The trait object indirection is negligible compared to I/O operations
//!
//! # See Also
//!
//! - [`ExecutionContext`]: Core trait for contexts
//! - [`downcast_ref()`]: Helper function for immutable downcasting
//! - [`downcast_mut()`]: Helper function for mutable downcasting

pub mod traits;

// Re-export commonly used types for convenience
pub use traits::{ExecutionContext, downcast_ref, downcast_mut};

#[cfg(test)]
mod tests {
    use super::{ExecutionContext, downcast_ref, downcast_mut};
    use std::any::Any;
    use std::collections::HashMap;

    /// Example context for testing
    #[derive(Default)]
    struct TestAppContext {
        command_count: u32,
        last_command: Option<String>,
        data: HashMap<String, String>,
    }

    impl ExecutionContext for TestAppContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// Another context type for testing type safety
    #[derive(Default)]
    struct AnotherContext {
        value: i32,
    }

    impl ExecutionContext for AnotherContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_module_exports() {
        // Verify that types are exported correctly from the module
        let ctx = TestAppContext::default();

        // Should be able to use the context as a trait object
        let _ctx_ref: &dyn ExecutionContext = &ctx;

        // Should be able to use free functions
        let ctx_ref: &dyn ExecutionContext = &ctx;
        let _downcasted = downcast_ref::<TestAppContext>(ctx_ref);
    }

    #[test]
    fn test_integration_command_pattern() {
        /// Simulate a command handler
        fn increment_command(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            app_ctx.command_count += 1;
            app_ctx.last_command = Some("increment".to_string());

            Ok(())
        }

        let mut ctx = TestAppContext::default();

        // Execute command
        increment_command(&mut ctx).unwrap();

        assert_eq!(ctx.command_count, 1);
        assert_eq!(ctx.last_command, Some("increment".to_string()));
    }

    #[test]
    fn test_integration_multiple_commands() {
        fn command_a(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            app_ctx.data.insert("key_a".to_string(), "value_a".to_string());
            app_ctx.command_count += 1;
            Ok(())
        }

        fn command_b(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            app_ctx.data.insert("key_b".to_string(), "value_b".to_string());
            app_ctx.command_count += 1;
            Ok(())
        }

        let mut ctx = TestAppContext::default();

        // Execute multiple commands
        command_a(&mut ctx).unwrap();
        command_b(&mut ctx).unwrap();

        assert_eq!(ctx.command_count, 2);
        assert_eq!(ctx.data.len(), 2);
        assert_eq!(ctx.data.get("key_a"), Some(&"value_a".to_string()));
        assert_eq!(ctx.data.get("key_b"), Some(&"value_b".to_string()));
    }

    #[test]
    fn test_integration_wrong_context_type() {
        fn command_expecting_test_context(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            downcast_mut::<TestAppContext>(ctx)
                .ok_or("Expected TestAppContext")?;
            Ok(())
        }

        let mut wrong_ctx = AnotherContext::default();

        // Should fail because we're passing the wrong context type
        let result = command_expecting_test_context(&mut wrong_ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_integration_read_only_access() {
        fn read_command_count(ctx: &dyn ExecutionContext) -> Result<u32, String> {
            let app_ctx = downcast_ref::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            Ok(app_ctx.command_count)
        }

        let mut ctx = TestAppContext::default();
        ctx.command_count = 42;

        let count = read_command_count(&ctx).unwrap();
        assert_eq!(count, 42);
    }

    #[test]
    fn test_integration_stateful_workflow() {
        // Simulate a series of commands that build up state
        fn init_command(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            app_ctx.data.insert("initialized".to_string(), "true".to_string());
            app_ctx.last_command = Some("init".to_string());
            Ok(())
        }

        fn process_command(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            // Check initialization
            if app_ctx.data.get("initialized") != Some(&"true".to_string()) {
                return Err("Not initialized".to_string());
            }

            app_ctx.data.insert("processed".to_string(), "true".to_string());
            app_ctx.last_command = Some("process".to_string());
            Ok(())
        }

        fn finalize_command(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            // Check processing
            if app_ctx.data.get("processed") != Some(&"true".to_string()) {
                return Err("Not processed".to_string());
            }

            app_ctx.data.insert("finalized".to_string(), "true".to_string());
            app_ctx.last_command = Some("finalize".to_string());
            Ok(())
        }

        let mut ctx = TestAppContext::default();

        // Execute workflow
        init_command(&mut ctx).unwrap();
        process_command(&mut ctx).unwrap();
        finalize_command(&mut ctx).unwrap();

        // Verify final state
        assert_eq!(ctx.data.get("initialized"), Some(&"true".to_string()));
        assert_eq!(ctx.data.get("processed"), Some(&"true".to_string()));
        assert_eq!(ctx.data.get("finalized"), Some(&"true".to_string()));
        assert_eq!(ctx.last_command, Some("finalize".to_string()));
    }

    #[test]
    fn test_integration_error_propagation() {
        fn failing_command(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            let _app_ctx = downcast_mut::<TestAppContext>(ctx)
                .ok_or("Invalid context")?;

            // Simulate command failure
            Err("Command failed".to_string())
        }

        let mut ctx = TestAppContext::default();

        let result = failing_command(&mut ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Command failed");
    }

    #[test]
    fn test_boxed_context() {
        // Test using Box<dyn ExecutionContext>
        let ctx: Box<dyn ExecutionContext> = Box::new(TestAppContext::default());

        // Should be able to downcast
        let downcasted = downcast_ref::<TestAppContext>(&*ctx);
        assert!(downcasted.is_some());
    }

    #[test]
    fn test_boxed_context_mutation() {
        let mut ctx: Box<dyn ExecutionContext> = Box::new(TestAppContext::default());

        // Downcast and modify
        if let Some(app_ctx) = downcast_mut::<TestAppContext>(&mut *ctx) {
            app_ctx.command_count = 100;
        }

        // Verify modification
        let app_ctx = downcast_ref::<TestAppContext>(&*ctx).unwrap();
        assert_eq!(app_ctx.command_count, 100);
    }

    /// Test complex nested context structure
    #[derive(Default)]
    struct NestedContext {
        outer: HashMap<String, InnerContext>,
    }

    #[derive(Default, Clone)]
    struct InnerContext {
        values: Vec<i32>,
    }

    impl ExecutionContext for NestedContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_nested_context_manipulation() {
        fn add_value(ctx: &mut dyn ExecutionContext, key: &str, value: i32) -> Result<(), String> {
            let nested = downcast_mut::<NestedContext>(ctx)
                .ok_or("Invalid context")?;

            nested.outer
                .entry(key.to_string())
                .or_insert_with(InnerContext::default)
                .values
                .push(value);

            Ok(())
        }

        let mut ctx = NestedContext::default();

        add_value(&mut ctx, "group1", 10).unwrap();
        add_value(&mut ctx, "group1", 20).unwrap();
        add_value(&mut ctx, "group2", 30).unwrap();

        assert_eq!(ctx.outer.get("group1").unwrap().values, vec![10, 20]);
        assert_eq!(ctx.outer.get("group2").unwrap().values, vec![30]);
    }

    #[test]
    fn test_context_with_lifetime_data() {
        // Test that contexts can hold references (with proper lifetimes)
        #[derive(Default)]
        struct RefContext {
            owned_data: String,
        }

        impl ExecutionContext for RefContext {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn Any {
                self
            }
        }

        let mut ctx = RefContext {
            owned_data: "test".to_string(),
        };

        let ctx_ref: &mut dyn ExecutionContext = &mut ctx;

        if let Some(ref_ctx) = downcast_mut::<RefContext>(ctx_ref) {
            ref_ctx.owned_data.push_str(" modified");
        }

        assert_eq!(ctx.owned_data, "test modified");
    }
}