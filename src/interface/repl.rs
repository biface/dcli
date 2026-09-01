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
use crate::error::{display_error, DynamicCliError, ExecutionError, ParseError, Result};
use crate::help::HelpFormatter;
use crate::parser::{ParsedArgs, ReplParser};
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

    /// Fully rendered prompt string (e.g., "myapp > "), built at construction
    /// time from the `prompt` argument and `prompt_suffix` below.
    prompt: String,

    /// Suffix appended after the app-name segment to build both `prompt`
    /// and the default multi-line continuation prompt (DD-027, #67).
    ///
    /// Sourced from `config.metadata.prompt_suffix` when a config is
    /// supplied, falling back to [`crate::config::schema::default_prompt_suffix`] otherwise —
    /// stored separately (rather than re-parsed out of `prompt`) so the
    /// multi-line default stays correct regardless of what the configured
    /// suffix actually is.
    prompt_suffix: String,

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

    /// Base (app-name-equivalent) segment of the continuation prompt shown
    /// while a `\`-continued command is being accumulated (DD-027, #67).
    ///
    /// `None` (the default) means the base falls back to `"..."`. Either
    /// way, `prompt_suffix` is always appended — this field overrides only
    /// the segment that replaces the app name, exactly like `prompt`
    /// (the constructor argument) only ever supplied the app-name segment
    /// of the main prompt.
    prompt_multiline: Option<String>,
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
    /// * `prompt`         — Prompt prefix (e.g., `"myapp"`). The displayed
    ///   prompt is `prompt` followed by `config.metadata.prompt_suffix`
    ///   (e.g., `"myapp > "`) when `config` is supplied, or by
    ///   [`crate::config::schema::default_prompt_suffix`] (`"myapp > "`) otherwise — the suffix is
    ///   never hardcoded independently of the config.
    /// * `config`         — Application configuration for completion, help,
    ///   and the prompt suffix. Pass `None` to disable all three.
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

        // Determine the prompt suffix *before* config is moved into the Arc
        // below — sourced from the live config when present (config-first,
        // principle 7), falling back to the same default the schema itself
        // uses when no config was supplied.
        let prompt_suffix = config
            .as_ref()
            .map(|c| c.metadata.prompt_suffix.clone())
            .unwrap_or_else(crate::config::schema::default_prompt_suffix);

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
            prompt: format!("{}{}", prompt, prompt_suffix),
            prompt_suffix,
            editor,
            history_path,
            config,
            help_formatter,
            prompt_multiline: None,
        };

        repl.load_history();

        Ok(repl)
    }

    /// Override the base segment of the continuation prompt shown while a
    /// `\`-continued command is being accumulated (DD-027, #67).
    ///
    /// `prompt` here plays the same role as the app-name segment passed to
    /// [`ReplInterface::new`] for the main prompt: only the base is
    /// supplied, `prompt_suffix` (from the config, or the default) is
    /// always appended after it by [`effective_prompt_multiline`][Self::effective_prompt_multiline].
    /// When this setter is never called, the base defaults to `"..."`.
    ///
    /// Purely additive — consuming `self` and returning `Self` like every
    /// other fluent setter in this crate; no existing `ReplInterface::new()`
    /// call site changes.
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
    /// let repl = ReplInterface::new(registry, context, "rpn".to_string(), None, None)?
    ///     .with_prompt_multiline("_ _ _".to_string());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_prompt_multiline(mut self, prompt: String) -> Self {
        self.prompt_multiline = Some(prompt);
        self
    }

    /// The prompt actually displayed while accumulating a `\`-continued
    /// command (DD-027, #67).
    ///
    /// With an override set via
    /// [`with_prompt_multiline`][Self::with_prompt_multiline], the base is
    /// used as-is, followed by `prompt_suffix` unchanged (e.g. base
    /// `"_ _ _"` + suffix `" $ "` → `"_ _ _ $ "`).
    ///
    /// Without an override, the default base `"..."` is followed by
    /// `prompt_suffix` **with its leading whitespace stripped** — matching
    /// DD-027's literal example (`"rpn > "` → `"...> "`): `"..."` takes the
    /// app name's *and* its immediately following space's place, rather
    /// than merely the app name's, so it reads as one continuous mark
    /// against the separator glyph instead of leaving a stray gap
    /// (`"... > "`). This asymmetry is deliberate — an explicit override is
    /// the caller's own spacing choice and is never second-guessed.
    fn effective_prompt_multiline(&self) -> String {
        match &self.prompt_multiline {
            Some(base) => format!("{}{}", base, self.prompt_suffix),
            None => format!("...{}", self.prompt_suffix.trim_start()),
        }
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

    /// Intercept a `:load <path>` line before normal command parsing (#41
    /// scope extension).
    ///
    /// Returns `None` when `line` doesn't start with `:load ` — normal
    /// dispatch proceeds. Returns `Some(result)` when it does, whether
    /// the load itself succeeds or fails.
    ///
    /// Unlike [`CliInterface::run_script`][crate::interface::CliInterface::run_script],
    /// there is no error-policy parameter here: a failing line is
    /// reported inline (via [`display_error`]) and the load always
    /// continues to the next line, printing a final `succeeded/attempted`
    /// summary. This matches how the REPL already surfaces errors for
    /// interactively-typed lines — one at a time, without halting the
    /// session — rather than the batch abort/continue choice that makes
    /// sense for a one-shot script run.
    ///
    /// Each loaded line is dispatched via [`execute_line`][Self::execute_line]
    /// itself — the same scalar-only path (DD-024 addendum) as any other
    /// REPL-typed line, **not**
    /// [`crate::interface::CliInterface::run_script`]'s typed/repeatable-options
    /// path. A loaded script is not added to `rustyline` history, and a
    /// script that `:load`s itself (directly or via another file) will
    /// recurse until the file handle limit or stack is exhausted — no
    /// cycle detection is implemented.
    fn try_handle_load(&mut self, line: &str) -> Option<Result<()>> {
        let path = line.trim().strip_prefix(":load ").map(str::trim)?;

        if path.is_empty() {
            return Some(Err(DynamicCliError::Parse(ParseError::InvalidSyntax {
                details: "`:load` requires a file path".to_string(),
                hint: Some("Usage: :load <path/to/script.txt>".to_string()),
            })));
        }

        Some(self.load_script(path))
    }

    /// Read `path` and dispatch each non-blank, non-comment (`#`-prefixed)
    /// line through [`execute_line`][Self::execute_line], continuing past
    /// any failure. See [`try_handle_load`][Self::try_handle_load] for the
    /// full behaviour.
    fn load_script(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
                "failed to read script file {}: {}",
                path,
                e
            )))
        })?;

        let mut attempted = 0usize;
        let mut succeeded = 0usize;

        for (idx, raw_line) in content.lines().enumerate() {
            let line_number = idx + 1;
            let script_line = raw_line.trim();

            if script_line.is_empty() || script_line.starts_with('#') {
                continue;
            }

            attempted += 1;

            match self.execute_line(script_line) {
                Ok(()) => succeeded += 1,
                Err(e) => {
                    eprintln!("  :load {} — line {}:", path, line_number);
                    display_error(&e);
                }
            }
        }

        println!(":load {path}: {succeeded}/{attempted} line(s) succeeded");
        Ok(())
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
    /// 1. Displays the prompt (or the continuation prompt while
    ///    accumulating a `\`-continued command, DD-027/#69)
    /// 2. Reads user input (with tab completion)
    /// 3. Parses and executes the command
    /// 4. Displays results or errors
    /// 5. Repeats until the user exits
    ///
    /// # Multi-line option accumulation (DD-027, #69)
    ///
    /// A line ending in a trailing `\` is not dispatched: the marker is
    /// stripped and the fragment is buffered, the prompt switches to
    /// [`effective_prompt_multiline`][Self::effective_prompt_multiline],
    /// and another line is read. The first line that does *not* end in `\`
    /// completes the buffer — every fragment plus this final one are
    /// joined with a single space and dispatched through
    /// [`execute_line`][Self::execute_line] exactly once, with the normal
    /// prompt restored. Only this fully reconstructed line (no `\`
    /// markers) can ever reach REPL history — `execute_line()` never sees
    /// a raw partial fragment.
    ///
    /// A `Ctrl+C` while accumulating discards the buffer and returns to
    /// the normal prompt, mirroring the familiar shell convention of
    /// abandoning a continued line on interrupt.
    ///
    /// This mechanism is entirely local to this loop: [`Self::execute_line`],
    /// [`ReplParser`], and `CliParser` are untouched, and non-interactive
    /// paths (`run_script()`, `:load`) never go through it — see DD-027's
    /// abort-by-construction rule for contexts without a live operator.
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
        let mut continuation_buffer: Vec<String> = Vec::new();

        loop {
            let prompt = if continuation_buffer.is_empty() {
                self.prompt.clone()
            } else {
                self.effective_prompt_multiline()
            };

            let readline = self.editor.readline(&prompt);

            match readline {
                Ok(raw_line) => {
                    let line = match Self::accumulate_line(&mut continuation_buffer, &raw_line) {
                        Some(line) => line,
                        // Still accumulating: read another line under the
                        // continuation prompt instead of dispatching.
                        None => continue,
                    };

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
                    match self.execute_line(&line) {
                        Ok(()) => {}
                        Err(e) => {
                            display_error(&e);
                        }
                    }
                }

                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    // A continued command doesn't survive an interrupt —
                    // same as a shell discarding a `\`-continued line.
                    continuation_buffer.clear();
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

    /// Feed one raw input line into the `\`-continuation buffer (DD-027,
    /// #69).
    ///
    /// Pure and I/O-free by design: [`run`][Self::run] is the only caller,
    /// but keeping this separate from the `rustyline`-driven loop makes the
    /// accumulation semantics themselves unit-testable without an
    /// interactive terminal.
    ///
    /// # Returns
    ///
    /// - `None` — `raw_line` ended in `\`; the marker was stripped and the
    ///   fragment pushed onto `buffer`. The caller should read another line
    ///   under the continuation prompt.
    /// - `Some(line)` — `raw_line` did not end in `\`, completing the
    ///   buffer. `buffer` is drained and every fragment (in order) plus
    ///   `raw_line` are joined with a single space; this is the
    ///   reconstructed line the caller should dispatch. When `buffer` was
    ///   already empty, `line` is simply `raw_line.trim()` — the ordinary,
    ///   non-continued case.
    fn accumulate_line(buffer: &mut Vec<String>, raw_line: &str) -> Option<String> {
        let trimmed = raw_line.trim();

        if let Some(fragment) = trimmed.strip_suffix('\\') {
            buffer.push(fragment.trim_end().to_string());
            return None;
        }

        if buffer.is_empty() {
            return Some(trimmed.to_string());
        }

        buffer.push(trimmed.to_string());
        Some(std::mem::take(buffer).join(" "))
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

        if let Some(result) = self.try_handle_load(line) {
            return result;
        }

        let parser = ReplParser::new(&self.registry);
        let parsed = parser.parse_line(line)?;

        // Write to history only on successful parse and when no secure
        // argument is present in the parsed command.
        if !self.has_secure_arg(&parsed.command_name, &parsed.arguments) {
            let _ = self.editor.add_history_entry(line);
        }

        // Sync tried first (unchanged behaviour), then async via `block_on`
        // (DD-022). Safe here because the REPL loop is strictly sequential
        // — one command finishes (readline blocks regardless) before the
        // next line is even read, so there is no other async task waiting
        // that `block_on` could starve.
        //
        // Wrapped via `from_scalars`: `ReplParser::parse_line` still
        // produces a plain `HashMap<String, String>` (DD-024 addendum —
        // repeatable options have no interactive REPL-typing use case, see
        // `DESIGN_DECISIONS.md`). Every handler receives `&ParsedArgs`
        // regardless of dispatch path (#39); the REPL path just never
        // populates `ParsedValue::Repeated` entries.
        let parsed_args = ParsedArgs::from_scalars(parsed.arguments);
        if let Some(handler) = self.registry.get_handler_sync(&parsed.command_name) {
            handler.execute(&mut *self.context, &parsed_args)?;
        } else if let Some(handler) = self.registry.get_handler_async(&parsed.command_name) {
            futures::executor::block_on(handler.execute(&mut *self.context, &parsed_args))?;
        } else {
            return Err(DynamicCliError::Execution(
                ExecutionError::handler_not_found(&parsed.command_name, "unknown"),
            ));
        }

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
    use rustyline::history::History;
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
        fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
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
            continue_on_failure: false,
            requires_success: false,
        };
        registry
            .register_sync(
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
                    repeatable: false,
                    option_parameters: HashMap::new(),
                }],
                implementation: "hello_handler".to_string(),
                continue_on_failure: false,
                requires_success: false,
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
            continue_on_failure: false,
            requires_success: false,
        };

        struct GreetHandler;
        impl crate::executor::CommandHandler for GreetHandler {
            fn execute(&self, _ctx: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
                assert_eq!(args.get_scalar("name"), Some("Alice"));
                Ok(())
            }
        }

        registry
            .register_sync(cmd_def, Box::new(GreetHandler))
            .unwrap();
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
            fn execute(&self, _: &mut dyn ExecutionContext, _: &ParsedArgs) -> Result<()> {
                Ok(())
            }
        }
        registry
            .register_sync(cmd_def, Box::new(DummyHandler))
            .unwrap();
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
            fn execute(&self, _: &mut dyn ExecutionContext, _: &ParsedArgs) -> Result<()> {
                Ok(())
            }
        }
        registry
            .register_sync(cmd_def, Box::new(DummyHandler))
            .unwrap();
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

    // ── has_secure_arg ────────────────────────────────────────────────────────

    /// Build a registry + config with one command that has a `secure` argument.
    fn make_secure_registry_and_config() -> (CommandRegistry, CommandsConfig) {
        use crate::config::schema::{CommandsConfig, Metadata};

        let cmd_def = CommandDefinition {
            name: "login".to_string(),
            aliases: vec![],
            description: "Login command".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "username".to_string(),
                    arg_type: ArgumentType::String,
                    required: true,
                    description: "Username".to_string(),
                    validation: vec![],
                    secure: false,
                },
                ArgumentDefinition {
                    name: "password".to_string(),
                    arg_type: ArgumentType::String,
                    required: true,
                    description: "Password".to_string(),
                    validation: vec![],
                    secure: true,
                },
            ],
            options: vec![],
            implementation: "login_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        };

        struct LoginHandler;
        impl crate::executor::CommandHandler for LoginHandler {
            fn execute(&self, _ctx: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
                Ok(())
            }
        }

        let mut registry = CommandRegistry::new();
        registry
            .register_sync(cmd_def.clone(), Box::new(LoginHandler))
            .unwrap();

        let config = CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: "testapp".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![cmd_def],
            global_options: vec![],
        };

        (registry, config)
    }

    #[test]
    fn test_has_secure_arg_returns_false_without_config() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let mut args = HashMap::new();
        args.insert("password".to_string(), "secret".to_string());

        assert!(!repl.has_secure_arg("login", &args));
    }

    #[test]
    fn test_has_secure_arg_returns_false_when_no_secure_field() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let config = make_help_config();
        let repl =
            ReplInterface::new(registry, context, "test".to_string(), Some(config), None).unwrap();

        let mut args = HashMap::new();
        args.insert("loud".to_string(), "true".to_string());

        assert!(!repl.has_secure_arg("hello", &args));
    }

    #[test]
    fn test_has_secure_arg_returns_true_when_secure_argument_present() {
        let (registry, config) = make_secure_registry_and_config();
        let context = Box::new(TestContext::default());
        let repl =
            ReplInterface::new(registry, context, "test".to_string(), Some(config), None).unwrap();

        let mut args = HashMap::new();
        args.insert("username".to_string(), "alice".to_string());
        args.insert("password".to_string(), "secret".to_string());

        assert!(repl.has_secure_arg("login", &args));
    }

    #[test]
    fn test_has_secure_arg_returns_false_when_only_non_secure_present() {
        let (registry, config) = make_secure_registry_and_config();
        let context = Box::new(TestContext::default());
        let repl =
            ReplInterface::new(registry, context, "test".to_string(), Some(config), None).unwrap();

        // Only username provided — password (secure) absent from parsed args.
        let mut args = HashMap::new();
        args.insert("username".to_string(), "alice".to_string());

        assert!(!repl.has_secure_arg("login", &args));
    }

    #[test]
    fn test_has_secure_arg_returns_false_for_unknown_command() {
        let (registry, config) = make_secure_registry_and_config();
        let context = Box::new(TestContext::default());
        let repl =
            ReplInterface::new(registry, context, "test".to_string(), Some(config), None).unwrap();

        let mut args = HashMap::new();
        args.insert("password".to_string(), "secret".to_string());

        assert!(!repl.has_secure_arg("nonexistent", &args));
    }

    // ── Secure argument history filtering ─────────────────────────────────────

    #[test]
    fn test_execute_line_with_secure_arg_does_not_add_to_history() {
        let (registry, config) = make_secure_registry_and_config();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), Some(config), None).unwrap();

        let result = repl.execute_line("login alice secret");
        assert!(result.is_ok());

        // The line must NOT appear in the in-memory history.
        let history = repl.editor.history();
        let in_history = (0..history.len()).any(|i| {
            history
                .get(i, rustyline::history::SearchDirection::Forward)
                .ok()
                .flatten()
                .map(|e| e.entry.as_ref() == "login alice secret")
                .unwrap_or(false)
        });
        assert!(
            !in_history,
            "secure command line must not be written to history"
        );
    }

    #[test]
    fn test_execute_line_without_secure_arg_adds_to_history() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let result = repl.execute_line("test");
        assert!(result.is_ok());

        // The line must appear in the in-memory history.
        let history = repl.editor.history();
        let in_history = (0..history.len()).any(|i| {
            history
                .get(i, rustyline::history::SearchDirection::Forward)
                .ok()
                .flatten()
                .map(|e| e.entry.as_ref() == "test")
                .unwrap_or(false)
        });
        assert!(
            in_history,
            "non-secure command line must be written to history"
        );
    }

    // ── :load (#41 scope extension) ─────────────────────────────────────────

    fn write_script(content: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().expect("failed to create temp script file");
        file.write_all(content.as_bytes())
            .expect("failed to write temp script file");
        file
    }

    #[test]
    fn test_load_executes_each_line_via_execute_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let script = write_script("test\nt\n");
        let line = format!(":load {}", script.path().display());

        assert!(repl.execute_line(&line).is_ok());

        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert_eq!(ctx.executed_commands, vec!["test", "test"]);
    }

    #[test]
    fn test_load_skips_blank_lines_and_comments() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let script = write_script("# a comment\n\ntest\n   \n# another\n");
        let line = format!(":load {}", script.path().display());

        assert!(repl.execute_line(&line).is_ok());

        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert_eq!(ctx.executed_commands, vec!["test"]);
    }

    #[test]
    fn test_load_continues_past_a_failing_line() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let script = write_script("test\nunknown_command\ntest\n");
        let line = format!(":load {}", script.path().display());

        // Unlike CliInterface::run_script(Abort), :load never returns Err
        // just because a line inside it failed — the failure is displayed
        // inline and the load proceeds to the next line.
        let result = repl.execute_line(&line);
        assert!(result.is_ok());

        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert_eq!(ctx.executed_commands, vec!["test", "test"]);
    }

    #[test]
    fn test_load_missing_path_argument_is_an_error() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let result = repl.execute_line(":load");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_file_is_an_error() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let result = repl.execute_line(":load /nonexistent/path/to/script.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_line_itself_is_not_added_to_history() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let script = write_script("test\n");
        let line = format!(":load {}", script.path().display());
        assert!(repl.execute_line(&line).is_ok());

        let history = repl.editor.history();
        let load_in_history = (0..history.len()).any(|i| {
            history
                .get(i, rustyline::history::SearchDirection::Forward)
                .ok()
                .flatten()
                .map(|e| e.entry.starts_with(":load"))
                .unwrap_or(false)
        });
        assert!(
            !load_in_history,
            ":load line itself must not be written to history"
        );
    }

    // ========================================================================
    // No accidental scope leak from CLI chaining (DD-026, #52 / #56)
    // ========================================================================

    #[test]
    fn test_repl_line_is_never_chained_across_multiple_commands() {
        // execute_line() (and, through it, :load) goes through
        // ReplParser::parse_line() -> CliParser::parse() (the scalar,
        // pre-DD-026 method) — never CliInterface::dispatch()'s
        // segmentation. A line naming two registered commands back to
        // back must therefore still be read as ONE command whose arity
        // is exceeded by the second name, exactly as before chaining
        // existed for the CLI — not as two chained commands.
        let mut registry = CommandRegistry::new();
        for name in ["first", "second"] {
            let cmd_def = CommandDefinition {
                name: name.to_string(),
                aliases: vec![],
                description: format!("Test command {}", name),
                required: false,
                arguments: vec![ArgumentDefinition {
                    name: "value".to_string(),
                    arg_type: ArgumentType::String,
                    required: true,
                    description: "Value".to_string(),
                    validation: vec![],
                    secure: false,
                }],
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
                .unwrap();
        }

        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        // "first" has arity 1; "1" fills it, leaving "second" as a
        // genuine overflow — never a second, chained command.
        let result = repl.execute_line("first 1 second");

        assert!(result.is_err());
        match result.unwrap_err() {
            DynamicCliError::Parse(ParseError::TooManyArguments { command, .. }) => {
                assert_eq!(command, "first");
            }
            other => panic!("Expected TooManyArguments error, got: {:?}", other),
        }

        // Neither command actually ran — in particular, "second" was
        // never silently executed as a chained segment.
        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        assert!(ctx.executed_commands.is_empty());
    }

    // ========================================================================
    // Prompt suffix bugfix + multi-line continuation prompt (DD-027, #67)
    // ========================================================================

    #[test]
    fn test_prompt_uses_default_suffix_without_config() {
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "myapp".to_string(), None, None).unwrap();

        // No config supplied: falls back to the schema's own default,
        // not an independently hardcoded literal.
        assert_eq!(repl.prompt, "myapp > ");
    }

    #[test]
    fn test_prompt_honours_configured_suffix() {
        // Regression test for the bug where `config.metadata.prompt_suffix`
        // was silently ignored in favour of a hardcoded " > ".
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut config = make_help_config();
        config.metadata.prompt_suffix = " $ ".to_string();

        let repl =
            ReplInterface::new(registry, context, "rpn".to_string(), Some(config), None).unwrap();

        assert_eq!(repl.prompt, "rpn $ ");
    }

    #[test]
    fn test_effective_prompt_multiline_default_derivation() {
        // The exact example from DD-027's closing checklist: default
        // suffix " > " -> multi-line default "...> ".
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let repl = ReplInterface::new(registry, context, "rpn".to_string(), None, None).unwrap();

        assert_eq!(repl.prompt, "rpn > ");
        assert_eq!(repl.effective_prompt_multiline(), "...> ");
    }

    #[test]
    fn test_effective_prompt_multiline_uses_configured_suffix() {
        // The multi-line default must track a *custom* suffix too, not just
        // the " > " default — same bug class as the main prompt. Leading
        // whitespace of the suffix is still stripped for the default "..."
        // base, same as the " > " case.
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut config = make_help_config();
        config.metadata.prompt_suffix = " $ ".to_string();

        let repl =
            ReplInterface::new(registry, context, "rpn".to_string(), Some(config), None).unwrap();

        assert_eq!(repl.effective_prompt_multiline(), "...$ ");
    }

    #[test]
    fn test_with_prompt_multiline_overrides_base_only() {
        // Overriding only replaces the "..." base — the configured suffix
        // is still appended automatically.
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut config = make_help_config();
        config.metadata.prompt_suffix = " $ ".to_string();

        let repl = ReplInterface::new(registry, context, "rpn".to_string(), Some(config), None)
            .unwrap()
            .with_prompt_multiline("_ _ _".to_string());

        assert_eq!(repl.effective_prompt_multiline(), "_ _ _ $ ");
    }

    // ========================================================================
    // `\`-continuation accumulation logic (DD-027, #69)
    // ========================================================================

    #[test]
    fn test_accumulate_line_single_line_no_continuation() {
        let mut buffer: Vec<String> = Vec::new();
        let result = ReplInterface::accumulate_line(&mut buffer, "test");

        assert_eq!(result, Some("test".to_string()));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_accumulate_line_trailing_backslash_buffers_and_returns_none() {
        let mut buffer: Vec<String> = Vec::new();
        let result = ReplInterface::accumulate_line(&mut buffer, "push \\");

        assert_eq!(result, None);
        assert_eq!(buffer, vec!["push".to_string()]);
    }

    #[test]
    fn test_accumulate_line_multi_fragment_reconstruction() {
        // Three lines: two continued, one final — mirrors the DD-027
        // example of a command whose options span several REPL lines.
        let mut buffer: Vec<String> = Vec::new();

        assert_eq!(
            ReplInterface::accumulate_line(&mut buffer, "config \\"),
            None
        );
        assert_eq!(
            ReplInterface::accumulate_line(&mut buffer, "--format model \\"),
            None
        );
        let result = ReplInterface::accumulate_line(&mut buffer, "source=model.yml");

        assert_eq!(
            result,
            Some("config --format model source=model.yml".to_string())
        );
        // The buffer is drained once the command completes.
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_accumulate_line_strips_marker_and_surrounding_whitespace() {
        // The `\` marker and the whitespace immediately before it are never
        // part of the reconstructed line — fragments are joined by exactly
        // one space, not whatever spacing the user typed.
        let mut buffer: Vec<String> = Vec::new();
        ReplInterface::accumulate_line(&mut buffer, "test   \\");
        let result = ReplInterface::accumulate_line(&mut buffer, "  arg");

        assert_eq!(result, Some("test arg".to_string()));
    }

    #[test]
    fn test_run_multiline_reconstruction_reaches_execute_line_once() {
        // Simulates what `run()` does with each `readline()` result,
        // without needing a real terminal: feed the same raw lines a real
        // session would produce, dispatch only once the buffer completes,
        // exactly as `run()` itself does.
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let mut buffer: Vec<String> = Vec::new();
        assert_eq!(ReplInterface::accumulate_line(&mut buffer, "te \\"), None);
        let line = ReplInterface::accumulate_line(&mut buffer, "st").unwrap();
        assert_eq!(line, "te st");

        // "te st" isn't a registered command name — proves the reconstructed
        // line, not raw fragments, is what actually gets dispatched.
        let result = repl.execute_line(&line);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_line_ending_in_backslash_is_not_continued() {
        // :load bypasses run()'s accumulation buffer entirely — each script
        // line goes straight to execute_line(), one at a time. A trailing
        // `\` in a script has no special meaning there (DD-027 scopes
        // continuation to the interactive `run()` loop only).
        let registry = create_test_registry();
        let context = Box::new(TestContext::default());
        let mut repl =
            ReplInterface::new(registry, context, "test".to_string(), None, None).unwrap();

        let script = write_script("test \\\ntest\n");
        let line = format!(":load {}", script.path().display());

        // The first line ("test \") is dispatched as-is — "\" is an
        // unexpected extra positional argument for the zero-arity "test"
        // command, not silently joined with the next line.
        assert!(repl.execute_line(&line).is_ok());

        let ctx = crate::context::downcast_ref::<TestContext>(&*repl.context).unwrap();
        // Only the second, unadorned "test" line actually executed.
        assert_eq!(ctx.executed_commands, vec!["test"]);
    }
}
