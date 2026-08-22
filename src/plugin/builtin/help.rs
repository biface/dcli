//! Standalone `HelpPlugin`.
//!
//! Contributes the same `system_help` handler as [`SystemPlugin`], reusing
//! [`SystemHelpHandler`]'s logic internally — no duplicated logic (#44 /
//! DD-025).
//!
//! [`SystemPlugin`]: crate::plugin::system::SystemPlugin
//! [`SystemHelpHandler`]: crate::plugin::system::SystemHelpHandler

use crate::config::schema::CommandsConfig;
use crate::executor::CommandHandler;
use crate::plugin::system::SystemHelpHandler;
use crate::plugin::Plugin;

/// Standalone builtin providing only the `help` command.
///
/// Use this instead of [`SystemPlugin`][crate::builtin::system::SystemPlugin]
/// when an application wants `help` without also registering `version` and
/// `exit`.
///
/// # YAML config
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
/// ```
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{HelpPlugin, Plugin};
///
/// let builtin = HelpPlugin::new();
/// assert_eq!(builtin.name(), "help");
///
/// let handlers = builtin.handlers();
/// assert_eq!(handlers.len(), 1);
/// assert_eq!(handlers[0].0, "system_help");
/// ```
pub struct HelpPlugin {
    /// Application config, needed to render help content.
    config: Option<CommandsConfig>,
}

impl HelpPlugin {
    /// Create a new `HelpPlugin` with no config attached.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{HelpPlugin, Plugin};
    ///
    /// let builtin = HelpPlugin::new();
    /// assert_eq!(builtin.name(), "help");
    /// ```
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Attach a config so the handler can render app/command help.
    ///
    /// Call this yourself before
    /// [`register_plugin`][crate::builder::CliBuilder::register_plugin] —
    /// `CliBuilder::build()` does not attach config to plugins on its own.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{HelpPlugin, Plugin};
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
    /// let builtin = HelpPlugin::new().with_config(config);
    /// assert_eq!(builtin.name(), "help");
    /// ```
    pub fn with_config(mut self, config: CommandsConfig) -> Self {
        self.config = Some(config);
        self
    }
}

impl Default for HelpPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for HelpPlugin {
    fn name(&self) -> &str {
        "help"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Standalone help command (split out of SystemPlugin, #44)"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        vec![(
            "system_help".to_string(),
            Box::new(SystemHelpHandler::new(self.config.clone())),
        )]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{CommandsConfig, Metadata};
    use crate::context::ExecutionContext;
    use crate::parser::ParsedArgs;
    use std::any::Any;
    use std::collections::HashMap;

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

    #[test]
    fn test_help_plugin_metadata() {
        let p = HelpPlugin::new();
        assert_eq!(p.name(), "help");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    fn test_help_plugin_default() {
        let p = HelpPlugin::default();
        assert_eq!(p.name(), "help");
    }

    #[test]
    fn test_help_plugin_handler_name() {
        let handlers = HelpPlugin::new().handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "system_help");
    }

    #[test]
    fn test_help_plugin_with_config() {
        let plugin = HelpPlugin::new().with_config(test_config());
        assert!(plugin.config.is_some());
        assert_eq!(plugin.config.unwrap().metadata.version, "2.0.0");
    }

    #[test]
    fn test_help_handler_executes_with_config() {
        let plugin = HelpPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(HashMap::new()))
            .is_ok());
    }

    #[test]
    fn test_help_handler_executes_without_config() {
        let handlers = HelpPlugin::new().handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(HashMap::new()))
            .is_ok());
    }

    #[test]
    fn test_help_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(HelpPlugin::new());
    }
}
