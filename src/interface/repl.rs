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

use crate::context::ExecutionContext;
use crate::error::{display_error, DynamicCliError, ExecutionError, Result};
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
        };

        // Load history if available
        repl.load_history();

        Ok(repl)
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
    fn execute_line(&mut self, line: &str) -> Result<()> {
        // Create parser (borrows registry immutably)
        let parser = ReplParser::new(&self.registry);

        // Parse command (parser is dropped after this, releasing the borrow)
        let parsed = parser.parse_line(line)?;

        // Now we can borrow registry again to get the handler
        let handler = self
            .registry
            .get_handler(&parsed.command_name)
            .ok_or_else(|| {
                DynamicCliError::Execution(ExecutionError::HandlerNotFound {
                    command: parsed.command_name.clone(),
                    implementation: "unknown".to_string(),
                })
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
}
