//! CLI (Command-Line Interface) implementation
//!
//! This module provides a simple CLI interface that parses command-line
//! arguments, executes the corresponding command, and exits.
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::interface::CliInterface;
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
//! let cli = CliInterface::new(registry, context);
//! cli.run(std::env::args().skip(1).collect())?;
//! # Ok(())
//! # }
//! ```

use crate::context::ExecutionContext;
use crate::error::{display_error, DynamicCliError, ExecutionError, Result};
use crate::parser::{CliParser, ParsedArgs, ReplParser};
use crate::registry::CommandRegistry;
use std::path::Path;
use std::process;

/// CLI (Command-Line Interface) handler
///
/// Provides a simple interface for executing commands from command-line arguments.
/// The CLI parses arguments, executes the command, and exits.
///
/// # Architecture
///
/// ```text
/// Command-line args → CliParser → CommandExecutor → Handler
///                                       ↓
///                                  ExecutionContext
/// ```
///
/// # Error Handling
///
/// Errors are displayed to stderr with colored formatting (if enabled)
/// and the process exits with appropriate exit codes:
/// - `0`: Success
/// - `1`: Execution error
/// - `2`: Argument parsing error
/// - `3`: Other errors
pub struct CliInterface {
    /// Command registry containing all available commands
    registry: CommandRegistry,

    /// Execution context (owned by the interface)
    context: Box<dyn ExecutionContext>,
}

impl CliInterface {
    /// Create a new CLI interface
    ///
    /// # Arguments
    ///
    /// * `registry` - Command registry with all registered commands
    /// * `context` - Execution context (will be consumed by the interface)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    ///
    /// let cli = CliInterface::new(registry, context);
    /// ```
    pub fn new(registry: CommandRegistry, context: Box<dyn ExecutionContext>) -> Self {
        Self { registry, context }
    }

    /// Run the CLI with provided arguments
    ///
    /// Parses the arguments, executes the corresponding command, and handles errors.
    /// This method consumes `self` as the CLI typically runs once and exits.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments (typically from `env::args().skip(1)`)
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err(DynamicCliError)` on any error (parsing, validation, execution)
    ///
    /// # Exit Codes
    ///
    /// The caller should handle errors and exit with appropriate codes:
    /// - Parse errors → exit code 2
    /// - Execution errors → exit code 1
    /// - Other errors → exit code 3
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    /// use std::process;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    /// let cli = CliInterface::new(registry, context);
    ///
    /// if let Err(e) = cli.run(std::env::args().skip(1).collect()) {
    ///     eprintln!("Error: {}", e);
    ///     process::exit(1);
    /// }
    /// # }
    /// ```
    pub fn run(mut self, args: Vec<String>) -> Result<()> {
        // Handle empty arguments (show help or error)
        if args.is_empty() {
            return Err(DynamicCliError::Parse(
                crate::error::ParseError::InvalidSyntax {
                    details: "No command specified".to_string(),
                    hint: Some("Try 'help' to see available commands".to_string()),
                },
            ));
        }

        self.dispatch(&args)
    }

    /// Resolve, parse, and execute a single already-tokenized command line.
    ///
    /// Shared by [`run`][Self::run] (one dispatch from CLI args) and
    /// [`run_script`][Self::run_script] (one dispatch per script line) —
    /// the actual resolution/parsing/execution logic lives here exactly
    /// once, per DD-024's "reuse the existing `ParsedArgs` path, no
    /// duplicate parsing logic" requirement (see #41).
    fn dispatch(&mut self, args: &[String]) -> Result<()> {
        // First argument is the command name
        let command_name = &args[0];

        // Resolve command name (handles aliases)
        let resolved_name = self.registry.resolve_name(command_name).ok_or_else(|| {
            crate::error::ParseError::unknown_command_with_suggestions(
                command_name,
                &self
                    .registry
                    .list_commands()
                    .iter()
                    .map(|cmd| cmd.name.clone())
                    .collect::<Vec<_>>(),
            )
        })?;

        // Get command definition
        let definition = self.registry.get_definition(resolved_name).ok_or_else(|| {
            DynamicCliError::Registry(crate::error::RegistryError::missing_handler(resolved_name))
        })?;

        // Parse arguments using CLI parser (DD-024/#39: typed to preserve
        // repeatable-option occurrences; ParsedArgs is the shape every
        // handler now receives).
        let parser = CliParser::new(definition);
        let parsed_args = ParsedArgs::new(parser.parse_typed(&args[1..])?);

        // Get handler and execute command. Sync is tried first (unchanged
        // behaviour); if no sync handler matches, fall through to the async
        // path (DD-022) and drive it via `block_on`. Safe here because
        // `run()`/`run_script()` are strictly sequential, one-shot dispatch —
        // there is no other async task waiting behind it that `block_on`
        // could starve.
        if let Some(handler) = self.registry.get_handler_sync(resolved_name) {
            handler.execute(&mut *self.context, &parsed_args)?;
        } else if let Some(handler) = self.registry.get_handler_async(resolved_name) {
            futures::executor::block_on(handler.execute(&mut *self.context, &parsed_args))?;
        } else {
            return Err(DynamicCliError::Execution(
                crate::error::ExecutionError::handler_not_found(
                    resolved_name,
                    &definition.implementation,
                ),
            ));
        }

        Ok(())
    }

    /// Run the CLI with automatic error handling and exit
    ///
    /// This is a convenience method that:
    /// 1. Runs the CLI with provided arguments
    /// 2. Handles errors by displaying them to stderr
    /// 3. Exits the process with appropriate exit code
    ///
    /// This method never returns.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::CliInterface;
    /// use dynamic_cli::prelude::*;
    ///
    /// # #[derive(Default)]
    /// # struct MyContext;
    /// # impl ExecutionContext for MyContext {
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// #     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    /// # }
    /// # fn main() {
    /// let registry = CommandRegistry::new();
    /// let context = Box::new(MyContext::default());
    /// let cli = CliInterface::new(registry, context);
    ///
    /// // This will handle errors and exit automatically
    /// cli.run_and_exit(std::env::args().skip(1).collect());
    /// # }
    /// ```
    pub fn run_and_exit(self, args: Vec<String>) -> ! {
        match self.run(args) {
            Ok(()) => process::exit(0),
            Err(e) => {
                display_error(&e);

                // Exit with appropriate code based on error type
                let exit_code = match e {
                    DynamicCliError::Parse(_) => 2,
                    DynamicCliError::Validation(_) => 2,
                    DynamicCliError::Execution(_) => 1,
                    _ => 3,
                };

                process::exit(exit_code);
            }
        }
    }

    /// Run a batch of command lines read from a file (#41).
    ///
    /// Each non-blank, non-comment (`#`-prefixed) line is tokenized the
    /// same quote-aware way as a typed REPL line (via
    /// [`ReplParser::tokenize`]), then dispatched through the exact same
    /// resolve → parse → execute path as [`run`][Self::run] — no
    /// duplicate parsing logic, and repeatable options (DD-024) are fully
    /// preserved since dispatch goes through `parse_typed()` either way.
    ///
    /// # Error policy
    ///
    /// `policy` decides what happens when a line fails:
    /// - [`ScriptErrorPolicy::Abort`]: stop immediately, returning `Err`
    ///   for the failing line. Lines before it have already run.
    /// - [`ScriptErrorPolicy::Continue`]: record the failure and proceed
    ///   to the next line. The method still returns `Ok`, with every
    ///   failure listed in the returned [`ScriptOutcome`].
    ///
    /// Every failure — whether it aborts the run or not — is reported
    /// with its 1-based line number, wrapped in
    /// [`ExecutionError::CommandFailed`][crate::error::ExecutionError::CommandFailed]
    /// (reusing the existing error hierarchy; no new enum variant, so no
    /// breaking change to `ExecutionError`'s non-`#[non_exhaustive]`
    /// shape).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::interface::{CliInterface, ScriptErrorPolicy};
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
    /// let cli = CliInterface::new(registry, context);
    ///
    /// let outcome = cli.run_script("commands.txt", ScriptErrorPolicy::Continue)?;
    /// println!("{}/{} lines succeeded", outcome.lines_succeeded, outcome.lines_executed);
    /// # Ok(())
    /// # }
    /// ```
    pub fn run_script(
        mut self,
        path: impl AsRef<Path>,
        policy: ScriptErrorPolicy,
    ) -> Result<ScriptOutcome> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
                "failed to read script file {}: {}",
                path.display(),
                e
            )))
        })?;

        let mut outcome = ScriptOutcome {
            lines_executed: 0,
            lines_succeeded: 0,
            failures: Vec::new(),
        };

        for (idx, raw_line) in content.lines().enumerate() {
            let line_number = idx + 1;
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            outcome.lines_executed += 1;

            // Scoped so the borrow of `self.registry` ends before
            // `self.dispatch(&mut self, ...)` needs exclusive access below.
            // `tokenize` is a pure function of the line text — it doesn't
            // read `self.registry` — but it lives on `ReplParser`, so a
            // throwaway instance is the reuse path rather than duplicating
            // the quote-handling logic here.
            let tokens_result = {
                let tokenizer = ReplParser::new(&self.registry);
                tokenizer.tokenize(line)
            };

            let tokens = match tokens_result {
                Ok(t) => t,
                Err(e) => {
                    let wrapped = wrap_line_error(line_number, e);
                    if policy == ScriptErrorPolicy::Abort {
                        return Err(wrapped);
                    }
                    outcome.failures.push((line_number, wrapped));
                    continue;
                }
            };

            if tokens.is_empty() {
                continue;
            }

            match self.dispatch(&tokens) {
                Ok(()) => outcome.lines_succeeded += 1,
                Err(e) => {
                    let wrapped = wrap_line_error(line_number, e);
                    if policy == ScriptErrorPolicy::Abort {
                        return Err(wrapped);
                    }
                    outcome.failures.push((line_number, wrapped));
                }
            }
        }

        Ok(outcome)
    }
}

/// Wrap an error with its 1-based script line number, reusing the
/// existing [`ExecutionError::CommandFailed`] variant so adding
/// line-number context never requires a breaking change to the error
/// hierarchy.
fn wrap_line_error(line_number: usize, source: DynamicCliError) -> DynamicCliError {
    DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
        "line {}: {}",
        line_number,
        source
    )))
}

/// What [`CliInterface::run_script`] does when a line fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptErrorPolicy {
    /// Stop at the first failing line — [`run_script`][CliInterface::run_script]
    /// returns `Err` immediately, with the lines before it already run.
    Abort,
    /// Record the failure and keep going —
    /// [`run_script`][CliInterface::run_script] returns `Ok` with every
    /// failure listed in [`ScriptOutcome::failures`].
    Continue,
}

/// Result of a full [`CliInterface::run_script`] run.
#[derive(Debug)]
pub struct ScriptOutcome {
    /// Number of non-blank, non-comment lines dispatched (attempted).
    pub lines_executed: usize,
    /// Number of those lines that succeeded.
    pub lines_succeeded: usize,
    /// `(1-based line number, wrapped error)` for every line that failed.
    /// Always empty when `policy` was
    /// [`ScriptErrorPolicy::Abort`][ScriptErrorPolicy::Abort] and the run
    /// completed (an abort returns `Err` instead of populating this).
    pub failures: Vec<(usize, DynamicCliError)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentDefinition, ArgumentType, CommandDefinition};

    // Test context
    #[derive(Default)]
    struct TestContext {
        executed_command: Option<String>,
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
        fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .expect("Failed to downcast context");
            ctx.executed_command = Some(self.name.clone());
            Ok(())
        }
    }

    fn create_test_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::new();

        // Create a simple command definition
        let cmd_def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec!["t".to_string()],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        };

        let handler = Box::new(TestHandler {
            name: "test".to_string(),
        });

        registry
            .register_sync(cmd_def, handler)
            .expect("Failed to register command");

        registry
    }

    #[test]
    fn test_cli_interface_creation() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());

        let _cli = CliInterface::new(registry, context);
        // If this compiles and runs, creation works
    }

    #[test]
    fn test_cli_run_simple_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run(vec!["test".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_run_with_alias() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run(vec!["t".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cli_empty_args() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run(vec![]);
        assert!(result.is_err());

        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::InvalidSyntax { .. }) => {}
            other => panic!("Expected InvalidSyntax error, got: {:?}", other),
        }
    }

    #[test]
    fn test_cli_unknown_command() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run(vec!["unknown".to_string()]);
        assert!(result.is_err());

        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::UnknownCommand { .. }) => {}
            other => panic!("Expected UnknownCommand error, got: {:?}", other),
        }
    }

    #[test]
    fn test_cli_command_with_args() {
        let mut registry = CommandRegistry::new();

        // Command with argument
        let cmd_def = CommandDefinition {
            name: "greet".to_string(),
            aliases: vec![],
            description: "Greet someone".to_string(),
            required: false,
            arguments: vec![ArgumentDefinition {
                name: "name".to_string(),
                arg_type: ArgumentType::String,
                required: true,
                description: "Name to greet".to_string(),
                validation: vec![],
                secure: false,
            }],
            options: vec![],
            implementation: "greet_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        };

        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(
                &self,
                _context: &mut dyn ExecutionContext,
                args: &ParsedArgs,
            ) -> Result<()> {
                assert_eq!(args.get_scalar("name"), Some("Alice"));
                Ok(())
            }
        }

        registry
            .register_sync(cmd_def, Box::new(GreetHandler))
            .unwrap();

        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run(vec!["greet".to_string(), "Alice".to_string()]);
        assert!(result.is_ok());
    }

    // ========================================================================
    // run_script tests (#41)
    // ========================================================================

    fn write_script(content: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().expect("failed to create temp script file");
        file.write_all(content.as_bytes())
            .expect("failed to write temp script file");
        file
    }

    #[test]
    fn test_run_script_all_lines_succeed() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let script = write_script("test\nt\ntest\n");
        let outcome = cli
            .run_script(script.path(), ScriptErrorPolicy::Abort)
            .expect("run_script should succeed when every line succeeds");

        assert_eq!(outcome.lines_executed, 3);
        assert_eq!(outcome.lines_succeeded, 3);
        assert!(outcome.failures.is_empty());
    }

    #[test]
    fn test_run_script_skips_blank_lines_and_comments() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let script = write_script("# a comment\n\ntest\n   \n# another\nt\n");
        let outcome = cli
            .run_script(script.path(), ScriptErrorPolicy::Abort)
            .expect("run_script should succeed");

        // Only the two real command lines count.
        assert_eq!(outcome.lines_executed, 2);
        assert_eq!(outcome.lines_succeeded, 2);
    }

    #[test]
    fn test_run_script_continue_policy_records_failures_and_keeps_going() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let script = write_script("test\nunknown_command\ntest\n");
        let outcome = cli
            .run_script(script.path(), ScriptErrorPolicy::Continue)
            .expect("Continue policy should return Ok even with a failing line");

        assert_eq!(outcome.lines_executed, 3);
        assert_eq!(outcome.lines_succeeded, 2);
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].0, 2); // 1-based line number
    }

    #[test]
    fn test_run_script_abort_policy_stops_at_first_failure() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        // A third "test" line would succeed if reached — it must not be.
        let script = write_script("test\nunknown_command\ntest\n");
        let result = cli.run_script(script.path(), ScriptErrorPolicy::Abort);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Execution(ExecutionError::CommandFailed(e)) => {
                assert!(e.to_string().contains("line 2"));
            }
            other => panic!("Expected wrapped CommandFailed error, got: {:?}", other),
        }
    }

    #[test]
    fn test_run_script_respects_quoted_tokens() {
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
                description: "Name to greet".to_string(),
                validation: vec![],
                secure: false,
            }],
            options: vec![],
            implementation: "greet_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        };

        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(
                &self,
                _context: &mut dyn ExecutionContext,
                args: &ParsedArgs,
            ) -> Result<()> {
                assert_eq!(args.get_scalar("name"), Some("Alice Wonderland"));
                Ok(())
            }
        }

        registry
            .register_sync(cmd_def, Box::new(GreetHandler))
            .unwrap();

        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let script = write_script(r#"greet "Alice Wonderland""#);
        let outcome = cli
            .run_script(script.path(), ScriptErrorPolicy::Abort)
            .expect("quoted argument should tokenize as a single value");

        assert_eq!(outcome.lines_succeeded, 1);
    }

    #[test]
    fn test_run_script_missing_file() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let result = cli.run_script("/nonexistent/path/to/script.txt", ScriptErrorPolicy::Abort);
        assert!(result.is_err());
    }
}
