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
use crate::error::{display_error, format_error, DynamicCliError, ExecutionError, Result};
use crate::parser::{CliParser, ParsedArgs, ReplParser};
use crate::registry::CommandRegistry;
use std::path::Path;
use std::process;

/// One resolved, fully-parsed command within a (possibly single-command)
/// chain (DD-026, #52) — the unit [`CliInterface::segment`] produces and
/// [`CliInterface::execute_segment`] consumes.
///
/// `name` is owned rather than borrowed from the registry: `resolve_name`
/// / `get_definition` are cheap, stateless lookups (no per-name state to
/// track across a chain — the same name may legitimately appear more
/// than once), so re-resolving by owned `String` at execution time avoids
/// tying this struct to the registry's borrow for the whole dispatch.
#[derive(Debug)]
struct ResolvedSegment {
    /// Canonical (alias-resolved) command name.
    name: String,
    /// Already-typed, already-validated arguments for this command.
    parsed: ParsedArgs,
}

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

    /// Resolve, parse, and execute an already-tokenized command line — one
    /// or more chained commands (DD-026, #52).
    ///
    /// Shared by [`run`][Self::run] (one dispatch from CLI args) and
    /// [`run_script`][Self::run_script] (one dispatch per script line) —
    /// the actual resolution/parsing/execution logic lives here exactly
    /// once, per DD-024's "reuse the existing `ParsedArgs` path, no
    /// duplicate parsing logic" requirement (see #41). `run_script()`
    /// gains chaining for free through this shared method, with no code
    /// change of its own.
    ///
    /// [`Self::segment`] resolves and parses every command in the line up
    /// front (so a genuinely too-long single command still errors exactly
    /// as before chaining existed). A single, non-chained command
    /// (`total == 1`) then executes through the exact pre-#55/#56 path —
    /// no chain-position wrapping, no skip bookkeeping, identical error
    /// variants for existing callers to match on. Two or more segments go
    /// through [`Self::execute_chain`], which applies DD-026's
    /// `continue_on_failure`/`requires_success` policy.
    fn dispatch(&mut self, args: &[String]) -> Result<()> {
        let segments = self.segment(args)?;

        if segments.len() == 1 {
            return self.execute_segment(&segments[0]);
        }

        self.execute_chain(&segments)
    }

    /// Execute two or more already-resolved segments applying DD-026's
    /// chain failure policy (#56).
    ///
    /// A running `chain_has_failure` flag, set on the first segment that
    /// fails (regardless of that segment's own `continue_on_failure`),
    /// drives two things for every later segment:
    ///
    /// - If the segment's `requires_success` is `true` and a failure has
    ///   already occurred anywhere earlier in the chain, it is **skipped**
    ///   — not executed, not counted as an additional failure — and
    ///   reported with `Skipped: command {n}/{total} ('{name}') — a
    ///   preceding command failed`, printed immediately (there is no
    ///   other way to surface a skip, since it never produces an `Err`).
    /// - Otherwise the segment executes as usual
    ///   ([`Self::execute_segment`]). On failure, the error is wrapped
    ///   with its chain position (`Error in command {n}/{total}
    ///   ('{name}'): {existing format_error output}`, reusing
    ///   `format_error`/`display_error` — no change to
    ///   `error/display.rs`). If this segment's own `continue_on_failure`
    ///   is `false`, the chain stops here. Either way, only the *first*
    ///   failure's wrapped error is kept as the chain's outcome — a later
    ///   failure (whether it stops the chain or is itself absorbed) never
    ///   overwrites it, matching DD-026's "the exit code reflects the
    ///   triggering failure" rule.
    ///
    /// Deliberately not printed as it happens: unlike a skip, a failure's
    /// wrapped message reaches the user exactly once, through the normal
    /// `Err` return path (either immediately here, or from the caller
    /// once this method returns it at the end of the loop) — printing it
    /// again here would double it up.
    fn execute_chain(&mut self, segments: &[ResolvedSegment]) -> Result<()> {
        let total = segments.len();
        let mut chain_has_failure = false;
        let mut triggering_failure: Option<DynamicCliError> = None;

        for (idx, segment) in segments.iter().enumerate() {
            let position = idx + 1;

            let (requires_success, continue_on_failure) = self
                .registry
                .get_definition(&segment.name)
                .map(|d| (d.requires_success, d.continue_on_failure))
                .unwrap_or((false, false));

            if chain_has_failure && requires_success {
                eprintln!(
                    "Skipped: command {}/{} ('{}') — a preceding command failed",
                    position, total, segment.name
                );
                continue;
            }

            if let Err(e) = self.execute_segment(segment) {
                let wrapped = wrap_chain_error(position, total, &segment.name, e);

                if !chain_has_failure {
                    triggering_failure = Some(wrapped);
                }
                chain_has_failure = true;

                if !continue_on_failure {
                    break;
                }
            }
        }

        match triggering_failure {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Resolve and parse a full, already-tokenized command line into one
    /// or more [`ResolvedSegment`]s, without executing any of them
    /// (DD-026, #52 / #55).
    ///
    /// For the current segment, the parser consumes its options and
    /// positional arguments up to the command's declared arity
    /// ([`CliParser::parse_typed_segment`], #54). Once arity is
    /// exhausted, the next bare token is looked up against
    /// [`CommandRegistry::resolve_name`] (aliases included): a match
    /// starts the next segment; no match raises the same
    /// [`crate::error::ParseError::too_many_arguments`] a single,
    /// non-chained command would raise today — no observable behaviour
    /// change for existing callers. A genuinely unknown command name is
    /// reported via
    /// [`crate::error::ParseError::unknown_command_with_suggestions`],
    /// exactly as before chaining existed.
    ///
    /// `resolve_name`/`get_definition` are stateless lookups, so the same
    /// command name (or one of its aliases) may legitimately resolve more
    /// than once within a single chain — nothing here tracks "already
    /// consumed" names, nor should it (DD-026's explicit acceptance
    /// criterion for #55/#56).
    ///
    /// **Known, accepted limitation (DD-026):** if a command line supplies
    /// one token more than that command's declared arity, and that
    /// leftover token happens to also be a registered command name, it is
    /// silently absorbed as the start of the next segment instead of
    /// raising `too_many_arguments` — segmentation cannot distinguish "a
    /// stray extra value" from "the next command" once arity is
    /// exhausted. No `--`-style local escape is implemented; documented
    /// as an accepted constraint, not scheduled for a fix.
    fn segment(&self, args: &[String]) -> Result<Vec<ResolvedSegment>> {
        let mut segments = Vec::new();
        let mut offset = 0;

        loop {
            let command_name = &args[offset];

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

            let definition = self.registry.get_definition(resolved_name).ok_or_else(|| {
                DynamicCliError::Registry(crate::error::RegistryError::missing_handler(
                    resolved_name,
                ))
            })?;

            let parser = CliParser::new(definition);
            let (parsed_map, consumed) = parser.parse_typed_segment(&args[offset + 1..])?;

            segments.push(ResolvedSegment {
                name: resolved_name.to_string(),
                parsed: ParsedArgs::new(parsed_map),
            });

            let next = offset + 1 + consumed;
            if next == args.len() {
                break;
            }

            if self.registry.resolve_name(&args[next]).is_none() {
                return Err(crate::error::ParseError::too_many_arguments(
                    &definition.name,
                    definition.arguments.len(),
                    definition.arguments.len() + 1,
                )
                .into());
            }

            offset = next;
        }

        Ok(segments)
    }

    /// Execute one already-resolved, already-parsed segment.
    ///
    /// Sync handler tried first (unchanged behaviour); if absent, the
    /// async path (DD-022) is driven via `block_on` — safe here because
    /// `run()`/`run_script()` are strictly sequential, one-shot dispatch,
    /// per segment exactly as for a single command before chaining
    /// existed.
    fn execute_segment(&mut self, segment: &ResolvedSegment) -> Result<()> {
        if let Some(handler) = self.registry.get_handler_sync(&segment.name) {
            handler.execute(&mut *self.context, &segment.parsed)?;
        } else if let Some(handler) = self.registry.get_handler_async(&segment.name) {
            futures::executor::block_on(handler.execute(&mut *self.context, &segment.parsed))?;
        } else {
            // segment() already resolved this name to a definition, so
            // this branch means a command is registered (schema-wise)
            // without either a sync or async handler — re-fetched here
            // only for the implementation name in the error message.
            let implementation = self
                .registry
                .get_definition(&segment.name)
                .map(|d| d.implementation.as_str())
                .unwrap_or("");
            return Err(DynamicCliError::Execution(
                crate::error::ExecutionError::handler_not_found(&segment.name, implementation),
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

/// Wrap a chain segment's failure with its 1-based position (DD-026,
/// #52 / #56), reusing the existing [`ExecutionError::CommandFailed`]
/// variant — same idiom as [`wrap_line_error`] — and the existing
/// [`format_error`] for the inner message, so no change to
/// `error/display.rs` is needed. Position (not name) is what
/// distinguishes two failures of the same repeated command at different
/// points in a chain.
fn wrap_chain_error(
    position: usize,
    total: usize,
    name: &str,
    source: DynamicCliError,
) -> DynamicCliError {
    DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
        "Error in command {}/{} ('{}'): {}",
        position,
        total,
        name,
        format_error(&source)
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
        // Ordered record of every handler executed so far — additive,
        // needed to assert chain execution order (DD-026, #52 / #55)
        // without disturbing `executed_command` (kept for any existing
        // single-dispatch assertions).
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
        fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .expect("Failed to downcast context");
            ctx.executed_command = Some(self.name.clone());
            ctx.executed_commands.push(self.name.clone());
            Ok(())
        }
    }

    /// A handler that always fails, recording the attempt first — needed
    /// to exercise `continue_on_failure`/`requires_success` (DD-026,
    /// #52 / #56), which only ever activate downstream of a failure.
    struct FailingHandler {
        name: String,
    }

    impl crate::executor::CommandHandler for FailingHandler {
        fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
            let ctx = crate::context::downcast_mut::<TestContext>(context)
                .expect("Failed to downcast context");
            ctx.executed_commands.push(self.name.clone());
            Err(DynamicCliError::Execution(ExecutionError::CommandFailed(
                anyhow::anyhow!("{} deliberately failed", self.name),
            )))
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

    // ========================================================================
    // segment() / dispatch() chaining tests (DD-026, #52 / #55)
    // ========================================================================

    /// Register a command taking exactly `arity` required `String`
    /// positional arguments (`arg0`, `arg1`, ...) and no options, backed
    /// by a [`TestHandler`] that records its name (and, in order, into
    /// [`TestContext::executed_commands`]).
    fn register_arity_command(registry: &mut CommandRegistry, name: &str, arity: usize) {
        let arguments = (0..arity)
            .map(|i| ArgumentDefinition {
                name: format!("arg{}", i),
                arg_type: ArgumentType::String,
                required: true,
                description: format!("Argument {}", i),
                validation: vec![],
                secure: false,
            })
            .collect();

        let cmd_def = CommandDefinition {
            name: name.to_string(),
            aliases: vec![],
            description: format!("Test command {}", name),
            required: false,
            arguments,
            options: vec![],
            implementation: format!("{}_handler", name),
            continue_on_failure: false,
            requires_success: false,
        };

        registry
            .register_sync(
                cmd_def,
                Box::new(TestHandler {
                    name: name.to_string(),
                }),
            )
            .expect("Failed to register command");
    }

    #[test]
    fn test_segment_single_command_produces_one_segment() {
        // Single-command case (today's behaviour): exactly one segment,
        // correctly parsed.
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "greet", 1);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let args = vec!["greet".to_string(), "Alice".to_string()];
        let segments = cli.segment(&args).unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].name, "greet");
        assert_eq!(segments[0].parsed.get_scalar("arg0"), Some("Alice"));
    }

    #[test]
    fn test_segment_single_command_overflow_still_raises_too_many_arguments() {
        // A genuinely too-long single command (no chain intended, the
        // leftover token isn't a registered command) must raise the
        // identical error dispatch() raised before chaining existed.
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "greet", 1);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let args = vec![
            "greet".to_string(),
            "Alice".to_string(),
            "extra".to_string(),
        ];
        let result = cli.segment(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::TooManyArguments {
                command,
                expected,
                got,
                ..
            }) => {
                assert_eq!(command, "greet");
                assert_eq!(expected, 1);
                assert_eq!(got, 2);
            }
            other => panic!("Expected TooManyArguments error, got: {:?}", other),
        }
    }

    #[test]
    fn test_segment_multi_command_chain_produces_three_segments() {
        // Generic three-command chain (structurally the same shape as
        // DD-026's chrom-rs-motivated example — a couple of
        // argument-taking commands followed by a zero-arity terminal
        // command — but with arbitrary names, since chrom-rs is only ever
        // an illustration, never the justification).
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "first", 1);
        register_arity_command(&mut registry, "second", 1);
        register_arity_command(&mut registry, "third", 0);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let args = vec![
            "first".to_string(),
            "1".to_string(),
            "second".to_string(),
            "2".to_string(),
            "third".to_string(),
        ];
        let segments = cli.segment(&args).unwrap();

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].name, "first");
        assert_eq!(segments[0].parsed.get_scalar("arg0"), Some("1"));
        assert_eq!(segments[1].name, "second");
        assert_eq!(segments[1].parsed.get_scalar("arg0"), Some("2"));
        assert_eq!(segments[2].name, "third");
    }

    #[test]
    fn test_segment_unknown_command_produces_unknown_command_error() {
        // Unchanged behaviour: a name that resolves to nothing at the
        // start of a segment (first position here — the only position
        // structurally reachable, since a later boundary token that
        // fails to resolve is by construction reported as
        // too_many_arguments against the preceding segment instead, not
        // as unknown_command) still raises the existing suggestion-aware
        // error.
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let args = vec!["nope".to_string()];
        let result = cli.segment(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Parse(crate::error::ParseError::UnknownCommand { .. }) => {}
            other => panic!("Expected UnknownCommand error, got: {:?}", other),
        }
    }

    #[test]
    fn test_segment_repeated_command_name_resolves_each_occurrence_independently() {
        // resolve_name()/get_definition() are stateless lookups: the same
        // command name may legitimately appear more than once in a
        // single chain, each occurrence carrying its own arguments.
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "source", 1);
        register_arity_command(&mut registry, "run", 0);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let args = vec![
            "source".to_string(),
            "modelfile".to_string(),
            "source".to_string(),
            "solverfile".to_string(),
            "run".to_string(),
        ];
        let segments = cli.segment(&args).unwrap();

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].name, "source");
        assert_eq!(segments[0].parsed.get_scalar("arg0"), Some("modelfile"));
        assert_eq!(segments[1].name, "source");
        assert_eq!(segments[1].parsed.get_scalar("arg0"), Some("solverfile"));
        assert_eq!(segments[2].name, "run");
    }

    #[test]
    fn test_dispatch_executes_chain_in_order() {
        // End-to-end: segmentation feeding execute_segment() actually
        // runs every resolved segment, in order — not just parses them.
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "first", 1);
        register_arity_command(&mut registry, "second", 1);
        register_arity_command(&mut registry, "third", 0);
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        let args = vec![
            "first".to_string(),
            "1".to_string(),
            "second".to_string(),
            "2".to_string(),
            "third".to_string(),
        ];
        cli.dispatch(&args).expect("chain should execute fully");

        let ctx = crate::context::downcast_ref::<TestContext>(&*cli.context)
            .expect("Failed to downcast context");
        assert_eq!(
            ctx.executed_commands,
            vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string()
            ]
        );
    }

    #[test]
    fn test_segment_known_limitation_extra_token_matching_command_name_is_silently_absorbed() {
        // DD-026's documented, accepted limitation: one token more than a
        // command's declared arity, which happens to also be a
        // registered command name, is silently read as the start of the
        // next segment instead of raising too_many_arguments.
        // Deliberately reproduced and pinned down here as *expected*
        // (not a bug to fix) — see DD-026's "Known limitation" note.
        let mut registry = CommandRegistry::new();
        register_arity_command(&mut registry, "greet", 1);
        register_arity_command(&mut registry, "run", 0);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        // Intent could plausibly have been "greet Alice" with a stray
        // trailing "run" (typo, or a genuinely too-long command) — but
        // because "run" is also a registered zero-arity command, it is
        // read as the next segment rather than reported as an error.
        let args = vec!["greet".to_string(), "Alice".to_string(), "run".to_string()];
        let segments = cli
            .segment(&args)
            .expect("known limitation: no error is raised here, by design");

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].name, "greet");
        assert_eq!(segments[0].parsed.get_scalar("arg0"), Some("Alice"));
        assert_eq!(segments[1].name, "run");
    }

    // ========================================================================
    // execute_chain() — continue_on_failure / requires_success (DD-026, #52 / #56)
    // ========================================================================

    /// Register a zero-arity command with the given chain-policy fields,
    /// backed by [`FailingHandler`] when `fails` is `true` or
    /// [`TestHandler`] otherwise.
    fn register_chain_command(
        registry: &mut CommandRegistry,
        name: &str,
        continue_on_failure: bool,
        requires_success: bool,
        fails: bool,
    ) {
        let cmd_def = CommandDefinition {
            name: name.to_string(),
            aliases: vec![],
            description: format!("Test command {}", name),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: format!("{}_handler", name),
            continue_on_failure,
            requires_success,
        };

        let handler: Box<dyn crate::executor::CommandHandler> = if fails {
            Box::new(FailingHandler {
                name: name.to_string(),
            })
        } else {
            Box::new(TestHandler {
                name: name.to_string(),
            })
        };

        registry
            .register_sync(cmd_def, handler)
            .expect("Failed to register command");
    }

    #[test]
    fn test_execute_chain_continue_on_failure_false_stops_chain() {
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "a", false, false, true); // fails, does not absorb
        register_chain_command(&mut registry, "b", false, false, false);
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        let args = vec!["a".to_string(), "b".to_string()];
        let result = cli.dispatch(&args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Error in command 1/2 ('a')"));

        let ctx = crate::context::downcast_ref::<TestContext>(&*cli.context).unwrap();
        assert_eq!(
            ctx.executed_commands,
            vec!["a".to_string()],
            "'b' must never run once 'a' stops the chain"
        );
    }

    #[test]
    fn test_execute_chain_continue_on_failure_true_proceeds_and_still_errors() {
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "a", true, false, true); // fails, absorbed
        register_chain_command(&mut registry, "b", false, false, false);
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        let args = vec!["a".to_string(), "b".to_string()];
        let result = cli.dispatch(&args);

        // The chain still reports Err overall (exit code must not be 0
        // just because every segment was *attempted*), but it's the
        // triggering ('a') failure that's reported.
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Error in command 1/2 ('a')"));

        let ctx = crate::context::downcast_ref::<TestContext>(&*cli.context).unwrap();
        assert_eq!(
            ctx.executed_commands,
            vec!["a".to_string(), "b".to_string()],
            "'b' must still run: 'a''s failure was absorbed"
        );
    }

    #[test]
    fn test_execute_chain_requires_success_skips_after_earlier_failure() {
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "a", true, false, true); // fails, absorbed
        register_chain_command(&mut registry, "b", false, true, false); // requires_success
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        let args = vec!["a".to_string(), "b".to_string()];
        let result = cli.dispatch(&args);

        assert!(result.is_err());
        let ctx = crate::context::downcast_ref::<TestContext>(&*cli.context).unwrap();
        assert_eq!(
            ctx.executed_commands,
            vec!["a".to_string()],
            "'b' must be skipped, not executed, once 'a' has failed"
        );
    }

    #[test]
    fn test_execute_chain_requires_success_runs_normally_without_a_preceding_failure() {
        // requires_success is moot when nothing earlier in the chain has
        // failed — the segment runs exactly as if the flag were absent.
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "a", false, false, false); // succeeds
        register_chain_command(&mut registry, "b", false, true, false); // requires_success, succeeds
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        let args = vec!["a".to_string(), "b".to_string()];
        cli.dispatch(&args)
            .expect("no failure anywhere in the chain");

        let ctx = crate::context::downcast_ref::<TestContext>(&*cli.context).unwrap();
        assert_eq!(
            ctx.executed_commands,
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn test_execute_chain_reports_repeated_command_name_by_position_not_name_early() {
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "ok", false, false, false);
        register_chain_command(&mut registry, "source", true, false, true); // fails, absorbed
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        // "source" fails at position 2 of 4.
        let args = vec![
            "ok".to_string(),
            "source".to_string(),
            "ok".to_string(),
            "ok".to_string(),
        ];
        let result = cli.dispatch(&args);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("Error in command 2/4 ('source')"));
        assert!(!message.contains("4/4"));
    }

    #[test]
    fn test_execute_chain_reports_repeated_command_name_by_position_not_name_late() {
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "ok", false, false, false);
        register_chain_command(&mut registry, "source", true, false, true); // fails, absorbed
        let context = Box::new(TestContext::default());
        let mut cli = CliInterface::new(registry, context);

        // Same two command names as the previous test, but "source" fails
        // at position 4 of 4 this time — the message must reflect *this*
        // position, not collide with or get deduplicated against the
        // other test's "2/4" message.
        let args = vec![
            "ok".to_string(),
            "ok".to_string(),
            "ok".to_string(),
            "source".to_string(),
        ];
        let result = cli.dispatch(&args);

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("Error in command 4/4 ('source')"));
        assert!(!message.contains("2/4"));
    }

    #[test]
    fn test_run_script_chain_failure_reports_chain_position_and_line_number() {
        // Integration: run_script() itself needs no code change (#56) —
        // a chain inside a single script line is dispatched through the
        // same dispatch()/execute_chain() path, and wrap_line_error()
        // (unchanged) wraps whatever dispatch() returns, so the final
        // message carries both the line number and the chain position.
        let mut registry = CommandRegistry::new();
        register_chain_command(&mut registry, "a", false, false, true); // fails
        register_chain_command(&mut registry, "b", false, false, false);
        let context = Box::new(TestContext::default());
        let cli = CliInterface::new(registry, context);

        let script = write_script("a b\n");
        let outcome = cli
            .run_script(script.path(), ScriptErrorPolicy::Continue)
            .expect("Continue policy should return Ok even with a failing line");

        assert_eq!(outcome.failures.len(), 1);
        let (line_number, error) = &outcome.failures[0];
        assert_eq!(*line_number, 1);
        let message = error.to_string();
        assert!(message.contains("line 1"));
        assert!(message.contains("Error in command 1/2 ('a')"));
    }
}
