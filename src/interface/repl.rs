//! REPL (Read-Eval-Print Loop) implementation
//!
//! This module provides an interactive REPL interface with:
//! - Line editing (arrow keys, history navigation)
//! - Per-application command history (persistent across sessions)
//! - Tab completion at three levels: commands, sub-commands, argument flags
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
//! let repl = ReplInterface::new(registry, context, "myapp".to_string(), None, None)?;
//! repl.run()?;
//! # Ok(())
//! # }
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Editor, Helper};

use crate::config::schema::CommandsConfig;
use crate::context::ExecutionContext;
use crate::error::{display_error, DynamicCliError, ExecutionError, Result};
use crate::help::HelpFormatter;
use crate::parser::ReplParser;
use crate::registry::CommandRegistry;

// ============================================================================
// DcliCompleter
// ============================================================================

/// Tab-completion engine for the REPL.
///
/// Completes at three depth levels driven by the YAML configuration:
///
/// | Input                    | Candidates                              |
/// |--------------------------|------------------------------------------|
/// | `<Tab>`                  | all command names + aliases              |
/// | `he<Tab>`                | command names/aliases starting with `he` |
/// | `hello <Tab>`            | long and short option flags of `hello`   |
/// | `hello --<Tab>`          | long flags of `hello`                    |
/// | `hello -<Tab>`           | short flags of `hello`                   |
///
/// Positional argument values are not completed (open-ended strings).
///
/// The completer holds `Arc` references so it shares the same data as
/// `ReplInterface` without duplication or unsafe aliasing.
struct DcliCompleter {
    /// Shared registry — single source of truth for command names and aliases.
    registry: Arc<CommandRegistry>,

    /// Shared configuration — source of truth for option flags.
    /// `None` when the REPL was constructed without a config.
    config: Option<Arc<CommandsConfig>>,
}

impl DcliCompleter {
    fn new(registry: Arc<CommandRegistry>, config: Option<Arc<CommandsConfig>>) -> Self {
        Self { registry, config }
    }

    /// Collect all flag completions for a given canonical command name.
    ///
    /// Returns both long forms (`--flag`) and short forms (`-f`) for every
    /// option defined on the command.
    fn flags_for(&self, command_name: &str) -> Vec<String> {
        let config = match &self.config {
            Some(c) => c,
            None => return vec![],
        };

        let cmd_def = match config.commands.iter().find(|c| c.name == command_name) {
            Some(d) => d,
            None => return vec![],
        };

        let mut flags = Vec::new();
        for opt in &cmd_def.options {
            if let Some(long) = &opt.long {
                flags.push(format!("--{}", long));
            }
            if let Some(short) = &opt.short {
                flags.push(format!("-{}", short));
            }
        }
        flags
    }
}

impl Completer for DcliCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Work only on the portion of the line up to the cursor.
        let line = &line[..pos];
        let tokens: Vec<&str> = line.split_whitespace().collect();

        // ── Level 1: no token yet, or first token still being typed ──────────
        // Complete command names and aliases.
        let completing_first_token =
            tokens.is_empty() || (tokens.len() == 1 && !line.ends_with(' '));

        if completing_first_token {
            let prefix = tokens.first().copied().unwrap_or("");
            let start = pos - prefix.len();

            let mut candidates: Vec<Pair> = self
                .registry
                .list_commands()
                .into_iter()
                .flat_map(|def| {
                    let mut names = vec![def.name.clone()];
                    names.extend(def.aliases.clone());
                    names
                })
                .filter(|name| name.starts_with(prefix))
                .map(|name| Pair {
                    display: name.clone(),
                    replacement: name,
                })
                .collect();

            candidates.sort_by(|a, b| a.display.cmp(&b.display));
            return Ok((start, candidates));
        }

        // ── Level 2: first token is a complete command, completing flags ──────
        // Resolve the command name (handles aliases).
        let command_token = tokens[0];
        let canonical = match self.registry.resolve_name(command_token) {
            Some(name) => name.to_string(),
            None => return Ok((pos, vec![])),
        };

        // The word being completed (may be empty if cursor follows a space).
        let current_word = if line.ends_with(' ') {
            ""
        } else {
            tokens.last().copied().unwrap_or("")
        };

        // Only offer flag completions when the current word looks like a flag
        // or when the user pressed Tab on an empty position after the command.
        let is_flag_context = current_word.is_empty() || current_word.starts_with('-');

        if !is_flag_context {
            return Ok((pos, vec![]));
        }

        let start = pos - current_word.len();
        let mut candidates: Vec<Pair> = self
            .flags_for(&canonical)
            .into_iter()
            .filter(|flag| flag.starts_with(current_word))
            .map(|flag| Pair {
                display: flag.clone(),
                replacement: flag,
            })
            .collect();

        candidates.sort_by(|a, b| a.display.cmp(&b.display));
        Ok((start, candidates))
    }
}

// ============================================================================
// DcliHelper — rustyline Helper glue
// ============================================================================

/// Rustyline `Helper` implementation that wires `DcliCompleter` into the
/// editor. The remaining traits (`Hinter`, `Highlighter`, `Validator`) use
/// their no-op default implementations.
struct DcliHelper {
    completer: DcliCompleter,
}

impl DcliHelper {
    fn new(registry: Arc<CommandRegistry>, config: Option<Arc<CommandsConfig>>) -> Self {
        Self {
            completer: DcliCompleter::new(registry, config),
        }
    }
}

impl Helper for DcliHelper {}

impl Completer for DcliHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

// No-op implementations required by the Helper supertrait bound.
impl Hinter for DcliHelper {
    type Hint = String;
}

impl Highlighter for DcliHelper {}

impl Validator for DcliHelper {}

// ============================================================================
// ReplInterface
// ============================================================================

/// REPL (Read-Eval-Print Loop) interface
///
/// Provides an interactive command-line interface with:
/// - Line editing and history
/// - Per-application persistent command history
/// - Tab completion (commands, aliases, option flags)
/// - Graceful error handling
/// - Special commands (exit, quit, --help)
///
/// # Architecture
///
/// ```text
/// User input → rustyline (DcliHelper) → ReplParser → CommandExecutor → Handler
///                    ↓                                      ↓
///             Tab completion                         ExecutionContext
///          (commands + flags)
/// ```
///
/// # Special Commands
///
/// The REPL recognizes these built-in commands:
/// - `exit`, `quit` — Exit the REPL
/// - `--help`, `-h` — Show application-level help (if a formatter is attached)
/// - `<cmd> --help`, `--help <cmd>` — Show per-command help
///
/// # History
///
/// Command history is stored per application under the XDG data directory:
/// - Linux/macOS: `~/.local/share/<app_name>/history`
/// - Windows:     `%LOCALAPPDATA%\<app_name>\history`
///
/// Lines containing a `secure: true` argument are never written to history.
/// Lines that fail to parse are discarded silently.
pub struct ReplInterface {
    /// Shared command registry — single source of truth for names, aliases,
    /// definitions, and handlers.
    registry: Arc<CommandRegistry>,

    /// Execution context passed to every command handler.
    context: Box<dyn ExecutionContext>,

    /// Prompt string (e.g., "myapp > ").
    prompt: String,

    /// Rustyline editor with tab-completion support.
    editor: Editor<DcliHelper, rustyline::history::DefaultHistory>,

    /// History file path.
    history_path: Option<PathBuf>,

    /// Application configuration — shared with the completer and used by the
    /// help formatter. `None` when no config was supplied at construction.
    config: Option<Arc<CommandsConfig>>,

    /// Help formatter — renders `--help` output.
    /// `None` when the application was built without a formatter.
    help_formatter: Option<Box<dyn HelpFormatter>>,
}

impl ReplInterface {
    /// Create a new REPL interface.
    ///
    /// All configuration is supplied at construction time so that the
    /// tab-completion engine and the help formatter share the same data
    /// without duplication.
    ///
    /// # Arguments
    ///
    /// * `registry`       — Command registry with all registered commands.
    /// * `context`        — Execution context passed to handlers.
    /// * `prompt`         — Prompt prefix (e.g., `"myapp"` displays as `"myapp > "`).
    /// * `config`         — Application configuration for completion and help.
    ///   Pass `None` to disable both features.
    /// * `help_formatter` — Help formatter implementation.
    ///   Pass `None` to use [`DefaultHelpFormatter`] lazily,
    ///   or supply a custom implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if rustyline initialisation fails (rare).
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
    /// // Without completion or help:
    /// let repl = ReplInterface::new(registry, context, "myapp".to_string(), None, None)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        registry: CommandRegistry,
        context: Box<dyn ExecutionContext>,
        prompt: String,
        config: Option<CommandsConfig>,
        help_formatter: Option<Box<dyn HelpFormatter>>,
    ) -> Result<Self> {
        // Wrap registry in Arc — shared with the completer.
        let registry = Arc::new(registry);

        // Wrap config in Arc if present — shared with the completer.
        let config: Option<Arc<CommandsConfig>> = config.map(Arc::new);

        // Build the rustyline editor with Tab completion enabled.
        let rl_config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let helper = DcliHelper::new(Arc::clone(&registry), config.clone());

        let mut editor = Editor::with_config(rl_config).map_err(|e| {
            ExecutionError::CommandFailed(anyhow::anyhow!("Failed to initialize REPL: {}", e))
        })?;
        editor.set_helper(Some(helper));

        // Determine history file path using the prompt as the app name.
        let history_path = Self::get_history_path(&prompt);

        let mut repl = Self {
            registry,
            context,
            prompt: format!("{} > ", prompt),
            editor,
            history_path,
            config,
            help_formatter,
        };

        repl.load_history();

        Ok(repl)
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
        let config = self.config.as_deref()?;
        let formatter = self.help_formatter.as_deref()?;

        let trimmed = line.trim();

        if trimmed == "--help" || trimmed == "-h" {
            return Some(formatter.format_app(config));
        }

        if let Some(rest) = trimmed
            .strip_prefix("--help ")
            .or_else(|| trimmed.strip_prefix("-h "))
        {
            let cmd = rest.trim();
            if !cmd.is_empty() {
                return Some(formatter.format_command(config, cmd));
            }
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let last = *parts.last().unwrap();
            if last == "--help" || last == "-h" {
                return Some(formatter.format_command(config, parts[0]));
            }
        }

        None
    }

    /// Check whether a parsed command involves at least one secure argument.
    ///
    /// Looks up the command definition in `self.config` (if available) and
    /// returns `true` when any argument name present in `parsed_args` is
    /// marked `secure: true` in the YAML schema.
    fn has_secure_arg(
        &self,
        command_name: &str,
        parsed_args: &std::collections::HashMap<String, String>,
    ) -> bool {
        let config = match &self.config {
            Some(c) => c,
            None => return false,
        };

        let cmd_def = match config.commands.iter().find(|c| c.name == command_name) {
            Some(d) => d,
            None => return false,
        };

        cmd_def
            .arguments
            .iter()
            .any(|arg| arg.secure && parsed_args.contains_key(&arg.name))
    }

    /// Get the history file path for this application.
    ///
    /// Each application gets its own isolated history file under the
    /// XDG data directory:
    ///
    /// - Linux/macOS: `~/.local/share/<app_name>/history`
    /// - Windows:     `%LOCALAPPDATA%\<app_name>\history`
    fn get_history_path(app_name: &str) -> Option<PathBuf> {
        dirs::data_local_dir().map(|data_dir| data_dir.join(app_name).join("history"))
    }

    /// Load command history from file.
    fn load_history(&mut self) {
        if let Some(ref path) = self.history_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = self.editor.load_history(path);
        }
    }

    /// Save command history to file.
    fn save_history(&mut self) {
        if let Some(ref path) = self.history_path {
            if let Err(e) = self.editor.save_history(path) {
                eprintln!("Warning: Failed to save command history: {}", e);
            }
        }
    }

    /// Run the REPL loop.
    ///
    /// Enters an interactive loop that:
    /// 1. Displays the prompt
    /// 2. Reads user input (with tab completion)
    /// 3. Parses and executes the command
    /// 4. Displays results or errors
    /// 5. Repeats until the user exits
    ///
    /// # Returns
    ///
    /// - `Ok(())` when the user exits normally (via `exit` or `quit`)
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
    /// let repl = ReplInterface::new(registry, context, "myapp".to_string(), None, None)?;
    /// repl.run()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn run(mut self) -> Result<()> {
        loop {
            let readline = self.editor.readline(&self.prompt);

            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if line == "exit" || line == "quit" {
                        println!("Goodbye!");
                        break;
                    }

                    // Parse and execute command.
                    // History is written inside execute_line(), after successful
                    // parsing and only when no secure argument is present.
                    match self.execute_line(line) {
                        Ok(()) => {}
                        Err(e) => {
                            display_error(&e);
                        }
                    }
                }

                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }

                Err(ReadlineError::Eof) => {
                    println!("exit");
                    break;
                }

                Err(err) => {
                    eprintln!("Error reading input: {}", err);
                    break;
                }
            }
        }

        self.save_history();
        Ok(())
    }

    /// Execute a single line of input.
    ///
    /// Parses the line and executes the corresponding command.
    /// `--help` and `-h` requests are intercepted before dispatch.
    ///
    /// History is written here — after successful parsing — so that:
    /// - Failed or invalid commands are never persisted.
    /// - Lines containing a `secure: true` argument are silently omitted.
    fn execute_line(&mut self, line: &str) -> Result<()> {
        if let Some(output) = self.try_handle_help(line) {
            print!("{}", output);
            return Ok(());
        }

        let parser = ReplParser::new(&self.registry);
        let parsed = parser.parse_line(line)?;

        // Write to history only on successful parse and when no secure
        // argument is present in the parsed command.
        if !self.has_secure_arg(&parsed.command_name, &parsed.arguments) {
            let _ = self.editor.add_history_entry(line);
        }

        let handler = self
            .registry
            .get_handler(&parsed.command_name)
            .ok_or_else(|| {
                DynamicCliError::Execution(ExecutionError::handler_not_found(
                    &parsed.command_name,
                    "unknown",
                ))
            })?;

        handler.execute(&mut *self.context, &parsed.arguments)?;

        Ok(())
    }
}

impl Drop for ReplInterface {
    fn drop(&mut self) {
        self.save_history();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{
        ArgumentDefinition, ArgumentType, CommandDefinition, OptionDefinition,
    };
    use std::collections::HashMap;

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
        registry
            .register(
                cmd_def,
                Box::new(TestHandler {
                    name: "test".to_string(),
                }),
            )
            .unwrap();
        registry
    }

    fn make_help_config() -> CommandsConfig {
        use crate::config::schema::{CommandsConfig, Metadata};
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
                options: vec![OptionDefinition {
                    name: "loud".to_string(),
                    short: Some("l".to_string()),
                    long: Some("loud".to_string()),
                    option_type: ArgumentType::Bool,
                    required: false,
                    default: Some("false".to_string()),
                    description: "Loud greeting".to_string(),
                    choices: vec![],
                }],
                implementation: "hello_handler".to_string(),
            }],
            global_options: vec![],
        }
    }

    // ── Construction ──────────────────────────────────────────────────────────

    #[test]
    fn test_repl_interface_creation() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "test".to_string(), None, None);
        assert!(repl.is_ok());
    }

    #[test]
    fn test_repl_interface_creation_with_config() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();
        let repl = ReplInterface::new(registry, context, "test".to_string(), Some(config), None);
        assert!(repl.is_ok());
    }

    // ── execute_line ──────────────────────────────────────────────────────────

    #[test]
    fn test_repl_execute_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
        let result = repl.execute_line("test");
        assert!(result.is_ok());
        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert_eq!(ctx.executed_commands, vec!["test".to_string()]);
    }

    #[test]
    fn test_repl_execute_with_alias() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
        assert!(repl.execute_line("t").is_ok());
    }

    #[test]
    fn test_repl_execute_unknown_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
        let result = repl.execute_line("unknown");
        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Parse(_) => {}
            other => panic!("Expected Parse error, got: {:?}", other),
        }
    }

    #[test]
    fn test_repl_empty_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
        assert!(repl.execute_line("").is_err());
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
                secure: false,
            }],
            options: vec![],
            implementation: "greet_handler".to_string(),
        };

        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(
                &self,
                _ctx: &mut dyn ExecutionContext,
                args: &HashMap<String, String>,
            ) -> Result<()> {
                assert_eq!(args.get("name"), Some(&"Alice".to_string()));
                Ok(())
            }
        }

        registry.register(cmd_def, Box::new(GreetHandler)).unwrap();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
        assert!(repl.execute_line("greet Alice").is_ok());
    }

    // ── History path ──────────────────────────────────────────────────────────

    #[test]
    fn test_repl_history_path() {
        let path = ReplInterface::get_history_path("myapp");
        if let Some(p) = path {
            let path_str = p.to_str().unwrap();
            assert!(path_str.contains("myapp"), "path should contain app name");
            assert!(
                path_str.ends_with("history"),
                "path should end with 'history', got: {}",
                path_str
            );
        }
    }

    // ── Help interception ─────────────────────────────────────────────────────

    #[test]
    fn test_try_handle_help_without_formatter_returns_none() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();
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
        let repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        let out = repl.try_handle_help("--help");
        assert!(out.is_some());
        let out = out.unwrap();
        assert!(out.contains("testapp"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn test_try_handle_help_short_flag() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();
        let repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
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
        let repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        let out = repl.try_handle_help("--help hello");
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));
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
        let repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        let out = repl.try_handle_help("hello --help");
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));
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
        let repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        let out = repl.try_handle_help("--help hi");
        assert!(out.is_some());
        assert!(out.unwrap().contains("hello"));
    }

    #[test]
    fn test_execute_line_help_intercepted() {
        use crate::help::DefaultHelpFormatter;
        colored::control::set_override(false);
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();
        let mut repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        assert!(repl.execute_line("--help").is_ok());
    }

    #[test]
    fn test_execute_line_normal_command_still_works_with_formatter() {
        use crate::help::DefaultHelpFormatter;
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();
        let mut repl = ReplInterface::new(
            registry,
            context,
            "test".to_string(),
            Some(config),
            Some(Box::new(DefaultHelpFormatter::new())),
        )
        .unwrap();
        assert!(repl.execute_line("test").is_ok());
    }

    // ── Tab completion ────────────────────────────────────────────────────────

    #[test]
    fn test_completer_commands_empty_input() {
        let registry = Arc::new(create_test_registry());
        let completer = DcliCompleter::new(Arc::clone(&registry), None);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = rustyline::Context::new(&history);
        let (_, candidates) = completer.complete("", 0, &ctx).unwrap();
        let names: Vec<&str> = candidates.iter().map(|p| p.display.as_str()).collect();
        assert!(names.contains(&"test"));
        assert!(names.contains(&"t"));
    }

    #[test]
    fn test_completer_commands_prefix_filter() {
        let registry = Arc::new(create_test_registry());
        let completer = DcliCompleter::new(Arc::clone(&registry), None);
        let history = rustyline::history::DefaultHistory::new();
        let ctx = rustyline::Context::new(&history);
        let (_, candidates) = completer.complete("te", 2, &ctx).unwrap();
        let names: Vec<&str> = candidates.iter().map(|p| p.display.as_str()).collect();
        assert!(names.contains(&"test"));
        assert!(!names.contains(&"t"));
    }

    #[test]
    fn test_completer_flags_after_command() {
        let config = Arc::new(make_help_config());
        // Registry with "hello" command
        let mut registry = CommandRegistry::new();
        let cmd_def = make_help_config().commands.into_iter().next().unwrap();
        struct DummyHandler;
        impl crate::executor::CommandHandler for DummyHandler {
            fn execute(
                &self,
                _: &mut dyn ExecutionContext,
                _: &HashMap<String, String>,
            ) -> Result<()> {
                Ok(())
            }
        }
        registry.register(cmd_def, Box::new(DummyHandler)).unwrap();
        let registry = Arc::new(registry);

        let completer = DcliCompleter::new(Arc::clone(&registry), Some(Arc::clone(&config)));
        let history = rustyline::history::DefaultHistory::new();
        let ctx = rustyline::Context::new(&history);

        // "hello " → should propose --loud and -l
        let (_, candidates) = completer.complete("hello ", 6, &ctx).unwrap();
        let names: Vec<&str> = candidates.iter().map(|p| p.display.as_str()).collect();
        assert!(
            names.contains(&"--loud"),
            "expected --loud, got {:?}",
            names
        );
        assert!(names.contains(&"-l"), "expected -l, got {:?}", names);
    }

    #[test]
    fn test_completer_flags_prefix_filter() {
        let config = Arc::new(make_help_config());
        let mut registry = CommandRegistry::new();
        let cmd_def = make_help_config().commands.into_iter().next().unwrap();
        struct DummyHandler;
        impl crate::executor::CommandHandler for DummyHandler {
            fn execute(
                &self,
                _: &mut dyn ExecutionContext,
                _: &HashMap<String, String>,
            ) -> Result<()> {
                Ok(())
            }
        }
        registry.register(cmd_def, Box::new(DummyHandler)).unwrap();
        let registry = Arc::new(registry);

        let completer = DcliCompleter::new(Arc::clone(&registry), Some(Arc::clone(&config)));
        let history = rustyline::history::DefaultHistory::new();
        let ctx = rustyline::Context::new(&history);

        // "hello --l" → only --loud
        let (_, candidates) = completer.complete("hello --l", 9, &ctx).unwrap();
        let names: Vec<&str> = candidates.iter().map(|p| p.display.as_str()).collect();
        assert!(names.contains(&"--loud"));
        assert!(!names.contains(&"-l"));
    }

    #[test]
    fn test_completer_no_flags_for_unknown_command() {
        let config = Arc::new(make_help_config());
        let registry = Arc::new(create_test_registry());
        let completer = DcliCompleter::new(Arc::clone(&registry), Some(Arc::clone(&config)));
        let history = rustyline::history::DefaultHistory::new();
        let ctx = rustyline::Context::new(&history);
        // "unknown " → empty (command not in registry)
        let (_, candidates) = completer.complete("unknown ", 8, &ctx).unwrap();
        assert!(candidates.is_empty());
    }
}
