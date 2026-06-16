//! Built-in system plugin for `dynamic-cli`
//!
//! Provides [`SystemPlugin`], a ready-made plugin supplying the three handlers
//! that every `dynamic-cli` application typically needs: `system_help`,
//! `system_version`, and `system_exit`.
//!
//! See [`SystemPlugin`] for usage and YAML configuration examples.

use crate::config::schema::CommandsConfig;
use crate::context::ExecutionContext;
use crate::executor::CommandHandler;
use crate::help::{DefaultHelpFormatter, HelpFormatter};
use crate::plugin::Plugin;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// SystemPlugin
// ============================================================================

/// Built-in plugin providing standard system commands.
///
/// Supplies ready-made handlers for the commands that every `dynamic-cli`
/// application typically needs. Users declare the corresponding commands in
/// their YAML config and register the plugin once — no manual handler wiring.
///
/// # Provided handlers
///
/// | Implementation name | Behaviour |
/// |---------------------|-----------|
/// | `system_help`       | Prints application or per-command help via the active [`HelpFormatter`][crate::help::HelpFormatter] |
/// | `system_version`    | Prints the version from `metadata.version` in the config |
/// | `system_exit`       | Runs the shutdown callback then exits (default: `std::process::exit(0)`) |
///
/// # Shutdown callback
///
/// `system_exit` accepts an optional callback via [`SystemPlugin::with_exit_fn`].
/// The callback runs **before** the process exits, allowing the application to
/// flush buffers, close connections, save state, or log a goodbye message.
///
/// The default callback calls `std::process::exit(0)` directly. Provide a
/// custom one when a clean shutdown sequence is required:
///
/// ```no_run
/// use dynamic_cli::plugin::SystemPlugin;
///
/// let plugin = SystemPlugin::new()
///     .with_exit_fn(|| {
///         // flush logs, close DB connections, save session…
///         eprintln!("Goodbye.");
///         std::process::exit(0);
///     });
/// ```
///
/// # YAML config
///
/// Declare the commands you want to activate:
///
/// ```yaml
/// commands:
///   - name: help
///     implementation: system_help
///     description: "Show help"
///     aliases: ["h", "?"]
///     required: false
///     arguments: []
///     options: []
///
///   - name: version
///     implementation: system_version
///     description: "Show version"
///     required: false
///     arguments: []
///     options: []
///
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
/// use dynamic_cli::plugin::{Plugin, SystemPlugin};
///
/// let plugin = SystemPlugin::new();
/// assert_eq!(plugin.name(), "system");
///
/// let handlers = plugin.handlers();
/// let names: Vec<&str> = handlers.iter().map(|(n, _)| n.as_str()).collect();
/// assert!(names.contains(&"system_help"));
/// assert!(names.contains(&"system_version"));
/// assert!(names.contains(&"system_exit"));
/// ```
pub struct SystemPlugin {
    /// Application config, needed by `system_help` and `system_version`.
    config: Option<CommandsConfig>,

    /// Shutdown callback invoked by `system_exit`.
    ///
    /// Defaults to `|| std::process::exit(0)`.
    /// Override with [`SystemPlugin::with_exit_fn`] for a clean shutdown
    /// sequence (flush buffers, close connections, save state, etc.).
    exit_fn: Arc<dyn Fn() + Send + Sync>,
}

impl SystemPlugin {
    /// Create a new `SystemPlugin` with the default shutdown behaviour.
    ///
    /// The default exit callback calls `std::process::exit(0)`. Use
    /// [`with_exit_fn`][Self::with_exit_fn] to supply a custom shutdown
    /// sequence.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{Plugin, SystemPlugin};
    ///
    /// let plugin = SystemPlugin::new();
    /// assert_eq!(plugin.name(), "system");
    /// ```
    pub fn new() -> Self {
        Self {
            config: None,
            exit_fn: Arc::new(|| std::process::exit(0)),
        }
    }

    /// Attach a config so the system handlers can access app metadata.
    ///
    /// Called automatically by [`CliBuilder::build()`] when the plugin is
    /// registered via [`CliBuilder::register_plugin`].
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{Plugin, SystemPlugin};
    /// use dynamic_cli::config::schema::{CommandsConfig, Metadata};
    ///
    /// let config = CommandsConfig {
    ///     metadata: Metadata {
    ///         version: "1.0.0".to_string(),
    ///         prompt: "myapp".to_string(),
    ///         prompt_suffix: " > ".to_string(),
    ///     },
    ///     commands: vec![],
    ///     global_options: vec![],
    /// };
    ///
    /// let plugin = SystemPlugin::new().with_config(config);
    /// assert_eq!(plugin.name(), "system");
    /// ```
    pub fn with_config(mut self, config: CommandsConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Supply a custom shutdown callback for `system_exit`.
    ///
    /// The callback is invoked when the user runs the command bound to
    /// `system_exit`. Use it to flush buffers, close connections, persist
    /// state, or display a goodbye message before the process terminates.
    ///
    /// The callback must be `Fn() + Send + Sync + 'static`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::plugin::SystemPlugin;
    ///
    /// let plugin = SystemPlugin::new()
    ///     .with_exit_fn(|| {
    ///         eprintln!("Saving session…");
    ///         // close resources here
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

impl Default for SystemPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SystemPlugin {
    fn name(&self) -> &str {
        "system"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Built-in system commands: help, version, exit"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        let config = self.config.clone();
        let exit_fn = self.exit_fn.clone();

        vec![
            (
                "system_help".to_string(),
                Box::new(SystemHelpHandler {
                    config: config.clone(),
                }),
            ),
            (
                "system_version".to_string(),
                Box::new(SystemVersionHandler { config }),
            ),
            (
                "system_exit".to_string(),
                Box::new(SystemExitHandler { exit_fn }),
            ),
        ]
    }
}

// ============================================================================
// System handlers (private)
// ============================================================================

/// Handler for `system_help` — prints app-level or per-command help.
struct SystemHelpHandler {
    config: Option<CommandsConfig>,
}

impl CommandHandler for SystemHelpHandler {
    fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let formatter = DefaultHelpFormatter::new();

        match self.config.as_ref() {
            Some(cfg) => {
                if let Some(command) = args.get("command") {
                    print!("{}", formatter.format_command(cfg, command));
                } else {
                    print!("{}", formatter.format_app(cfg));
                }
            }
            None => {
                println!("Help is not available (no configuration loaded).");
            }
        }
        Ok(())
    }
}

/// Handler for `system_version` — prints the app version from config metadata.
struct SystemVersionHandler {
    config: Option<CommandsConfig>,
}

impl CommandHandler for SystemVersionHandler {
    fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        match self.config.as_ref() {
            Some(cfg) => println!("{}", cfg.metadata.version),
            None => println!("(version unknown)"),
        }
        Ok(())
    }
}

/// Handler for `system_exit` — invokes the shutdown callback and exits.
///
/// The callback is set via [`SystemPlugin::with_exit_fn`]. The default
/// callback calls `std::process::exit(0)`.
struct SystemExitHandler {
    /// Shutdown callback — runs before the process exits.
    exit_fn: Arc<dyn Fn() + Send + Sync>,
}

impl CommandHandler for SystemExitHandler {
    fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        // Run the shutdown sequence supplied by the application.
        // The default implementation calls std::process::exit(0).
        (self.exit_fn)();
        // Unreachable in production (exit_fn terminates the process),
        // but required for the return type in test configurations
        // where exit_fn does not call std::process::exit.
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{CommandsConfig, Metadata};
    use std::any::Any;

    // -------------------------------------------------------------------------
    // Test fixtures
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

    fn test_config() -> CommandsConfig {
        CommandsConfig {
            metadata: Metadata {
                version: "2.0.0".to_string(),
                prompt: "testapp".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![],
            global_options: vec![],
        }
    }

    // -------------------------------------------------------------------------
    // Metadata
    // -------------------------------------------------------------------------

    #[test]
    fn test_system_plugin_metadata() {
        let p = SystemPlugin::new();
        assert_eq!(p.name(), "system");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    fn test_system_plugin_default() {
        let p = SystemPlugin::default();
        assert_eq!(p.name(), "system");
    }

    #[test]
    fn test_system_plugin_handler_names() {
        let handlers = SystemPlugin::new().handlers();
        let names: Vec<&str> = handlers.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"system_help"));
        assert!(names.contains(&"system_version"));
        assert!(names.contains(&"system_exit"));
        assert_eq!(handlers.len(), 3);
    }

    // -------------------------------------------------------------------------
    // with_config
    // -------------------------------------------------------------------------

    #[test]
    fn test_system_plugin_with_config() {
        let plugin = SystemPlugin::new().with_config(test_config());
        assert!(plugin.config.is_some());
        assert_eq!(plugin.config.unwrap().metadata.version, "2.0.0");
    }

    #[test]
    fn test_system_version_handler_with_config() {
        let plugin = SystemPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (name, handler) = handlers
            .iter()
            .find(|(n, _)| n == "system_version")
            .unwrap();
        assert_eq!(name, "system_version");
        let mut ctx = TestContext;
        assert!(handler.execute(&mut ctx, &HashMap::new()).is_ok());
    }

    #[test]
    fn test_system_version_handler_without_config() {
        let handlers = SystemPlugin::new().handlers();
        let (_, handler) = handlers
            .iter()
            .find(|(n, _)| n == "system_version")
            .unwrap();
        let mut ctx = TestContext;
        assert!(handler.execute(&mut ctx, &HashMap::new()).is_ok());
    }

    #[test]
    fn test_system_help_handler_with_config() {
        let plugin = SystemPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (_, handler) = handlers.iter().find(|(n, _)| n == "system_help").unwrap();
        let mut ctx = TestContext;
        assert!(handler.execute(&mut ctx, &HashMap::new()).is_ok());
    }

    #[test]
    fn test_system_help_handler_with_command_arg() {
        let plugin = SystemPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (_, handler) = handlers.iter().find(|(n, _)| n == "system_help").unwrap();
        let mut ctx = TestContext;
        let mut args = HashMap::new();
        args.insert("command".to_string(), "nonexistent".to_string());
        assert!(handler.execute(&mut ctx, &args).is_ok());
    }

    #[test]
    fn test_system_help_handler_without_config() {
        let handlers = SystemPlugin::new().handlers();
        let (_, handler) = handlers.iter().find(|(n, _)| n == "system_help").unwrap();
        let mut ctx = TestContext;
        assert!(handler.execute(&mut ctx, &HashMap::new()).is_ok());
    }

    // -------------------------------------------------------------------------
    // Shutdown callback
    // -------------------------------------------------------------------------

    #[test]
    fn test_system_exit_default_callback_is_set() {
        // Verify that SystemPlugin::new() initialises exit_fn without panicking.
        // The default callback (process::exit) cannot be invoked in tests;
        // we only check that the plugin builds and exposes the handler.
        let plugin = SystemPlugin::new();
        let handlers = plugin.handlers();
        assert!(handlers.iter().any(|(n, _)| n == "system_exit"));
    }

    #[test]
    fn test_system_exit_custom_callback_invoked() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let plugin = SystemPlugin::new().with_exit_fn(move || {
            called_clone.store(true, Ordering::SeqCst);
            // Does NOT call std::process::exit — safe in tests.
        });

        let handlers = plugin.handlers();
        let (_, handler) = handlers.iter().find(|(n, _)| n == "system_exit").unwrap();

        let mut ctx = TestContext;
        let result = handler.execute(&mut ctx, &HashMap::new());

        assert!(result.is_ok());
        assert!(
            called.load(Ordering::SeqCst),
            "shutdown callback was not invoked"
        );
    }

    #[test]
    fn test_system_exit_callback_ignores_args() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = called.clone();

        let plugin = SystemPlugin::new().with_exit_fn(move || {
            called_clone.store(true, Ordering::SeqCst);
        });

        let handlers = plugin.handlers();
        let (_, handler) = handlers.iter().find(|(n, _)| n == "system_exit").unwrap();

        let mut ctx = TestContext;
        let mut args = HashMap::new();
        args.insert("unexpected_arg".to_string(), "value".to_string());

        assert!(handler.execute(&mut ctx, &args).is_ok());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_with_exit_fn_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        let plugin = SystemPlugin::new().with_exit_fn(|| {});
        assert_send_sync(plugin);
    }
}
