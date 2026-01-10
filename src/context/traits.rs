//! Execution context traits
//!
//! This module defines the core traits for managing execution context
//! in CLI/REPL applications. The execution context is shared state
//! that persists across command executions.
//!
//! # Design Philosophy
//!
//! The context system uses Rust's type system to provide:
//! - **Type safety**: Contexts are strongly typed
//! - **Flexibility**: Each application defines its own context type
//! - **Thread safety**: Contexts must be `Send + Sync`
//! - **Ergonomic downcasting**: Helper methods for type conversion
//!
//! # Example
//!
//! ```
//! use dynamic_cli::context::{ExecutionContext, downcast_mut};
//! use std::any::Any;
//!
//! // Define your application's context
//! #[derive(Default)]
//! struct MyContext {
//!     counter: u32,
//!     data: Vec<String>,
//! }
//!
//! // Implement the ExecutionContext trait
//! impl ExecutionContext for MyContext {
//!     fn as_any(&self) -> &dyn Any {
//!         self
//!     }
//!
//!     fn as_any_mut(&mut self) -> &mut dyn Any {
//!         self
//!     }
//! }
//!
//! // Use the context with downcasting
//! fn use_context(ctx: &mut dyn ExecutionContext) {
//!     // Downcast to concrete type
//!     if let Some(my_ctx) = downcast_mut::<MyContext>(ctx) {
//!         my_ctx.counter += 1;
//!         my_ctx.data.push("Hello".to_string());
//!     }
//! }
//! ```

use std::any::Any;

/// Execution context trait
///
/// This trait defines the interface for application-specific execution contexts.
/// The context is shared state that persists across command executions in
/// both CLI and REPL modes.
///
/// # Thread Safety
///
/// Contexts must implement `Send + Sync` to support:
/// - Multi-threaded command execution
/// - Async runtime compatibility
/// - Future extensibility
///
/// # Type Erasure and Downcasting
///
/// Since the framework doesn't know the concrete context type at compile time,
/// we use the `Any` trait for type-safe runtime downcasting. The `as_any()`
/// and `as_any_mut()` methods enable converting the trait object back to
/// the concrete type.
///
/// # Implementation Guide
///
/// Implementing this trait is straightforward - just return `self`:
///
/// ```
/// use dynamic_cli::context::ExecutionContext;
/// use std::any::Any;
///
/// struct MyContext {
///     // Your application state
///     config: String,
/// }
///
/// impl ExecutionContext for MyContext {
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
///
///     fn as_any_mut(&mut self) -> &mut dyn Any {
///         self
///     }
/// }
/// ```
///
/// # Usage in Command Handlers
///
/// Command handlers receive a `&mut dyn ExecutionContext` and must
/// downcast it to their expected type:
///
/// ```
/// use dynamic_cli::context::{ExecutionContext, downcast_mut};
/// use std::collections::HashMap;///
/// #
/// use rustyline::Context;
///
/// struct MyContext { value: i32 }
/// # impl ExecutionContext for MyContext {
/// #     fn as_any(&self) -> &dyn std::any::Any { self }
/// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
/// # }
/// #
/// fn my_handler(
///     context: &mut dyn ExecutionContext,
///     args: &HashMap<String, String>,
/// ) -> Result<(), Box<dyn std::error::Error>> {
///     // Downcast to concrete type
///     let ctx = downcast_mut::<MyContext>(context)
///         .ok_or("Invalid context type")?;
///
///     // Use the context
///     ctx.value += 1;
///
///     Ok(())
/// }
/// ```
pub trait ExecutionContext: Send + Sync {
    /// Convert this context to an `Any` trait object
    ///
    /// This method enables type-safe downcasting from the trait object
    /// back to the concrete type. The framework uses this internally
    /// to support generic command handlers.
    ///
    /// # Implementation
    ///
    /// Simply return `self` - the compiler handles the conversion:
    ///
    /// ```
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use std::any::Any;
    /// # struct MyContext;
    /// impl ExecutionContext for MyContext {
    ///     fn as_any(&self) -> &dyn Any {
    ///         self  // Just return self
    ///     }
    ///     # fn as_any_mut(&mut self) -> &mut dyn Any { self }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A reference to this object as an `Any` trait object
    fn as_any(&self) -> &dyn Any;

    /// Convert this context to a mutable `Any` trait object
    ///
    /// This is the mutable version of [`as_any()`](Self::as_any),
    /// enabling command handlers to modify the context state.
    ///
    /// # Implementation
    ///
    /// Simply return `self`:
    ///
    /// ```
    /// # use dynamic_cli::context::ExecutionContext;
    /// # use std::any::Any;
    /// # struct MyContext;
    /// impl ExecutionContext for MyContext {
    ///     # fn as_any(&self) -> &dyn Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn Any {
    ///         self  // Just return self
    ///     }
    /// }
    /// ```
    ///
    /// # Returns
    ///
    /// A mutable reference to this object as an `Any` trait object
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Attempt to downcast a context reference to a concrete type
///
/// This function safely converts a trait object reference to a
/// reference of the concrete type `T`. If the context is not
/// of type `T`, it returns `None`.
///
/// # Type Parameters
///
/// * `T` - The concrete type to downcast to. Must implement
///   `ExecutionContext` and have a `'static` lifetime.
///
/// # Arguments
///
/// * `context` - Reference to the execution context trait object
///
/// # Returns
///
/// - `Some(&T)` if the context is of type `T`
/// - `None` if the context is a different type
///
/// # Example
///
/// ```
/// use dynamic_cli::context::{ExecutionContext, downcast_ref};
/// use std::any::Any;
///
/// struct DatabaseContext {
///     connection_string: String,
/// }
///
/// impl ExecutionContext for DatabaseContext {
///     fn as_any(&self) -> &dyn Any { self }
///     fn as_any_mut(&mut self) -> &mut dyn Any { self }
/// }
///
/// fn get_connection(ctx: &dyn ExecutionContext) -> Option<&str> {
///     downcast_ref::<DatabaseContext>(ctx)
///         .map(|db| db.connection_string.as_str())
/// }
/// ```
///
/// # Safety
///
/// This operation is safe because it uses Rust's `Any::downcast_ref`,
/// which performs runtime type checking.
pub fn downcast_ref<T: ExecutionContext + 'static>(context: &dyn ExecutionContext) -> Option<&T> {
    context.as_any().downcast_ref::<T>()
}

/// Attempt to downcast a mutable context reference to a concrete type
///
/// This is the mutable version of [`downcast_ref()`]. It allows command
/// handlers to modify the context state after downcasting.
///
/// # Type Parameters
///
/// * `T` - The concrete type to downcast to. Must implement
///   `ExecutionContext` and have a `'static` lifetime.
///
/// # Arguments
///
/// * `context` - Mutable reference to the execution context trait object
///
/// # Returns
///
/// - `Some(&mut T)` if the context is of type `T`
/// - `None` if the context is a different type
///
/// # Example
///
/// ```
/// use dynamic_cli::context::{ExecutionContext, downcast_mut};
/// use std::any::Any;
///
/// struct Counter {
///     value: u32,
/// }
///
/// impl ExecutionContext for Counter {
///     fn as_any(&self) -> &dyn Any { self }
///     fn as_any_mut(&mut self) -> &mut dyn Any { self }
/// }
///
/// fn increment(ctx: &mut dyn ExecutionContext) {
///     if let Some(counter) = downcast_mut::<Counter>(ctx) {
///         counter.value += 1;
///     }
/// }
/// ```
///
/// # Safety
///
/// This operation is safe because it uses Rust's `Any::downcast_mut`,
/// which performs runtime type checking.
pub fn downcast_mut<T: ExecutionContext + 'static>(
    context: &mut dyn ExecutionContext,
) -> Option<&mut T> {
    context.as_any_mut().downcast_mut::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test context type for basic operations
    #[derive(Default, Debug, PartialEq)]
    struct TestContext {
        value: i32,
        name: String,
    }

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// Alternative context type for testing type mismatch
    #[derive(Default)]
    struct OtherContext {
        data: Vec<u8>,
    }

    impl ExecutionContext for OtherContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_basic_context_implementation() {
        let ctx = TestContext {
            value: 42,
            name: "test".to_string(),
        };

        // Verify the context was created correctly
        assert_eq!(ctx.value, 42);
        assert_eq!(ctx.name, "test");
    }

    #[test]
    fn test_as_any_conversion() {
        let ctx = TestContext::default();

        // Convert to Any
        let any_ref = ctx.as_any();

        // Verify we can downcast back
        let downcasted = any_ref.downcast_ref::<TestContext>();
        assert!(downcasted.is_some());
        assert_eq!(downcasted.unwrap().value, 0);
    }

    #[test]
    fn test_as_any_mut_conversion() {
        let mut ctx = TestContext::default();

        // Convert to mutable Any
        let any_mut = ctx.as_any_mut();

        // Verify we can downcast back and modify
        if let Some(test_ctx) = any_mut.downcast_mut::<TestContext>() {
            test_ctx.value = 100;
        }

        assert_eq!(ctx.value, 100);
    }

    #[test]
    fn test_downcast_ref_success() {
        let ctx = TestContext {
            value: 42,
            name: "test".to_string(),
        };

        // Create trait object
        let ctx_ref: &dyn ExecutionContext = &ctx;

        // Downcast to concrete type
        let downcasted = downcast_ref::<TestContext>(ctx_ref);

        assert!(downcasted.is_some());
        let concrete = downcasted.unwrap();
        assert_eq!(concrete.value, 42);
        assert_eq!(concrete.name, "test");
    }

    #[test]
    fn test_downcast_ref_failure() {
        let ctx = TestContext::default();

        // Create trait object
        let ctx_ref: &dyn ExecutionContext = &ctx;

        // Try to downcast to wrong type
        let downcasted = downcast_ref::<OtherContext>(ctx_ref);

        // Should fail because TestContext is not OtherContext
        assert!(downcasted.is_none());
    }

    #[test]
    fn test_downcast_mut_success() {
        let mut ctx = TestContext {
            value: 10,
            name: "initial".to_string(),
        };

        // Create mutable trait object
        let ctx_mut: &mut dyn ExecutionContext = &mut ctx;

        // Downcast and modify
        if let Some(concrete) = downcast_mut::<TestContext>(ctx_mut) {
            concrete.value = 20;
            concrete.name = "modified".to_string();
        }

        // Verify modifications
        assert_eq!(ctx.value, 20);
        assert_eq!(ctx.name, "modified");
    }

    #[test]
    fn test_downcast_mut_failure() {
        let mut ctx = TestContext::default();

        // Create mutable trait object
        let ctx_mut: &mut dyn ExecutionContext = &mut ctx;

        // Try to downcast to wrong type
        let downcasted = downcast_mut::<OtherContext>(ctx_mut);

        // Should fail because TestContext is not OtherContext
        assert!(downcasted.is_none());
    }

    #[test]
    fn test_multiple_downcasts() {
        let mut ctx = TestContext::default();

        // First downcast
        {
            let ctx_mut: &mut dyn ExecutionContext = &mut ctx;
            if let Some(concrete) = downcast_mut::<TestContext>(ctx_mut) {
                concrete.value = 1;
            }
        }

        // Second downcast
        {
            let ctx_mut: &mut dyn ExecutionContext = &mut ctx;
            if let Some(concrete) = downcast_mut::<TestContext>(ctx_mut) {
                concrete.value += 1;
            }
        }

        assert_eq!(ctx.value, 2);
    }

    #[test]
    fn test_context_is_send_sync() {
        // This test verifies that ExecutionContext requires Send + Sync
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<TestContext>();
        assert_sync::<TestContext>();

        // Verify the trait object is also Send + Sync
        assert_send::<Box<dyn ExecutionContext>>();
        assert_sync::<Box<dyn ExecutionContext>>();
    }

    #[test]
    fn test_realistic_handler_scenario() {
        // Simulate a command handler pattern
        fn handler(ctx: &mut dyn ExecutionContext, increment: i32) -> Result<(), String> {
            let concrete = downcast_mut::<TestContext>(ctx).ok_or("Invalid context type")?;

            concrete.value += increment;
            Ok(())
        }

        let mut ctx = TestContext::default();

        // Execute handler multiple times
        handler(&mut ctx, 10).unwrap();
        handler(&mut ctx, 20).unwrap();
        handler(&mut ctx, 30).unwrap();

        assert_eq!(ctx.value, 60);
    }

    #[test]
    fn test_handler_with_wrong_context_type() {
        fn handler(ctx: &mut dyn ExecutionContext) -> Result<(), String> {
            downcast_mut::<TestContext>(ctx).ok_or("Wrong context type")?;
            Ok(())
        }

        let mut ctx = OtherContext::default();

        // Should fail because we're passing OtherContext
        let result = handler(&mut ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Wrong context type");
    }

    /// Test context with complex state
    #[derive(Default)]
    struct ComplexContext {
        counters: std::collections::HashMap<String, u64>,
        flags: Vec<bool>,
        optional_data: Option<String>,
    }

    impl ExecutionContext for ComplexContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_complex_context_operations() {
        let mut ctx = ComplexContext::default();

        // Insert some data
        ctx.counters.insert("visits".to_string(), 0);
        ctx.flags.push(true);
        ctx.optional_data = Some("data".to_string());

        // Use as trait object
        let ctx_ref: &mut dyn ExecutionContext = &mut ctx;

        // Downcast and modify
        if let Some(complex) = downcast_mut::<ComplexContext>(ctx_ref) {
            *complex.counters.get_mut("visits").unwrap() += 1;
            complex.flags[0] = false;
            complex.optional_data = None;
        }

        // Verify changes
        assert_eq!(*ctx.counters.get("visits").unwrap(), 1);
        assert_eq!(ctx.flags[0], false);
        assert!(ctx.optional_data.is_none());
    }

    #[test]
    fn test_pattern_matching_with_downcast() {
        let mut ctx = TestContext {
            value: 0,
            name: String::new(),
        };

        let ctx_ref: &mut dyn ExecutionContext = &mut ctx;

        // Pattern matching style
        match downcast_mut::<TestContext>(ctx_ref) {
            Some(test_ctx) => {
                test_ctx.value = 42;
                test_ctx.name = "success".to_string();
            }
            None => panic!("Downcast failed"),
        }

        assert_eq!(ctx.value, 42);
        assert_eq!(ctx.name, "success");
    }

    #[test]
    fn test_option_combinators_with_downcast() {
        let ctx = TestContext {
            value: 100,
            name: "test".to_string(),
        };

        let ctx_ref: &dyn ExecutionContext = &ctx;

        // Using Option combinators
        let value = downcast_ref::<TestContext>(ctx_ref)
            .map(|c| c.value)
            .unwrap_or(0);

        assert_eq!(value, 100);
    }

    #[test]
    fn test_default_implementation() {
        // Test that Default works correctly
        let ctx = TestContext::default();

        assert_eq!(ctx.value, 0);
        assert_eq!(ctx.name, "");
    }
}
