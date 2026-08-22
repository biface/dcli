//! Standalone `ExitPlugin`.
//!
//! Contributes the same `system_exit` handler as [`SystemPlugin`], reusing
//! [`SystemExitHandler`]'s logic internally — no duplicated logic (#44 /
//! DD-025). Supports the same shutdown-callback mechanism as
//! [`SystemPlugin::with_exit_fn`][crate::builtin::system::SystemPlugin::with_exit_fn].
//!
//! [`SystemPlugin`]: crate::builtin::system::SystemPlugin
//! [`SystemExitHandler`]: crate::builtin::system::SystemExitHandler

use crate::executor::CommandHandler;
use crate::plugin::system::SystemExitHandler;
use crate::plugin::Plugin;
use std::sync::Arc;

/// Standalone builtin providing only the `exit` command.
///
/// Use this instead of [`SystemPlugin`][crate::builtin::system::SystemPlugin]
/// when an application wants `exit` without also registering `help` and
/// `version`.
///
/// # Shutdown callback
///
/// Same mechanism as `SystemPlugin`: the callback runs **before** the
/// process exits. The default callback calls `std::process::exit(0)`
/// directly.
///
/// ```no_run
/// use dynamic_cli::plugin::ExitPlugin;
///
/// let builtin = ExitPlugin::new()
///     .with_exit_fn(|| {
///         eprintln!("Goodbye.");
///         std::process::exit(0);
///     });
/// ```
///
/// # YAML config
///
/// ```yaml
/// commands:
///   - name: exit
///     implementation: system_exit
///     description: "Exit the application"
///     aliases: ["quit", "q"]
///     required: false
///     arguments: []
///     options: []
/// ```
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{ExitPlugin, Plugin};
///
/// let builtin = ExitPlugin::new();
/// assert_eq!(builtin.name(), "exit");
///
/// let handlers = builtin.handlers();
/// assert_eq!(handlers.len(), 1);
/// assert_eq!(handlers[0].0, "system_exit");
/// ```
pub struct ExitPlugin {
    /// Shutdown callback invoked by `system_exit`.
    ///
    /// Defaults to `|| std::process::exit(0)`.
    /// Override with [`ExitPlugin::with_exit_fn`] for a clean shutdown
    /// sequence (flush buffers, close connections, save state, etc.).
    exit_fn: Arc<dyn Fn() + Send + Sync>,
}

impl ExitPlugin {
    /// Create a new `ExitPlugin` with the default shutdown behaviour.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{ExitPlugin, Plugin};
    ///
    /// let builtin = ExitPlugin::new();
    /// assert_eq!(builtin.name(), "exit");
    /// ```
    pub fn new() -> Self {
        Self {
            exit_fn: Arc::new(|| std::process::exit(0)),
        }
    }

    /// Supply a custom shutdown callback for `system_exit`.
    ///
    /// The callback must be `Fn() + Send + Sync + 'static`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::plugin::ExitPlugin;
    ///
    /// let builtin = ExitPlugin::new()
    ///     .with_exit_fn(|| {
    ///         eprintln!("Saving session…");
    ///         std::process::exit(0);
    ///     });
    /// ```
    pub fn with_exit_fn<F>(mut self, f: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.exit_fn = Arc::new(f);
        self
    }
}

impl Default for ExitPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ExitPlugin {
    fn name(&self) -> &str {
        "exit"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Standalone exit command (split out of SystemPlugin, #44)"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        vec![(
            "system_exit".to_string(),
            Box::new(SystemExitHandler::new(self.exit_fn.clone())),
        )]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;
    use crate::parser::ParsedArgs;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};

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

    #[test]
    fn test_exit_plugin_metadata() {
        let p = ExitPlugin::new();
        assert_eq!(p.name(), "exit");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    fn test_exit_plugin_default() {
        let p = ExitPlugin::default();
        assert_eq!(p.name(), "exit");
    }

    #[test]
    fn test_exit_plugin_handler_name() {
        let handlers = ExitPlugin::new().handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "system_exit");
    }

    #[test]
    fn test_exit_default_callback_is_set() {
        // The default callback (process::exit) cannot be invoked in tests;
        // only check that the builtin builds and exposes the handler.
        let plugin = ExitPlugin::new();
        let handlers = plugin.handlers();
        assert_eq!(handlers.len(), 1);
    }

    #[test]
    fn test_exit_custom_callback_invoked() {
        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let plugin = ExitPlugin::new().with_exit_fn(move || {
            called_clone.store(true, Ordering::SeqCst);
            // Does NOT call std::process::exit — safe in tests.
        });

        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let result = handler.execute(&mut ctx, &ParsedArgs::from_scalars(HashMap::new()));

        assert!(result.is_ok());
        assert!(
            called.load(Ordering::SeqCst),
            "shutdown callback was not invoked"
        );
    }

    #[test]
    fn test_exit_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(ExitPlugin::new().with_exit_fn(|| {}));
    }
}
