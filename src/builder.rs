//! Fluent builder API for creating CLI/REPL applications
//!
//! This module provides a builder pattern for easily constructing
//! CLI and REPL applications with minimal boilerplate.
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::prelude::*;
//! use std::collections::HashMap;
//!
//! // Define context
//! #[derive(Default)]
//! struct MyContext;
//!
//! impl ExecutionContext for MyContext {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! }
//!
//! // Define handler
//! struct HelloCommand;
//!
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &HashMap<String, String>,
//!     ) -> dynamic_cli::Result<()> {
//!         println!("Hello!");
//!         Ok(())
//!     }
//! }
//!
//! # fn main() -> dynamic_cli::Result<()> {
//! // Build and run
//! CliBuilder::new()
//!     .config_file("commands.yaml")
//!     .context(Box::new(MyContext::default()))
//!     .register_handler("hello_handler", Box::new(HelloCommand))
//!     .build()?
//!     .run()
//! # }
//! ```

use crate::config::loader::load_config;
use crate::config::schema::CommandsConfig;
use crate::context::ExecutionContext;
use crate::error::{ConfigError, DynamicCliError, Result};
use crate::executor::CommandHandler;
use crate::interface::{CliInterface, ReplInterface};
use crate::registry::CommandRegistry;
use std::collections::HashMap;
use std::path::PathBuf;

/// Fluent builder for creating CLI/REPL applications
///
/// Provides a chainable API for configuring and building applications.
/// Automatically loads configuration, registers handlers, and creates
/// the appropriate interface (CLI or REPL).
///
/// # Builder Pattern
///
/// The builder follows the standard Rust builder pattern:
/// - Methods consume `self` and return `Self`
/// - Final `build()` method consumes the builder and returns the app
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::prelude::*;
///
/// # #[derive(Default)]
/// # struct MyContext;
/// # impl ExecutionContext for MyContext {
/// #     fn as_any(&self) -> &dyn std::any::Any { self }
/// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
/// # }
/// # struct MyHandler;
/// # impl CommandHandler for MyHandler {
/// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
/// # }
/// # fn main() -> dynamic_cli::Result<()> {
/// let app = CliBuilder::new()
///     .config_file("commands.yaml")
///     .context(Box::new(MyContext::default()))
///     .register_handler("my_handler", Box::new(MyHandler))
///     .prompt("myapp")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct CliBuilder {
    /// Path to configuration file
    config_path: Option<PathBuf>,

    /// Loaded configuration
    config: Option<CommandsConfig>,

    /// Execution context
    context: Option<Box<dyn ExecutionContext>>,

    /// Registered command handlers (name -> handler)
    handlers: HashMap<String, Box<dyn CommandHandler>>,

    /// REPL prompt (if None, will use config default or "cli")
    prompt: Option<String>,
}

impl CliBuilder {
    /// Create a new builder
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::CliBuilder;
    ///
    /// let builder = CliBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            config_path: None,
            config: None,
            context: None,
            handlers: HashMap::new(),
            prompt: None,
        }
    }

    /// Specify the configuration file
    ///
    /// The file will be loaded during `build()`. Supports YAML and JSON formats.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file (`.yaml`, `.yml`, or `.json`)
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::CliBuilder;
    ///
    /// let builder = CliBuilder::new()
    ///     .config_file("commands.yaml");
    /// ```
    pub fn config_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config_path = Some(path.into());
        self
    }

    /// Provide a pre-loaded configuration
    ///
    /// Use this instead of `config_file()` if you want to load and potentially
    /// modify the configuration before building.
    ///
    /// # Arguments
    ///
    /// * `config` - Loaded and validated configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::{CliBuilder, config::loader::load_config};
    ///
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let mut config = load_config("commands.yaml")?;
    /// // Modify config if needed...
    ///
    /// let builder = CliBuilder::new()
    ///     .config(config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn config(mut self, config: CommandsConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the execution context
    ///
    /// The context will be passed to all command handlers and can store
    /// application state.
    ///
    /// # Arguments
    ///
    /// * `context` - Boxed execution context implementing `ExecutionContext`
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct MyContext {
    ///     count: u32,
    /// }
    ///
    /// impl ExecutionContext for MyContext {
    ///     fn as_any(&self) -> &dyn std::any::Any { self }
    ///     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// }
    ///
    /// let builder = CliBuilder::new()
    ///     .context(Box::new(MyContext::default()));
    /// ```
    pub fn context(mut self, context: Box<dyn ExecutionContext>) -> Self {
        self.context = Some(context);
        self
    }

    /// Register a command handler
    ///
    /// Associates a handler with the command's implementation name from the config.
    /// The name must match the `implementation` field in the command definition.
    ///
    /// # Arguments
    ///
    /// * `name` - Implementation name from the configuration
    /// * `handler` - Boxed command handler implementing `CommandHandler`
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::prelude::*;
    /// use std::collections::HashMap;
    ///
    /// struct MyCommand;
    ///
    /// impl CommandHandler for MyCommand {
    ///     fn execute(
    ///         &self,
    ///         _ctx: &mut dyn ExecutionContext,
    ///         _args: &HashMap<String, String>,
    ///     ) -> dynamic_cli::Result<()> {
    ///         println!("Executed!");
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let builder = CliBuilder::new()
    ///     .register_handler("my_command", Box::new(MyCommand));
    /// ```
    pub fn register_handler(
        mut self,
        name: impl Into<String>,
        handler: Box<dyn CommandHandler>,
    ) -> Self {
        self.handlers.insert(name.into(), handler);
        self
    }

    /// Set the REPL prompt
    ///
    /// Only used in REPL mode. If not specified, uses the prompt from
    /// the configuration or defaults to "cli".
    ///
    /// # Arguments
    ///
    /// * `prompt` - Prompt prefix (e.g., "myapp" displays as "myapp > ")
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::CliBuilder;
    ///
    /// let builder = CliBuilder::new()
    ///     .prompt("myapp");
    /// ```
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Build the application
    ///
    /// Performs the following steps:
    /// 1. Load configuration (if `config_file()` was used)
    /// 2. Validate that a context was provided
    /// 3. Create the command registry
    /// 4. Register all command handlers
    /// 5. Verify that all required commands have handlers
    /// 6. Create the `CliApp`
    ///
    /// # Returns
    ///
    /// A configured `CliApp` ready to run
    ///
    /// # Errors
    ///
    /// - Configuration errors (file not found, invalid format, etc.)
    /// - Missing context
    /// - Missing required handlers
    /// - Registry errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # struct MyHandler;
    /// # impl CommandHandler for MyHandler {
    /// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let app = CliBuilder::new()
    ///     .config_file("commands.yaml")
    ///     .context(Box::new(MyContext::default()))
    ///     .register_handler("handler", Box::new(MyHandler))
    ///     .build()?;
    ///
    /// // Now app is ready to run
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(mut self) -> Result<CliApp> {
        // Load configuration if path was specified
        let config = if let Some(config) = self.config.take() {
            config
        } else if let Some(path) = self.config_path.take() {
            load_config(path)?
        } else {
            return Err(DynamicCliError::Config(ConfigError::InvalidSchema {
                reason: "No configuration provided. Use config_file() or config()".to_string(),
                path: None,
            }));
        };

        // Validate context was provided
        let context = self.context.take().ok_or_else(|| {
            DynamicCliError::Config(ConfigError::InvalidSchema {
                reason: "No execution context provided. Use context()".to_string(),
                path: None,
            })
        })?;

        // Create registry and register commands
        let mut registry = CommandRegistry::new();

        for command_def in &config.commands {
            // Find handler for this command
            let handler = self.handlers.remove(&command_def.implementation);

            // Check if handler is required
            if command_def.required && handler.is_none() {
                return Err(DynamicCliError::Config(ConfigError::InvalidSchema {
                    reason: format!(
                        "Required command '{}' has no registered handler (implementation: '{}'). \
                        Use register_handler() to register it.",
                        command_def.name, command_def.implementation
                    ),
                    path: None,
                }));
            }

            // Register command if handler exists
            if let Some(handler) = handler {
                registry.register(command_def.clone(), handler)?;
            }
        }

        // Determine prompt
        let prompt = self
            .prompt
            .or_else(|| Some(config.metadata.prompt.clone()))
            .unwrap_or_else(|| "cli".to_string());

        Ok(CliApp {
            registry,
            context,
            prompt,
        })
    }
}

impl Default for CliBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Built CLI/REPL application
///
/// Created by `CliBuilder::build()`. Provides methods to run the application
/// in different modes:
/// - `run()` - Auto-detect CLI vs REPL based on arguments
/// - `run_cli()` - Force CLI mode with specific arguments
/// - `run_repl()` - Force REPL mode
///
/// # Example
///
/// ```no_run
/// use dynamic_cli::prelude::*;
///
/// # #[derive(Default)]
/// # struct MyContext;
/// # impl ExecutionContext for MyContext {
/// #     fn as_any(&self) -> &dyn std::any::Any { self }
/// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
/// # }
/// # struct MyHandler;
/// # impl CommandHandler for MyHandler {
/// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
/// # }
/// # fn main() -> dynamic_cli::Result<()> {
/// let app = CliBuilder::new()
///     .config_file("commands.yaml")
///     .context(Box::new(MyContext::default()))
///     .register_handler("handler", Box::new(MyHandler))
///     .build()?;
///
/// // Auto-detect mode (CLI if args provided, REPL otherwise)
/// app.run()
/// # }
/// ```
pub struct CliApp {
    /// Command registry
    registry: CommandRegistry,

    /// Execution context
    context: Box<dyn ExecutionContext>,

    /// REPL prompt
    prompt: String,
}

impl std::fmt::Debug for CliApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliApp")
            .field("prompt", &self.prompt)
            .field("registry", &"<CommandRegistry>")
            .field("context", &"<ExecutionContext>")
            .finish()
    }
}

impl CliApp {
    /// Run in CLI mode with provided arguments
    ///
    /// Executes a single command and exits.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (typically from `env::args().skip(1)`)
    ///
    /// # Returns
    ///
    /// - `Ok(())` on successful execution
    /// - `Err(...)` on parse, validation, or execution errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dynamic_cli::prelude::*;
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # struct MyHandler;
    /// # impl CommandHandler for MyHandler {
    /// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// # let app = CliBuilder::new()
    /// #     .config_file("commands.yaml")
    /// #     .context(Box::new(MyContext::default()))
    /// #     .register_handler("handler", Box::new(MyHandler))
    /// #     .build()?;
    /// // Run with specific arguments
    /// app.run_cli(vec!["command".to_string(), "arg1".to_string()])
    /// # }
    /// ```
    pub fn run_cli(self, args: Vec<String>) -> Result<()> {
        let cli = CliInterface::new(self.registry, self.context);
        cli.run(args)
    }

    /// Run in REPL mode
    ///
    /// Enters an interactive loop that continues until the user exits.
    ///
    /// # Returns
    ///
    /// - `Ok(())` when user exits normally
    /// - `Err(...)` on critical errors (e.g., rustyline initialization failure)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dynamic_cli::prelude::*;
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # struct MyHandler;
    /// # impl CommandHandler for MyHandler {
    /// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// # let app = CliBuilder::new()
    /// #     .config_file("commands.yaml")
    /// #     .context(Box::new(MyContext::default()))
    /// #     .register_handler("handler", Box::new(MyHandler))
    /// #     .build()?;
    /// // Start interactive REPL
    /// app.run_repl()
    /// # }
    /// ```
    pub fn run_repl(self) -> Result<()> {
        let repl = ReplInterface::new(self.registry, self.context, self.prompt)?;
        repl.run()
    }

    /// Run with automatic mode detection
    ///
    /// Decides between CLI and REPL based on command-line arguments:
    /// - If arguments provided → CLI mode
    /// - If no arguments → REPL mode
    ///
    /// This is the recommended method for most applications.
    ///
    /// # Returns
    ///
    /// - `Ok(())` on successful execution
    /// - `Err(...)` on errors
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use dynamic_cli::prelude::*;
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # struct MyHandler;
    /// # impl CommandHandler for MyHandler {
    /// #     fn execute(&self, _: &mut dyn ExecutionContext, _: &std::collections::HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// # let app = CliBuilder::new()
    /// #     .config_file("commands.yaml")
    /// #     .context(Box::new(MyContext::default()))
    /// #     .register_handler("handler", Box::new(MyHandler))
    /// #     .build()?;
    /// // Auto-detect: CLI if args, REPL if no args
    /// app.run()
    /// # }
    /// ```
    pub fn run(self) -> Result<()> {
        let args: Vec<String> = std::env::args().skip(1).collect();

        if args.is_empty() {
            // No arguments → REPL mode
            self.run_repl()
        } else {
            // Arguments provided → CLI mode
            self.run_cli(args)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, CommandDefinition, Metadata};

    // Test context
    #[derive(Default)]
    struct TestContext {
        executed: Vec<String>,
    }

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // Test handler
    struct TestHandler {
        name: String,
    }

    impl CommandHandler for TestHandler {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            let ctx =
                crate::context::downcast_mut::<TestContext>(context).expect("Failed to downcast");
            ctx.executed.push(self.name.clone());
            Ok(())
        }
    }

    fn create_test_config() -> CommandsConfig {
        CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: "test".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![CommandDefinition {
                name: "test".to_string(),
                aliases: vec![],
                description: "Test command".to_string(),
                required: true,
                arguments: vec![],
                options: vec![],
                implementation: "test_handler".to_string(),
            }],
            global_options: vec![],
        }
    }

    #[test]
    fn test_builder_creation() {
        let builder = CliBuilder::new();
        assert!(builder.config.is_none());
        assert!(builder.context.is_none());
    }

    #[test]
    fn test_builder_with_config() {
        let config = create_test_config();
        let builder = CliBuilder::new().config(config.clone());

        assert!(builder.config.is_some());
    }

    #[test]
    fn test_builder_with_context() {
        let context = Box::new(TestContext::default());
        let builder = CliBuilder::new().context(context);

        assert!(builder.context.is_some());
    }

    #[test]
    fn test_builder_with_handler() {
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        let builder = CliBuilder::new().register_handler("test_handler", handler);

        assert_eq!(builder.handlers.len(), 1);
    }

    #[test]
    fn test_builder_with_prompt() {
        let builder = CliBuilder::new().prompt("myapp");

        assert_eq!(builder.prompt, Some("myapp".to_string()));
    }

    #[test]
    fn test_builder_build_success() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        let app = CliBuilder::new()
            .config(config)
            .context(context)
            .register_handler("test_handler", handler)
            .build();

        assert!(app.is_ok());
    }

    #[test]
    fn test_builder_build_missing_config() {
        let context = Box::new(TestContext::default());

        let result = CliBuilder::new().context(context).build();

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::InvalidSchema { reason, .. }) => {
                assert!(reason.contains("No configuration provided"));
            }
            other => panic!("Expected InvalidSchema error, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_build_missing_context() {
        let config = create_test_config();

        let result = CliBuilder::new().config(config).build();

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::InvalidSchema { reason, .. }) => {
                assert!(reason.contains("No execution context provided"));
            }
            other => panic!("Expected InvalidSchema error, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_build_missing_required_handler() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());

        let result = CliBuilder::new().config(config).context(context).build();

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Config(ConfigError::InvalidSchema { reason, .. }) => {
                assert!(reason.contains("Required command"));
                assert!(reason.contains("no registered handler"));
            }
            other => panic!("Expected InvalidSchema error, got: {:?}", other),
        }
    }

    #[test]
    fn test_builder_chaining() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        // Test that all methods chain correctly
        let app = CliBuilder::new()
            .config(config)
            .context(context)
            .register_handler("test_handler", handler)
            .prompt("test")
            .build();

        assert!(app.is_ok());
    }

    #[test]
    fn test_cli_app_run_cli() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        let app = CliBuilder::new()
            .config(config)
            .context(context)
            .register_handler("test_handler", handler)
            .build()
            .unwrap();

        // Run with test command
        let result = app.run_cli(vec!["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_prompt_from_config() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        let app = CliBuilder::new()
            .config(config)
            .context(context)
            .register_handler("test_handler", handler)
            .build()
            .unwrap();

        // Prompt should be taken from config
        assert_eq!(app.prompt, "test");
    }

    #[test]
    fn test_override_prompt() {
        let config = create_test_config();
        let context = Box::new(TestContext::default());
        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        let app = CliBuilder::new()
            .config(config)
            .context(context)
            .register_handler("test_handler", handler)
            .prompt("custom")
            .build()
            .unwrap();

        // Prompt should be overridden
        assert_eq!(app.prompt, "custom");
    }
}
