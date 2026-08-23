//! Standalone `ConfigPlugin` (feature-gated).
//!
//! Contributes two handlers over the application's own loaded
//! [`CommandsConfig`] — `config_show` (display it as YAML) and
//! `config_validate` (re-run schema validation without restarting the
//! application) — following the same `Option<CommandsConfig>`
//! attachment pattern as [`SystemPlugin::with_config`][crate::plugin::system::SystemPlugin::with_config].
//!
//! Only available with the `config-plugin` feature.

use crate::config::{validate_config, CommandsConfig};
use crate::context::ExecutionContext;
use crate::executor::CommandHandler;
use crate::parser::ParsedArgs;
use crate::plugin::Plugin;
use crate::Result;

/// Standalone plugin providing `config` commands: `show` and `validate`.
///
/// Both handlers operate on the same [`CommandsConfig`] the application
/// attaches via [`with_config`][Self::with_config] — there is no
/// separate config-loading logic here, only display and re-validation
/// of what the application already loaded.
///
/// # YAML config
///
/// ```yaml
/// commands:
///   - name: config-show
///     implementation: config_show
///     description: "Show the loaded configuration"
///     required: false
///     arguments: []
///     options: []
///   - name: config-validate
///     implementation: config_validate
///     description: "Re-validate the loaded configuration"
///     required: false
///     arguments: []
///     options: []
/// ```
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{ConfigPlugin, Plugin};
///
/// let plugin = ConfigPlugin::new();
/// assert_eq!(plugin.name(), "config");
///
/// let handlers = plugin.handlers();
/// assert_eq!(handlers.len(), 2);
/// ```
pub struct ConfigPlugin {
    /// The application's own config, attached via [`with_config`][Self::with_config].
    config: Option<CommandsConfig>,
}

impl ConfigPlugin {
    /// Create a new `ConfigPlugin` with no config attached.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{ConfigPlugin, Plugin};
    ///
    /// let plugin = ConfigPlugin::new();
    /// assert_eq!(plugin.name(), "config");
    /// ```
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Attach the application's config so `config_show`/`config_validate`
    /// have something to operate on.
    ///
    /// Call this yourself before
    /// [`register_plugin`][crate::builder::CliBuilder::register_plugin] —
    /// `CliBuilder::build()` does not attach config to plugins on its own.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{ConfigPlugin, Plugin};
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
    /// let plugin = ConfigPlugin::new().with_config(config);
    /// assert_eq!(plugin.name(), "config");
    /// ```
    pub fn with_config(mut self, config: CommandsConfig) -> Self {
        self.config = Some(config);
        self
    }
}

impl Default for ConfigPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ConfigPlugin {
    fn name(&self) -> &str {
        "config"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Show/validate the loaded YAML config, without restarting (feature-gated, #47)"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        vec![
            (
                "config_show".to_string(),
                Box::new(ConfigShowHandler {
                    config: self.config.clone(),
                }),
            ),
            (
                "config_validate".to_string(),
                Box::new(ConfigValidateHandler {
                    config: self.config.clone(),
                }),
            ),
        ]
    }
}

/// Handler for `config_show` — prints the loaded config as YAML.
struct ConfigShowHandler {
    config: Option<CommandsConfig>,
}

impl CommandHandler for ConfigShowHandler {
    fn execute(&self, _ctx: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        match &self.config {
            Some(cfg) => match serde_yaml::to_string(cfg) {
                Ok(yaml) => println!("{yaml}"),
                Err(e) => println!("Failed to render config as YAML: {e}"),
            },
            None => println!("No configuration attached to this application."),
        }
        Ok(())
    }
}

/// Handler for `config_validate` — re-runs schema validation on the
/// loaded config, without restarting the application.
struct ConfigValidateHandler {
    config: Option<CommandsConfig>,
}

impl CommandHandler for ConfigValidateHandler {
    fn execute(&self, _ctx: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        match &self.config {
            Some(cfg) => match validate_config(cfg) {
                Ok(()) => println!("Configuration is valid."),
                Err(e) => println!("Configuration is invalid: {e}"),
            },
            None => println!("No configuration attached to this application."),
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::Metadata;
    use std::any::Any;

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
    fn test_config_plugin_metadata() {
        let p = ConfigPlugin::new();
        assert_eq!(p.name(), "config");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    fn test_config_plugin_default() {
        let p = ConfigPlugin::default();
        assert_eq!(p.name(), "config");
    }

    #[test]
    fn test_config_plugin_handler_names() {
        let handlers = ConfigPlugin::new().handlers();
        assert_eq!(handlers.len(), 2);
        let names: Vec<&str> = handlers.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"config_show"));
        assert!(names.contains(&"config_validate"));
    }

    #[test]
    fn test_config_plugin_with_config() {
        let plugin = ConfigPlugin::new().with_config(test_config());
        assert!(plugin.config.is_some());
        assert_eq!(plugin.config.unwrap().metadata.version, "2.0.0");
    }

    #[test]
    fn test_config_show_executes_with_config() {
        let plugin = ConfigPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (name, handler) = &handlers[0];
        assert_eq!(name, "config_show");
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_config_show_executes_without_config() {
        let handlers = ConfigPlugin::new().handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_config_validate_executes_with_valid_config() {
        let plugin = ConfigPlugin::new().with_config(test_config());
        let handlers = plugin.handlers();
        let (name, handler) = &handlers[1];
        assert_eq!(name, "config_validate");
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_config_validate_executes_without_config() {
        let handlers = ConfigPlugin::new().handlers();
        let (_, handler) = &handlers[1];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_config_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(ConfigPlugin::new());
    }
}
