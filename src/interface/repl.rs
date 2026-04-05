//! REPL (Read-Eval-Print Loop) implementation
//!
//! This module provides an interactive REPL interface with:
//! - Line editing (arrow keys, history navigation)
//! - Command history (persistent across sessions)
//! - Tab completion (future enhancement)
//! - Colored prompts and error display
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::interface::ReplInterface;
//! use dynamic_cli::prelude::*;
//!
//! # #[derive(Default)]
//! # struct MyContext;
//! # impl ExecutionContext for MyContext {
//! #     fn as_any(&self) -> &dyn std::any::Any { self }
//! #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! # }
//! # fn main() -> dynamic_cli::Result<()> {
//! let registry = CommandRegistry::new();
//! let context = Box::new(MyContext::default());
//!
//! let repl = ReplInterface::new(registry, context, "myapp".to_string())?;
//! repl.run()?;
//! # Ok(())
//! # }
//! ```

use crate::config::schema::CommandsConfig;
use crate::context::ExecutionContext;
use crate::error::{display_error, DynamicCliError, ExecutionError, Result};
use crate::help::HelpFormatter;
use crate::parser::ReplParser;
use crate::registry::CommandRegistry;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;

/// REPL (Read-Eval-Print Loop) interface
///
/// Provides an interactive command-line interface with:
/// - Line editing and history
/// - Persistent command history
/// - Graceful error handling
/// - Special commands (exit, quit, help)
///
/// # Architecture
///
/// ```text
/// User input → rustyline → ReplParser → CommandExecutor → Handler
///                                             ↓
///                                       ExecutionContext
/// ```
///
/// # Special Commands
///
/// The REPL recognizes these built-in commands:
/// - `exit`, `quit` - Exit the REPL
/// - `help` - Show available commands (if registered)
///
/// # History
///
/// Command history is stored in the user's config directory:
/// - Linux: `~/.config/<app_name>/history.txt`
/// - macOS: `~/Library/Application Support/<app_name>/history.txt`
/// - Windows: `%APPDATA%\<app_name>\history.txt`
pub struct ReplInterface {
    /// Command registry
    registry: CommandRegistry,

    /// Execution context
    context: Box<dyn ExecutionContext>,

    /// Prompt string (e.g., "myapp > ")
    prompt: String,

    /// Rustyline editor for input
    editor: DefaultEditor,

    /// History file path
    history_path: Option<PathBuf>,

    /// Application configuration — used by the help formatter.
    /// `None` when no formatter has been supplied.
    config: Option<CommandsConfig>,

    /// Help formatter — renders `--help` output.
    /// `None` when the application was built without a formatter.
    help_formatter: Option<Box<dyn HelpFormatter>>,
}

impl ReplInterface {
    /// Create a new REPL interface
    ///
    /// # Arguments
    ///
    /// * `registry` - Command registry with all registered commands
    /// * `context` - Execution context
    /// * `prompt` - Prompt prefix (e.g., "myapp" will display as "myapp > ")
    ///
    /// # Errors
    ///
    /// Returns an error if rustyline initialization fails (rare).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::ReplInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    ///
    /// let repl = ReplInterface::new(registry, context, "myapp".to_string())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        registry: CommandRegistry,
        context: Box<dyn ExecutionContext>,
        prompt: String,
    ) -> Result<Self> {
        // Create rustyline editor
        let editor = DefaultEditor::new().map_err(|e| {
            ExecutionError::CommandFailed(anyhow::anyhow!("Failed to initialize REPL: {}", e))
        })?;

        // Determine history file path
        let history_path = Self::get_history_path(&prompt);

        let mut repl = Self {
            registry,
            context,
            prompt: format!("{} > ", prompt),
            editor,
            history_path,
            config: None,
            help_formatter: None,
        };

        // Load history if available
        repl.load_history();

        Ok(repl)
    }

    /// Attach a help formatter and configuration to this REPL.
    ///
    /// When supplied, the REPL will intercept `--help` and `--help <command>`
    /// (and their `-h` short forms, as well as `<command> --help`) before
    /// dispatch and print formatted help instead of executing a command.
    ///
    /// This method is called automatically by [`CliBuilder`] when a formatter
    /// has been registered. It can also be called directly when constructing
    /// a `ReplInterface` manually.
    ///
    /// # Arguments
    ///
    /// * `config`    - The loaded application configuration (commands, metadata)
    /// * `formatter` - A boxed [`HelpFormatter`] implementation
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::ReplInterface;
    /// use dynamic_cli::help::DefaultHelpFormatter;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// # let config = dynamic_cli::config::loader::load_config("commands.yaml")?;
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    ///
    /// let repl = ReplInterface::new(registry, context, "myapp".to_string())?
    ///     .with_help(config, Box::new(DefaultHelpFormatter::new()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_help(mut self, config: CommandsConfig, formatter: Box<dyn HelpFormatter>) -> Self {
        self.config = Some(config);
        self.help_formatter = Some(formatter);
        self
    }

    /// Try to handle a `--help` / `-h` request.
    ///
    /// Returns `Some(output)` when the line is a help request and a formatter
    /// is available, `None` otherwise (normal command processing continues).
    ///
    /// Recognized patterns (case-sensitive):
    ///
    /// | Input              | Output                    |
    /// |--------------------|---------------------------|
    /// | `--help`           | Application-level help    |
    /// | `-h`               | Application-level help    |
    /// | `--help <command>` | Per-command help          |
    /// | `-h <command>`     | Per-command help          |
    /// | `<command> --help` | Per-command help          |
    /// | `<command> -h`     | Per-command help          |
    fn try_handle_help(&self, line: &str) -> Option<String> {
        let (config, formatter) = match (&self.config, &self.help_formatter) {
            (Some(c), Some(f)) => (c, f.as_ref()),
            _ => return None,
        };

        let trimmed = line.trim();

        // "--help" or "-h" alone → application-level help
        if trimmed == "--help" || trimmed == "-h" {
            return Some(formatter.format_app(config));
        }

        // "--help <command>" or "-h <command>" → per-command help
        if let Some(rest) = trimmed
            .strip_prefix("--help ")
            .or_else(|| trimmed.strip_prefix("-h "))
        {
            let cmd = rest.trim();
            if !cmd.is_empty() {
                return Some(formatter.format_command(config, cmd));
            }
        }

        // "<command> --help" or "<command> -h" → per-command help
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let last = *parts.last().unwrap();
            if last == "--help" || last == "-h" {
                return Some(formatter.format_command(config, parts[0]));
            }
        }

        None
    }

    /// Get the history file path
    ///
    /// Uses the user's config directory to store command history.
    fn get_history_path(app_name: &str) -> Option<PathBuf> {
        dirs::config_dir().map(|config_dir| {
            let app_dir = config_dir.join(app_name);
            app_dir.join("history.txt")
        })
    }

    /// Load command history from file
    fn load_history(&mut self) {
        if let Some(ref path) = self.history_path {
            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            // Load history (ignore errors if file doesn't exist yet)
            let _ = self.editor.load_history(path);
        }
    }

    /// Save command history to file
    fn save_history(&mut self) {
        if let Some(ref path) = self.history_path {
            if let Err(e) = self.editor.save_history(path) {
                eprintln!("Warning: Failed to save command history: {}", e);
            }
        }
    }

    /// Run the REPL loop
    ///
    /// Enters an interactive loop that:
    /// 1. Displays the prompt
    /// 2. Reads user input
    /// 3. Parses and executes the command
    /// 4. Displays results or errors
    /// 5. Repeats until user exits
    ///
    /// # Returns
    ///
    /// - `Ok(())` when user exits normally (via `exit` or `quit`)
    /// - `Err(_)` on critical errors (I/O failures, etc.)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::ReplInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    ///
    /// let repl = ReplInterface::new(registry, context, "myapp".to_string())?;
    /// repl.run()?; // Starts the REPL loop
    /// # Ok(())
    /// # }
    /// ```
    pub fn run(mut self) -> Result<()> {
        loop {
            // Read line from user
            let readline = self.editor.readline(&self.prompt);

            match readline {
                Ok(line) => {
                    // Skip empty lines
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(line);

                    // Check for built-in exit commands
                    if line == "exit" || line == "quit" {
                        println!("Goodbye!");
                        break;
                    }

                    // Parse and execute command
                    match self.execute_line(line) {
                        Ok(()) => {
                            // Command executed successfully
                        }
                        Err(e) => {
                            // Display error but continue REPL
                            display_error(&e);
                        }
                    }
                }

                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C pressed
                    println!("^C");
                    continue;
                }

                Err(ReadlineError::Eof) => {
                    // Ctrl-D pressed
                    println!("exit");
                    break;
                }

                Err(err) => {
                    // Other readline errors (rare)
                    eprintln!("Error reading input: {}", err);
                    break;
                }
            }
        }

        // Save history before exiting
        self.save_history();

        Ok(())
    }

    /// Execute a single line of input
    ///
    /// Parses the line and executes the corresponding command.
    /// `--help` and `-h` requests are intercepted before dispatch
    /// and handled by the configured [`HelpFormatter`] if one is present.
    fn execute_line(&mut self, line: &str) -> Result<()> {
        // Intercept --help / -h before the parser so the registry
        // is never consulted for help requests.
        if let Some(output) = self.try_handle_help(line) {
            print!("{}", output);
            return Ok(());
        }

        // Create parser (borrows registry immutably)
        let parser = ReplParser::new(&self.registry);

        // Parse command (parser is dropped after this, releasing the borrow)
        let parsed = parser.parse_line(line)?;

        // Now we can borrow registry again to get the handler
        let handler = self
            .registry
            .get_handler(&parsed.command_name)
            .ok_or_else(|| {
                DynamicCliError::Execution(ExecutionError::handler_not_found(
                    &parsed.command_name,
                    "unknown",
                ))
            })?;

        // Execute (handler references registry, context is borrowed mutably)
        handler.execute(&mut *self.context, &parsed.arguments)?;

        Ok(())
    }
}

// Implement Drop to ensure history is saved even if run() is not called
impl Drop for ReplInterface {
    fn drop(&mut self) {
        self.save_history();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, CommandDefinition};
    use std::collections::HashMap;

    // Test context
    #[derive(Default)]
    struct TestContext {
        executed_commands: Vec<String>,
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

    impl crate::executor::CommandHandler for TestHandler {
        fn execute(
            &self,
            context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .expect("Failed to downcast context");
            ctx.executed_commands.push(self.name.clone());
            Ok(())
        }
    }

    fn create_test_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();

        let cmd_def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec!["t".to_string()],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test_handler".to_string(),
        };

        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        registry.register(cmd_def, handler).unwrap();

        registry
    }

    #[test]
    fn test_repl_interface_creation() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let repl = ReplInterface::new(registry, context, "test".to_string());
        assert!(repl.is_ok());
    }

    #[test]
    fn test_repl_execute_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let mut repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        let result = repl.execute_line("test");
        assert!(result.is_ok());

        // Verify command was executed
        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert_eq!(ctx.executed_commands, vec!["test".to_string()]);
    }

    #[test]
    fn test_repl_execute_with_alias() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let mut repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        let result = repl.execute_line("t");
        assert!(result.is_ok());
    }

    #[test]
    fn test_repl_execute_unknown_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let mut repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        let result = repl.execute_line("unknown");
        assert!(result.is_err());

        match result.unwrap_err() {
            DynamicCliError::Parse(_) => {}
            other => panic!("Expected Parse error, got: {:?}", other),
        }
    }

    #[test]
    fn test_repl_history_path() {
        let path = ReplInterface::get_history_path("myapp");

        // Path should exist (unless we're in a very restricted environment)
        if let Some(p) = path {
            assert!(p.to_str().unwrap().contains("myapp"));
            assert!(p.to_str().unwrap().contains("history.txt"));
        }
    }

    #[test]
    fn test_repl_command_with_args() {
        let mut registry = CommandRegistry::new();

        let cmd_def = CommandDefinition {
            name: "greet".to_string(),
            aliases: vec![],
            description: "Greet someone".to_string(),
            required: false,
            arguments: vec![ArgumentDefinition {
                name: "name".to_string(),
                arg_type: ArgumentType::String,
                required: true,
                description: "Name".to_string(),
                validation: vec![],
            }],
            options: vec![],
            implementation: "greet_handler".to_string(),
        };

        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(
                &self,
                _context: &mut dyn ExecutionContext,
                args: &HashMap<String, String>,
            ) -> Result<()> {
                assert_eq!(args.get("name"), Some(&"Alice".to_string()));
                Ok(())
            }
        }

        registry.register(cmd_def, Box::new(GreetHandler)).unwrap();

        let context = Box::new(TestContext::default());
        let mut repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        let result = repl.execute_line("greet Alice");
        assert!(result.is_ok());
    }

    #[test]
    fn test_repl_empty_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let mut repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        // Empty line should return an error from parser
        let result = repl.execute_line("");
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // with_help / try_handle_help
    // -------------------------------------------------------------------------

    /// Minimal config for help tests.
    fn make_help_config() -> crate::config::schema::CommandsConfig {
        use crate::config::schema::{CommandDefinition, CommandsConfig, Metadata};
        CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: "testapp".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![CommandDefinition {
                name: "hello".to_string(),
                aliases: vec!["hi".to_string()],
                description: "Say hello".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "hello_handler".to_string(),
            }],
            global_options: vec![],
        }
    }

    #[test]
    fn test_try_handle_help_without_formatter_returns_none() {
        // No formatter attached → try_handle_help must be a no-op.
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "test".to_string()).unwrap();

        assert!(repl.try_handle_help("--help").is_none());
        assert!(repl.try_handle_help("-h").is_none());
    }

    #[test]
    fn test_try_handle_help_global() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        let out = repl.try_handle_help("--help");
        assert!(out.is_some());
        let out = out.unwrap();
        assert!(out.contains("testapp"), "should contain app prompt");
        assert!(out.contains("hello"), "should list commands");
    }

    #[test]
    fn test_try_handle_help_short_flag() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // -h alone → same as --help
        let out = repl.try_handle_help("-h");
        assert!(out.is_some());
        assert!(out.unwrap().contains("testapp"));
    }

    #[test]
    fn test_try_handle_help_with_command_prefix() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // "--help hello" → per-command help
        let out = repl.try_handle_help("--help hello");
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));

        // "-h hello" → per-command help
        let out2 = repl.try_handle_help("-h hello");
        assert!(out2.is_some());
    }

    #[test]
    fn test_try_handle_help_command_suffix() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // "hello --help" → per-command help
        let out = repl.try_handle_help("hello --help");
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));

        // "hello -h" → per-command help
        let out2 = repl.try_handle_help("hello -h");
        assert!(out2.is_some());
    }

    #[test]
    fn test_try_handle_help_alias() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // "hi" is an alias for "hello"
        let out = repl.try_handle_help("--help hi");
        assert!(out.is_some());
        // The formatter resolves aliases, output should mention hello
        assert!(out.unwrap().contains("hello"));
    }

    #[test]
    fn test_execute_line_help_intercepted() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let mut repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // "--help" must succeed (print help, not dispatch)
        let result = repl.execute_line("--help");
        assert!(result.is_ok(), "help request must not return an error");
    }

    #[test]
    fn test_execute_line_normal_command_still_works_with_formatter() {
        use crate::help::DefaultHelpFormatter;

        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();

        let mut repl = ReplInterface::new(registry, context, "test".to_string())
            .unwrap()
            .with_help(config, Box::new(DefaultHelpFormatter::new()));

        // Normal commands must still execute normally
        let result = repl.execute_line("test");
        assert!(result.is_ok());
    }
}
