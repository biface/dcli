//! REPL directives
//!
//! A REPL line starting with `:` is a framework directive; any other line is
//! an application command. The set of directives is closed for a given
//! version of the crate and described by [`ReplDirective`]. An application
//! may rename a directive, change its aliases or its description through the
//! optional `directives:` section of its configuration, but it can neither
//! add nor disable one.
//!
//! # Default directives
//!
//! | `implementation` | Name   | Aliases  | Arguments   |
//! |------------------|--------|----------|-------------|
//! | `repl_help`      | `help` | `h`, `?` | `[command]` |
//! | `repl_load`      | `load` |          | `<path>`    |
//! | `repl_quit`      | `quit` | `q`      |             |
//! | `repl_exit`      | `exit` |          |             |
//!
//! Names and aliases are written without the leading `:`; the REPL adds it.
//!
//! # Overriding a directive
//!
//! ```yaml
//! directives:
//!   - implementation: repl_help
//!     name: aide
//!     aliases: [a]
//!     description: "Afficher l'aide"
//! ```
//!
//! An entry replaces the name, the aliases and the description of the
//! directive selected by `implementation`; `aliases` defaults to an empty
//! list. Directives without an entry keep their defaults.
//! [`effective_directives`] computes the resulting table.

// Design: DA-030 (#92). The directive set is an enum rather than a trait:
// commands are an open set supplied by the application, directives a closed
// set that needs REPL internals (editor, history, loop control). A new
// directive is a new variant, and the exhaustive matches below make the
// compiler point at every place that must handle it.

use serde::{Deserialize, Serialize};

/// Prefix that marks a REPL line as a directive
pub(crate) const DIRECTIVE_PREFIX: char = ':';

/// Framework directive available in the REPL
///
/// Deserialized from the `implementation` field of a `directives:` entry
/// (`repl_help`, `repl_load`, `repl_quit`, `repl_exit`). An unknown value
/// fails at load time, and the error lists the accepted values.
///
/// The enum is `#[non_exhaustive]`: later versions may add directives.
///
/// # Example
///
/// ```
/// use dynamic_cli::config::ReplDirective;
///
/// assert_eq!(ReplDirective::Help.default_name(), "help");
/// assert_eq!(ReplDirective::Help.default_aliases(), &["h", "?"]);
/// assert_eq!(ReplDirective::Help.usage(), "[command]");
/// assert_eq!(ReplDirective::Help.as_str(), "repl_help");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[non_exhaustive]
pub enum ReplDirective {
    /// Show the application help, or the help of one command
    #[serde(rename = "repl_help")]
    Help,

    /// Run every line of a script file in the current session
    #[serde(rename = "repl_load")]
    Load,

    /// Save the session history, then leave the REPL
    #[serde(rename = "repl_quit")]
    Quit,

    /// Leave the REPL without saving the session history
    #[serde(rename = "repl_exit")]
    Exit,
}

impl ReplDirective {
    /// Every directive, in the order used by [`effective_directives`] and
    /// by help listings
    pub const ALL: &'static [ReplDirective] = &[
        ReplDirective::Help,
        ReplDirective::Load,
        ReplDirective::Quit,
        ReplDirective::Exit,
    ];

    /// Identifier used in the `implementation` field of the configuration
    pub fn as_str(self) -> &'static str {
        match self {
            ReplDirective::Help => "repl_help",
            ReplDirective::Load => "repl_load",
            ReplDirective::Quit => "repl_quit",
            ReplDirective::Exit => "repl_exit",
        }
    }

    /// Name used when the configuration does not override the directive
    pub fn default_name(self) -> &'static str {
        match self {
            ReplDirective::Help => "help",
            ReplDirective::Load => "load",
            ReplDirective::Quit => "quit",
            ReplDirective::Exit => "exit",
        }
    }

    /// Aliases used when the configuration does not override the directive
    pub fn default_aliases(self) -> &'static [&'static str] {
        match self {
            ReplDirective::Help => &["h", "?"],
            ReplDirective::Load => &[],
            ReplDirective::Quit => &["q"],
            ReplDirective::Exit => &[],
        }
    }

    /// Description used when the configuration does not override the
    /// directive
    pub fn default_description(self) -> &'static str {
        match self {
            ReplDirective::Help => "Show the application help, or the help of a command",
            ReplDirective::Load => "Run every line of a script file",
            ReplDirective::Quit => "Save the session history and leave",
            ReplDirective::Exit => "Leave without saving the session history",
        }
    }

    /// Arguments accepted after the directive name, as shown in help
    ///
    /// Returns only the argument part, since the name itself may be
    /// overridden; the result is empty for a directive without arguments.
    /// Arguments are fixed by the action and cannot be configured.
    pub fn usage(self) -> &'static str {
        match self {
            ReplDirective::Help => "[command]",
            ReplDirective::Load => "<path>",
            ReplDirective::Quit => "",
            ReplDirective::Exit => "",
        }
    }
}

/// Name, aliases and description of a REPL directive
///
/// Used both for the entries of the `directives:` configuration section and
/// for the complete table returned by [`effective_directives`].
///
/// # Example YAML
///
/// ```yaml
/// implementation: repl_quit
/// name: quitter
/// aliases: [q]
/// description: "Enregistrer l'historique et quitter"
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DirectiveDefinition {
    /// Directive being defined or overridden
    pub implementation: ReplDirective,

    /// Name typed after `:` to invoke the directive
    pub name: String,

    /// Alternative names, also typed after `:`
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Human-readable description for help text
    pub description: String,
}

impl DirectiveDefinition {
    /// Definition of `directive` with its default name, aliases and
    /// description
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::config::{DirectiveDefinition, ReplDirective};
    ///
    /// let quit = DirectiveDefinition::default_for(ReplDirective::Quit);
    /// assert_eq!(quit.name, "quit");
    /// assert_eq!(quit.aliases, vec!["q".to_string()]);
    /// ```
    pub fn default_for(directive: ReplDirective) -> Self {
        Self {
            implementation: directive,
            name: directive.default_name().to_string(),
            aliases: directive
                .default_aliases()
                .iter()
                .map(|alias| alias.to_string())
                .collect(),
            description: directive.default_description().to_string(),
        }
    }
}

/// Complete directive table after applying the configuration overrides
///
/// Returns one definition per directive, in [`ReplDirective::ALL`] order: the
/// override whose `implementation` matches when there is one, the default
/// definition otherwise.
///
/// The configuration validator rejects two overrides of the same directive.
/// On an unvalidated list, the last override of a directive wins.
///
/// # Example
///
/// ```
/// use dynamic_cli::config::{effective_directives, DirectiveDefinition, ReplDirective};
///
/// let overrides = vec![DirectiveDefinition {
///     implementation: ReplDirective::Help,
///     name: "aide".to_string(),
///     aliases: vec![],
///     description: "Afficher l'aide".to_string(),
/// }];
///
/// let table = effective_directives(&overrides);
/// assert_eq!(table.len(), ReplDirective::ALL.len());
/// assert_eq!(table[0].name, "aide");
/// assert_eq!(table[2].name, "quit");
/// ```
pub fn effective_directives(overrides: &[DirectiveDefinition]) -> Vec<DirectiveDefinition> {
    ReplDirective::ALL
        .iter()
        .map(|&directive| {
            overrides
                .iter()
                .rev()
                .find(|entry| entry.implementation == directive)
                .cloned()
                .unwrap_or_else(|| DirectiveDefinition::default_for(directive))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn entry(directive: ReplDirective, name: &str, aliases: &[&str]) -> DirectiveDefinition {
        DirectiveDefinition {
            implementation: directive,
            name: name.to_string(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            description: format!("custom {}", name),
        }
    }

    // ------------------------------------------------------------------
    // Defaults
    // ------------------------------------------------------------------

    #[test]
    fn test_all_lists_each_variant_once() {
        let unique: HashSet<_> = ReplDirective::ALL.iter().collect();
        assert_eq!(unique.len(), ReplDirective::ALL.len());
        assert_eq!(ReplDirective::ALL.len(), 4);
    }

    #[test]
    fn test_every_directive_has_default_name_and_description() {
        for &directive in ReplDirective::ALL {
            assert!(!directive.default_name().is_empty(), "{:?}", directive);
            assert!(
                !directive.default_description().is_empty(),
                "{:?}",
                directive
            );
            assert!(directive.as_str().starts_with("repl_"), "{:?}", directive);
        }
    }

    #[test]
    fn test_default_values_per_directive() {
        assert_eq!(ReplDirective::Help.default_name(), "help");
        assert_eq!(ReplDirective::Help.default_aliases(), &["h", "?"]);
        assert_eq!(ReplDirective::Help.usage(), "[command]");

        assert_eq!(ReplDirective::Load.default_name(), "load");
        assert!(ReplDirective::Load.default_aliases().is_empty());
        assert_eq!(ReplDirective::Load.usage(), "<path>");

        assert_eq!(ReplDirective::Quit.default_name(), "quit");
        assert_eq!(ReplDirective::Quit.default_aliases(), &["q"]);
        assert_eq!(ReplDirective::Quit.usage(), "");

        assert_eq!(ReplDirective::Exit.default_name(), "exit");
        assert!(ReplDirective::Exit.default_aliases().is_empty());
        assert_eq!(ReplDirective::Exit.usage(), "");
    }

    #[test]
    fn test_default_names_and_aliases_are_disjoint_and_valid() {
        let mut seen = HashSet::new();
        for &directive in ReplDirective::ALL {
            let tokens = std::iter::once(directive.default_name())
                .chain(directive.default_aliases().iter().copied());
            for token in tokens {
                assert!(!token.is_empty());
                assert!(!token.starts_with(':'));
                assert!(!token.chars().any(char::is_whitespace));
                assert!(seen.insert(token), "'{}' used twice", token);
            }
        }
    }

    #[test]
    fn test_usage_has_no_surrounding_whitespace() {
        for &directive in ReplDirective::ALL {
            assert_eq!(directive.usage(), directive.usage().trim());
        }
    }

    #[test]
    fn test_default_for_matches_accessors() {
        for &directive in ReplDirective::ALL {
            let def = DirectiveDefinition::default_for(directive);
            assert_eq!(def.implementation, directive);
            assert_eq!(def.name, directive.default_name());
            assert_eq!(def.aliases, directive.default_aliases());
            assert_eq!(def.description, directive.default_description());
        }
    }

    // ------------------------------------------------------------------
    // Serde
    // ------------------------------------------------------------------

    #[test]
    fn test_serde_identifiers_match_as_str() {
        for &directive in ReplDirective::ALL {
            let yaml = serde_yaml::to_string(&directive).unwrap();
            assert_eq!(yaml.trim(), directive.as_str());
            let back: ReplDirective = serde_yaml::from_str(directive.as_str()).unwrap();
            assert_eq!(back, directive);
        }
    }

    #[test]
    fn test_deserialize_definition_aliases_default_empty() {
        let yaml = r#"
implementation: repl_exit
name: sortir
description: "Sortir sans enregistrer"
"#;
        let def: DirectiveDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.implementation, ReplDirective::Exit);
        assert_eq!(def.name, "sortir");
        assert!(def.aliases.is_empty());
    }

    #[test]
    fn test_deserialize_definition_requires_name_and_description() {
        let no_name = "implementation: repl_help\ndescription: x\n";
        assert!(serde_yaml::from_str::<DirectiveDefinition>(no_name).is_err());
        let no_description = "implementation: repl_help\nname: aide\n";
        assert!(serde_yaml::from_str::<DirectiveDefinition>(no_description).is_err());
    }

    #[test]
    fn test_deserialize_unknown_implementation_lists_valid_values() {
        let yaml = "implementation: repl_history\nname: hist\ndescription: x\n";
        let err = serde_yaml::from_str::<DirectiveDefinition>(yaml)
            .unwrap_err()
            .to_string();
        assert!(err.contains("repl_history"), "{}", err);
        for &directive in ReplDirective::ALL {
            assert!(err.contains(directive.as_str()), "{}", err);
        }
    }

    // ------------------------------------------------------------------
    // Merge
    // ------------------------------------------------------------------

    #[test]
    fn test_effective_directives_without_override() {
        let table = effective_directives(&[]);
        let expected: Vec<_> = ReplDirective::ALL
            .iter()
            .map(|&d| DirectiveDefinition::default_for(d))
            .collect();
        assert_eq!(table, expected);
    }

    #[test]
    fn test_effective_directives_partial_override() {
        let help = entry(ReplDirective::Help, "aide", &[]);
        let table = effective_directives(std::slice::from_ref(&help));

        assert_eq!(table.len(), ReplDirective::ALL.len());
        assert_eq!(table[0], help);
        // Overriding replaces the aliases: `h` and `?` are gone.
        assert!(table[0].aliases.is_empty());
        for (def, &directive) in table.iter().zip(ReplDirective::ALL).skip(1) {
            assert_eq!(*def, DirectiveDefinition::default_for(directive));
        }
    }

    #[test]
    fn test_effective_directives_full_override_keeps_all_order() {
        // Overrides given in reverse order; the table follows `ALL`.
        let overrides = vec![
            entry(ReplDirective::Exit, "sortir", &[]),
            entry(ReplDirective::Quit, "quitter", &["q"]),
            entry(ReplDirective::Load, "charger", &["c"]),
            entry(ReplDirective::Help, "aide", &["a", "?"]),
        ];
        let table = effective_directives(&overrides);

        let names: Vec<_> = table.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["aide", "charger", "quitter", "sortir"]);
        let order: Vec<_> = table.iter().map(|d| d.implementation).collect();
        assert_eq!(order, ReplDirective::ALL);
    }

    #[test]
    fn test_effective_directives_last_duplicate_wins() {
        let overrides = vec![
            entry(ReplDirective::Quit, "first", &[]),
            entry(ReplDirective::Quit, "second", &[]),
        ];
        let table = effective_directives(&overrides);
        assert_eq!(table[2].name, "second");
    }
}
