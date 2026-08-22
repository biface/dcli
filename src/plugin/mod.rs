//! Plugin system for `dynamic-cli`
//!
//! This module defines the [`Plugin`] trait, the standard extension mechanism
//! for `dynamic-cli` applications. A builtin groups related command handlers
//! under a single unit of deployment with explicit metadata.
//!
//! # Design
//!
//! The builtin system follows the principle established by DD-001 and DD-002:
//! - **The YAML config is the sole source of truth** for command definitions.
//! - **Plugins supply handlers only** — identified by their `implementation`
//!   name, exactly as [`CliBuilder::register_handler`] does.
//! - **The framework controls registration** — the builtin declares what it
//!   provides via [`Plugin::handlers`]; the framework validates and registers.
//!   The builtin never receives a `&mut CommandRegistry`.
//!
//! # Standard builtin
//!
//! [`SystemPlugin`] is provided out of the box. It supplies handlers for the
//! common system commands (`help`, `version`, `exit` / `quit`) that every
//! application typically needs. Users declare the corresponding commands in
//! their YAML config and register the builtin with a single call.
//!
//! # Example
//!
//! ```
//! use dynamic_cli::plugin::{Plugin, SystemPlugin};
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//!
//! // A minimal builtin supplying one handler
//! struct GreetPlugin;
//!
//! impl Plugin for GreetPlugin {
//!     fn name(&self) -> &str { "greet" }
//!     fn version(&self) -> &str { "0.1.0" }
//!     fn description(&self) -> &str { "Greeting commands" }
//!
//!     fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
//!         struct HelloHandler;
//!         impl CommandHandler for HelloHandler {
//!             fn execute(
//!                 &self,
//!                 _ctx: &mut dyn dynamic_cli::context::ExecutionContext,
//!                 args: &ParsedArgs,
//!             ) -> dynamic_cli::Result<()> {
//!                 println!("Hello, {}!", args.get_scalar("name").unwrap_or("World"));
//!                 Ok(())
//!             }
//!         }
//!         vec![("greet_hello".to_string(), Box::new(HelloHandler))]
//!     }
//! }
//!
//! // Verify the trait contract
//! let builtin = GreetPlugin;
//! assert_eq!(builtin.name(), "greet");
//! let handlers = builtin.handlers();
//! assert_eq!(handlers.len(), 1);
//! assert_eq!(handlers[0].0, "greet_hello");
//! ```

use crate::executor::CommandHandler;

// Sub-modules
pub mod system;

// Standalone, single-command plugins split out of `SystemPlugin` (#44 / DD-025).
// Each of `HelpPlugin`, `VersionPlugin`, `ExitPlugin` contributes exactly one
// handler, reusing `system`'s handler logic internally — no duplication.
pub mod builtin;

// Sub-module for the WASM loader (feature-gated, added in #23)
#[cfg(feature = "wasm-plugins")]
pub mod wasm;

// Re-exports for convenience
pub use builtin::{ExitPlugin, HelpPlugin, VersionPlugin};
pub use system::SystemPlugin;

// ============================================================================
// Plugin trait
// ============================================================================

/// Extension point for grouping related command handlers.
///
/// A builtin declares its metadata and the handlers it provides. The framework
/// validates and registers those handlers into the [`CommandRegistry`] during
/// [`CliBuilder::build()`]. The builtin never has direct access to the registry.
///
/// # Contract
///
/// - [`Plugin::handlers`] returns `(implementation_name, handler)` pairs.
/// - Each `implementation_name` must match the `implementation` field of a
///   command declared in the YAML config — exactly as with
///   [`CliBuilder::register_handler`].
/// - The YAML config remains the sole source of truth for command definitions.
///   A builtin cannot inject commands that are not declared in the config.
///
/// # Object safety
///
/// This trait is intentionally object-safe (`dyn Plugin` is valid).
/// Do not add methods with generic type parameters.
///
/// # Thread safety
///
/// Implementations must be `Send + Sync`.
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{Plugin, SystemPlugin};
/// use dynamic_cli::executor::{CommandHandler, ParsedArgs};
/// use dynamic_cli::context::ExecutionContext;
///
/// struct MyPlugin;
///
/// impl Plugin for MyPlugin {
///     fn name(&self) -> &str { "my-builtin" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn description(&self) -> &str { "My custom builtin" }
///
///     fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
///         struct MyHandler;
///         impl CommandHandler for MyHandler {
///             fn execute(
///                 &self,
///                 _ctx: &mut dyn ExecutionContext,
///                 _args: &ParsedArgs,
///             ) -> dynamic_cli::Result<()> {
///                 println!("executed");
///                 Ok(())
///             }
///         }
///         vec![("my_handler".to_string(), Box::new(MyHandler))]
///     }
/// }
///
/// // Trait object usage (object-safe)
/// let builtin: Box<dyn Plugin> = Box::new(MyPlugin);
/// assert_eq!(builtin.name(), "my-builtin");
/// assert_eq!(builtin.version(), "1.0.0");
/// assert_eq!(builtin.handlers().len(), 1);
/// ```
pub trait Plugin: Send + Sync {
    /// Short identifier for this builtin (e.g. `"system"`, `"greet"`).
    fn name(&self) -> &str;

    /// Semantic version string (e.g. `"1.0.0"`).
    fn version(&self) -> &str;

    /// Human-readable description of what this builtin provides.
    fn description(&self) -> &str;

    /// Returns the handlers this builtin contributes.
    ///
    /// Each element is `(implementation_name, handler)` where
    /// `implementation_name` matches the `implementation` field in the YAML
    /// config for the corresponding command.
    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)>;
}

// ============================================================================
// Tests — Plugin trait contract and fixture plugins
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;
    use crate::parser::ParsedArgs;
    use crate::Result;
    use std::any::Any;
    use std::collections::HashMap;

    // -------------------------------------------------------------------------
    // Test fixtures (trait-level — no SystemPlugin dependency here)
    // -------------------------------------------------------------------------

    #[derive(Default)]
    struct TestContext;

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    struct EchoPlugin;

    impl Plugin for EchoPlugin {
        fn name(&self) -> &str {
            "echo"
        }
        fn version(&self) -> &str {
            "0.1.0"
        }
        fn description(&self) -> &str {
            "Echoes its arguments"
        }

        fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
            struct EchoHandler;
            impl CommandHandler for EchoHandler {
                fn execute(
                    &self,
                    _ctx: &mut dyn ExecutionContext,
                    args: &ParsedArgs,
                ) -> Result<()> {
                    for (k, v) in args.to_scalar_map() {
                        println!("{k}={v}");
                    }
                    Ok(())
                }
            }
            vec![("echo_handler".to_string(), Box::new(EchoHandler))]
        }
    }

    struct MultiHandlerPlugin;

    impl Plugin for MultiHandlerPlugin {
        fn name(&self) -> &str {
            "multi"
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn description(&self) -> &str {
            "Plugin with multiple handlers"
        }

        fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
            struct NoopHandler;
            impl CommandHandler for NoopHandler {
                fn execute(&self, _: &mut dyn ExecutionContext, _: &ParsedArgs) -> Result<()> {
                    Ok(())
                }
            }
            vec![
                ("multi_alpha".to_string(), Box::new(NoopHandler)),
                ("multi_beta".to_string(), Box::new(NoopHandler)),
                ("multi_gamma".to_string(), Box::new(NoopHandler)),
            ]
        }
    }

    struct MetadataPlugin;

    impl Plugin for MetadataPlugin {
        fn name(&self) -> &str {
            "acme-analytics"
        }
        fn version(&self) -> &str {
            "3.1.4"
        }
        fn description(&self) -> &str {
            "Analytics commands for Acme Corp"
        }
        fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
            vec![]
        }
    }

    // -------------------------------------------------------------------------
    // Plugin trait — object safety
    // -------------------------------------------------------------------------

    #[test]
    fn test_plugin_is_object_safe() {
        // If this compiles, Plugin is dyn-compatible.
        let _: Box<dyn Plugin> = Box::new(EchoPlugin);
    }

    #[test]
    fn test_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EchoPlugin>();
        assert_send_sync::<MultiHandlerPlugin>();
        assert_send_sync::<SystemPlugin>();
    }

    // -------------------------------------------------------------------------
    // EchoPlugin — minimal single-handler builtin
    // -------------------------------------------------------------------------

    #[test]
    fn test_echo_plugin_metadata() {
        let p = EchoPlugin;
        assert_eq!(p.name(), "echo");
        assert_eq!(p.version(), "0.1.0");
        assert_eq!(p.description(), "Echoes its arguments");
    }

    #[test]
    fn test_echo_plugin_handlers_count() {
        let handlers = EchoPlugin.handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "echo_handler");
    }

    #[test]
    fn test_echo_handler_executes() {
        let handlers = EchoPlugin.handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        let mut args = HashMap::new();
        args.insert("key".to_string(), "value".to_string());
        let args = ParsedArgs::from_scalars(args);
        assert!(handler.execute(&mut ctx, &args).is_ok());
    }

    // -------------------------------------------------------------------------
    // MultiHandlerPlugin
    // -------------------------------------------------------------------------

    #[test]
    fn test_multi_handler_plugin_count() {
        let handlers = MultiHandlerPlugin.handlers();
        assert_eq!(handlers.len(), 3);
        let names: Vec<&str> = handlers.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"multi_alpha"));
        assert!(names.contains(&"multi_beta"));
        assert!(names.contains(&"multi_gamma"));
    }

    // -------------------------------------------------------------------------
    // MetadataPlugin
    // -------------------------------------------------------------------------

    #[test]
    fn test_metadata_plugin_fields() {
        let p = MetadataPlugin;
        assert_eq!(p.name(), "acme-analytics");
        assert_eq!(p.version(), "3.1.4");
        assert_eq!(p.description(), "Analytics commands for Acme Corp");
        assert_eq!(p.handlers().len(), 0);
    }

    // -------------------------------------------------------------------------
    // Plugin as trait object — collections
    // -------------------------------------------------------------------------

    #[test]
    fn test_plugin_trait_object_in_vec() {
        let plugins: Vec<Box<dyn Plugin>> = vec![
            Box::new(EchoPlugin),
            Box::new(MultiHandlerPlugin),
            Box::new(MetadataPlugin),
            Box::new(SystemPlugin::new()),
        ];
        assert_eq!(plugins.len(), 4);
        let names: Vec<&str> = plugins.iter().map(|p| p.name()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"multi"));
        assert!(names.contains(&"acme-analytics"));
        assert!(names.contains(&"system"));
    }

    #[test]
    fn test_plugin_handlers_total_count() {
        let plugins: Vec<Box<dyn Plugin>> =
            vec![Box::new(EchoPlugin), Box::new(MultiHandlerPlugin)];
        let total: usize = plugins.iter().map(|p| p.handlers().len()).sum();
        assert_eq!(total, 4); // 1 + 3
    }
}
