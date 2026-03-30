//! Dynamic help generation for CLI applications
//!
//! This module provides the [`HelpFormatter`] trait and its default
//! implementation [`DefaultHelpFormatter`], which generate help text
//! from a [`CommandsConfig`] at runtime.
//!
//! # Design
//!
//! - The trait is public and `dyn`-compatible so users can supply custom implementations.
//! - The default implementation outputs English-only text; custom implementations
//!   choose their own language.
//! - The formatter is instantiated lazily — only when `--help` is detected —
//!   and outputs to the terminal only.
//!
//! # Extension point
//!
//! Users of the framework can supply their own formatter via
//! [`CliBuilder::help_formatter()`](crate::builder::CliBuilder::help_formatter):
//!
//! ```
//! use dynamic_cli::help::HelpFormatter;
//! use dynamic_cli::config::schema::CommandsConfig;
//!
//! struct MyFormatter;
//!
//! impl HelpFormatter for MyFormatter {
//!     fn format_app(&self, config: &CommandsConfig) -> String {
//!         format!("Custom help for {}", config.metadata.prompt)
//!     }
//!
//!     fn format_command(&self, config: &CommandsConfig, command: &str) -> String {
//!         format!("Custom help for command '{command}' in {}", config.metadata.prompt)
//!     }
//! }
//! ```

use crate::config::schema::{ArgumentType, CommandDefinition, CommandsConfig};
use colored::Colorize;

// ============================================================================
// Public trait
// ============================================================================

/// Generates help text from a runtime configuration.
///
/// Both methods receive the full [`CommandsConfig`] so implementations
/// have access to metadata, commands, and global options.
///
/// # Object safety
///
/// This trait has no generic methods and no `Self` return types, so it is
/// fully `dyn`-compatible. It can be used as `Box<dyn HelpFormatter>`.
///
/// # Example
///
/// ```
/// use dynamic_cli::help::{HelpFormatter, DefaultHelpFormatter};
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
/// let formatter = DefaultHelpFormatter::new();
/// let help = formatter.format_app(&config);
/// assert!(help.contains("myapp"));
/// assert!(help.contains("1.0.0"));
/// ```
pub trait HelpFormatter {
    /// Generate help text for the whole application.
    ///
    /// Lists all commands with their descriptions, and prints usage.
    fn format_app(&self, config: &CommandsConfig) -> String;

    /// Generate help text for a single command.
    ///
    /// Looks up `command` by name **or alias** and prints its arguments,
    /// options, and aliases. If the command is not found, returns an
    /// informative error string (never panics).
    fn format_command(&self, config: &CommandsConfig, command: &str) -> String;
}

// ============================================================================
// Default implementation
// ============================================================================

/// Default help formatter — colored, aligned, English-only output.
///
/// Outputs English text to the terminal. If you need another language,
/// implement [`HelpFormatter`] and supply it via
/// [`CliBuilder::help_formatter()`](crate::builder::CliBuilder::help_formatter).
/// Uses the [`colored`] crate for terminal output. Colours are applied
/// automatically and disabled when the terminal does not support them.
///
/// Column widths are computed dynamically so that descriptions are aligned
/// regardless of command or argument name lengths.
///
/// # Example
///
/// ```
/// use dynamic_cli::help::DefaultHelpFormatter;
///
/// let fmt = DefaultHelpFormatter::new();
/// // or equivalently:
/// let fmt = DefaultHelpFormatter::default();
/// ```
#[derive(Debug, Default)]
pub struct DefaultHelpFormatter;

impl DefaultHelpFormatter {
    /// Create a new `DefaultHelpFormatter`.
    pub fn new() -> Self {
        Self
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Return the display string for an [`ArgumentType`].
    fn type_label(t: ArgumentType) -> &'static str {
        t.as_str()
    }

    /// Pad `s` to at least `width` characters with trailing spaces.
    fn pad(s: &str, width: usize) -> String {
        format!("{:<width$}", s, width = width)
    }

    /// Resolve a command by name or alias. Returns `None` if not found.
    fn find_command<'a>(config: &'a CommandsConfig, name: &str) -> Option<&'a CommandDefinition> {
        config
            .commands
            .iter()
            .find(|cmd| cmd.name == name || cmd.aliases.iter().any(|a| a == name))
    }

    /// Format the ARGUMENTS section of a command.
    fn format_arguments(cmd: &CommandDefinition) -> String {
        if cmd.arguments.is_empty() {
            return String::new();
        }

        // Compute column width from the longest argument name.
        let col_width = cmd
            .arguments
            .iter()
            .map(|a| a.name.len())
            .max()
            .unwrap_or(0)
            + 4; // minimum padding

        let mut out = format!("\n{}\n", "ARGUMENTS:".bold());
        for arg in &cmd.arguments {
            let req = if arg.required { "required" } else { "optional" };
            let label = format!("({}, {req})", Self::type_label(arg.arg_type));
            out.push_str(&format!(
                "    {}  {}  {}\n",
                Self::pad(&arg.name, col_width).green(),
                label.dimmed(),
                arg.description
            ));
        }
        out
    }

    /// Format the OPTIONS section of a command.
    fn format_options(cmd: &CommandDefinition) -> String {
        if cmd.options.is_empty() {
            return String::new();
        }

        // Build the flag string for each option, e.g. "-v, --verbose".
        let flags: Vec<String> = cmd
            .options
            .iter()
            .map(|opt| {
                let short = opt
                    .short
                    .as_deref()
                    .map(|s| format!("-{s}"))
                    .unwrap_or_default();
                let long = opt
                    .long
                    .as_deref()
                    .map(|l| format!("--{l}"))
                    .unwrap_or_default();
                match (short.is_empty(), long.is_empty()) {
                    (false, false) => format!("{short}, {long}"),
                    (false, true) => short,
                    (true, false) => long,
                    (true, true) => opt.name.clone(),
                }
            })
            .collect();

        let col_width = flags.iter().map(|f| f.len()).max().unwrap_or(0) + 4;

        let mut out = format!("\n{}\n", "OPTIONS:".bold());
        for (opt, flag) in cmd.options.iter().zip(flags.iter()) {
            let type_label = format!("({})", Self::type_label(opt.option_type));
            let default_note = opt
                .default
                .as_deref()
                .map(|d| format!(" [default: {d}]"))
                .unwrap_or_default();
            out.push_str(&format!(
                "    {}  {}  {}{}\n",
                Self::pad(flag, col_width).yellow(),
                type_label.dimmed(),
                opt.description,
                default_note.dimmed()
            ));
        }
        out
    }

    /// Format the ALIASES section of a command.
    fn format_aliases(cmd: &CommandDefinition) -> String {
        if cmd.aliases.is_empty() {
            return String::new();
        }
        format!(
            "\n{}\n    {}\n",
            "ALIASES:".bold(),
            cmd.aliases.join(", ").italic()
        )
    }

    /// Build the inline usage token for a command (e.g. `<input> [output]`).
    fn usage_args(cmd: &CommandDefinition) -> String {
        let args: String = cmd
            .arguments
            .iter()
            .map(|a| {
                if a.required {
                    format!("<{}>", a.name)
                } else {
                    format!("[{}]", a.name)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let opts = if cmd.options.is_empty() {
            String::new()
        } else {
            " [options]".to_string()
        };

        format!("{args}{opts}")
    }
}

impl HelpFormatter for DefaultHelpFormatter {
    /// Format the application-level help (list of all commands).
    ///
    /// # Output structure
    ///
    /// ```text
    /// myapp 1.0.0
    ///
    /// USAGE:
    ///     myapp <command> [arguments] [options]
    ///
    /// COMMANDS:
    ///     hello      Say hello to someone
    ///     process    Process data files
    ///
    /// Run 'myapp --help <command>' for more information on a command.
    /// ```
    fn format_app(&self, config: &CommandsConfig) -> String {
        let mut out = String::new();

        // Header: "prompt version"
        out.push_str(&format!(
            "{} {}\n",
            config.metadata.prompt.bold().cyan(),
            config.metadata.version.dimmed()
        ));

        // USAGE
        out.push('\n');
        out.push_str(&format!("{}\n", "USAGE:".bold()));
        out.push_str(&format!(
            "    {} {} [arguments] [options]\n",
            config.metadata.prompt,
            "<command>".green()
        ));

        // COMMANDS
        if !config.commands.is_empty() {
            out.push('\n');
            out.push_str(&format!("{}\n", "COMMANDS:".bold()));

            let col_width = config
                .commands
                .iter()
                .map(|c| c.name.len())
                .max()
                .unwrap_or(0)
                + 4;

            for cmd in &config.commands {
                out.push_str(&format!(
                    "    {}  {}\n",
                    Self::pad(&cmd.name, col_width).green(),
                    cmd.description
                ));
            }
        }

        // Footer hint
        out.push('\n');
        out.push_str(&format!(
            "{} '{}' {}\n",
            "Run".dimmed(),
            format!("{} --help <command>", config.metadata.prompt).italic(),
            "for more information on a command.".dimmed()
        ));

        out
    }

    /// Format help for a single command, looked up by name or alias.
    ///
    /// # Output structure
    ///
    /// ```text
    /// hello — Say hello to someone
    ///
    /// USAGE:
    ///     hello <name> [options]
    ///
    /// ARGUMENTS:
    ///     name    (string, required)  Name to greet
    ///
    /// OPTIONS:
    ///     -l, --loud    (bool)  Use uppercase
    ///
    /// ALIASES:
    ///     hi
    /// ```
    ///
    /// If the command is not found, returns a message listing available commands.
    fn format_command(&self, config: &CommandsConfig, command: &str) -> String {
        let Some(cmd) = Self::find_command(config, command) else {
            // Unknown command — list available names to guide the user.
            let available = config
                .commands
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return format!(
                "{} '{}'\n\nAvailable commands: {}\n",
                "Unknown command:".red().bold(),
                command,
                available
            );
        };

        let mut out = String::new();

        // Header: "name — description"
        out.push_str(&format!(
            "{} — {}\n",
            cmd.name.bold().cyan(),
            cmd.description
        ));

        // USAGE
        out.push('\n');
        out.push_str(&format!("{}\n", "USAGE:".bold()));
        out.push_str(&format!(
            "    {} {}\n",
            cmd.name.green(),
            Self::usage_args(cmd)
        ));

        // ARGUMENTS, OPTIONS, ALIASES (empty sections are omitted)
        out.push_str(&Self::format_arguments(cmd));
        out.push_str(&Self::format_options(cmd));
        out.push_str(&Self::format_aliases(cmd));

        out
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{
        ArgumentDefinition, ArgumentType, CommandDefinition, Metadata, OptionDefinition,
    };

    // Disable ANSI codes in tests so assertions work on plain text.
    fn no_color() {
        colored::control::set_override(false);
    }

    // -----------------------------------------------------------------------
    // Test fixtures
    // -----------------------------------------------------------------------

    fn make_config() -> CommandsConfig {
        CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: "myapp".to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: vec![
                CommandDefinition {
                    name: "hello".to_string(),
                    aliases: vec!["hi".to_string(), "hey".to_string()],
                    description: "Say hello to someone".to_string(),
                    required: false,
                    arguments: vec![ArgumentDefinition {
                        name: "name".to_string(),
                        arg_type: ArgumentType::String,
                        required: true,
                        description: "Name to greet".to_string(),
                        validation: vec![],
                    }],
                    options: vec![OptionDefinition {
                        name: "loud".to_string(),
                        short: Some("l".to_string()),
                        long: Some("loud".to_string()),
                        option_type: ArgumentType::Bool,
                        required: false,
                        default: None,
                        description: "Use uppercase".to_string(),
                        choices: vec![],
                    }],
                    implementation: "hello_handler".to_string(),
                },
                CommandDefinition {
                    name: "process".to_string(),
                    aliases: vec![],
                    description: "Process data files".to_string(),
                    required: true,
                    arguments: vec![],
                    options: vec![],
                    implementation: "process_handler".to_string(),
                },
            ],
            global_options: vec![],
        }
    }

    fn make_formatter() -> DefaultHelpFormatter {
        DefaultHelpFormatter::new()
    }

    // -----------------------------------------------------------------------
    // DefaultHelpFormatter — construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_and_default_are_equivalent() {
        // Both construction paths compile and produce the same type.
        let _a = DefaultHelpFormatter::new();
        let _b = DefaultHelpFormatter::default();
    }

    // -----------------------------------------------------------------------
    // format_app
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_app_contains_prompt_and_version() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_app(&config);

        assert!(out.contains("myapp"), "should contain prompt");
        assert!(out.contains("1.0.0"), "should contain version");
    }

    #[test]
    fn test_format_app_contains_all_commands() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_app(&config);

        assert!(out.contains("hello"), "should list command 'hello'");
        assert!(out.contains("process"), "should list command 'process'");
        assert!(
            out.contains("Say hello to someone"),
            "should include description"
        );
    }

    #[test]
    fn test_format_app_contains_usage_and_footer() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_app(&config);

        assert!(out.contains("USAGE:"), "should have USAGE section");
        assert!(out.contains("COMMANDS:"), "should have COMMANDS section");
        assert!(
            out.contains("--help <command>"),
            "should hint at per-command help"
        );
    }

    #[test]
    fn test_format_app_empty_commands() {
        no_color();
        let mut config = make_config();
        config.commands.clear();
        let out = make_formatter().format_app(&config);

        // Should still render without panicking; no COMMANDS section.
        assert!(out.contains("myapp"));
        assert!(!out.contains("COMMANDS:"));
    }

    // -----------------------------------------------------------------------
    // format_command — known command
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_command_by_name() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "hello");

        assert!(out.contains("hello"), "should contain command name");
        assert!(
            out.contains("Say hello to someone"),
            "should contain description"
        );
    }

    #[test]
    fn test_format_command_by_alias() {
        no_color();
        let config = make_config();
        // "hi" is an alias for "hello"
        let out = make_formatter().format_command(&config, "hi");

        // Resolves to the canonical command
        assert!(out.contains("hello"));
        assert!(out.contains("Say hello to someone"));
    }

    #[test]
    fn test_format_command_shows_arguments() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "hello");

        assert!(out.contains("ARGUMENTS:"), "should have ARGUMENTS section");
        assert!(out.contains("name"), "should list argument name");
        assert!(out.contains("string"), "should show argument type");
        assert!(out.contains("required"), "should show required status");
        assert!(out.contains("Name to greet"), "should show description");
    }

    #[test]
    fn test_format_command_shows_options() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "hello");

        assert!(out.contains("OPTIONS:"), "should have OPTIONS section");
        assert!(out.contains("-l"), "should show short flag");
        assert!(out.contains("--loud"), "should show long flag");
        assert!(
            out.contains("Use uppercase"),
            "should show option description"
        );
    }

    #[test]
    fn test_format_command_shows_aliases() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "hello");

        assert!(out.contains("ALIASES:"), "should have ALIASES section");
        assert!(out.contains("hi"), "should list alias 'hi'");
        assert!(out.contains("hey"), "should list alias 'hey'");
    }

    #[test]
    fn test_format_command_no_aliases_section_when_empty() {
        no_color();
        let config = make_config();
        // "process" has no aliases
        let out = make_formatter().format_command(&config, "process");

        assert!(!out.contains("ALIASES:"), "should omit ALIASES section");
    }

    #[test]
    fn test_format_command_no_arguments_section_when_empty() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "process");

        assert!(!out.contains("ARGUMENTS:"), "should omit ARGUMENTS section");
    }

    #[test]
    fn test_format_command_no_options_section_when_empty() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "process");

        assert!(!out.contains("OPTIONS:"), "should omit OPTIONS section");
    }

    // -----------------------------------------------------------------------
    // format_command — unknown command
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_command_unknown_returns_error_string() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "nonexistent");

        assert!(
            out.contains("Unknown command"),
            "should signal unknown command"
        );
        assert!(
            out.contains("nonexistent"),
            "should echo the unknown name back"
        );
    }

    #[test]
    fn test_format_command_unknown_lists_available() {
        no_color();
        let config = make_config();
        let out = make_formatter().format_command(&config, "nonexistent");

        // Should list alternatives so the user can self-correct.
        assert!(
            out.contains("hello"),
            "should list available command 'hello'"
        );
        assert!(
            out.contains("process"),
            "should list available command 'process'"
        );
    }

    // -----------------------------------------------------------------------
    // HelpFormatter trait — object safety check
    // -----------------------------------------------------------------------

    #[test]
    fn test_trait_is_dyn_compatible() {
        no_color();
        // If this compiles, the trait is object-safe.
        let formatter: Box<dyn HelpFormatter> = Box::new(DefaultHelpFormatter::new());
        let config = make_config();
        let _ = formatter.format_app(&config);
    }

    // -----------------------------------------------------------------------
    // Option default display in options section
    // -----------------------------------------------------------------------

    #[test]
    fn test_format_command_shows_default_value() {
        no_color();
        let mut config = make_config();
        // Add a default value to the 'loud' option
        config.commands[0].options[0].default = Some("false".to_string());
        let out = make_formatter().format_command(&config, "hello");

        assert!(out.contains("false"), "should show default value");
    }

    // -----------------------------------------------------------------------
    // Custom HelpFormatter implementation (framework extensibility)
    // -----------------------------------------------------------------------

    struct MinimalFormatter;

    impl HelpFormatter for MinimalFormatter {
        fn format_app(&self, config: &CommandsConfig) -> String {
            config.metadata.prompt.clone()
        }
        fn format_command(&self, _config: &CommandsConfig, command: &str) -> String {
            command.to_string()
        }
    }

    #[test]
    fn test_custom_formatter_via_trait_object() {
        let config = make_config();
        let f: Box<dyn HelpFormatter> = Box::new(MinimalFormatter);

        assert_eq!(f.format_app(&config), "myapp");
        assert_eq!(f.format_command(&config, "hello"), "hello");
    }
}
