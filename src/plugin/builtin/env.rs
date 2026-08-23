//! Standalone `EnvPlugin` (feature-gated).
//!
//! Displays environment variables, with sensitive-looking ones excluded
//! by default — never a raw `std::env::vars()` dump. Only available with
//! the `env-plugin` feature.
//!
//! # Filtering rule
//!
//! Unlike [`ArgumentDefinition::secure`][crate::config::schema::ArgumentDefinition]
//! (DD-023), which relies on an explicit `secure: true` opt-in on a
//! config-declared argument, environment variables have no such upfront
//! declaration — the set of names is arbitrary and unknown ahead of
//! time. An allow-list is therefore not viable here; this plugin instead
//! uses a **deny-list of case-insensitive substrings** applied to each
//! variable's name:
//!
//! ```text
//! SECRET, TOKEN, KEY, PASS, CREDENTIAL, AUTH, PRIVATE
//! ```
//!
//! Any variable whose name contains one of these substrings (case
//! folded) is hidden — its value is never printed, never included in
//! output, only counted. This is a heuristic, not a guarantee: an
//! oddly-named secret can still slip through. See [`is_sensitive_name`]
//! if a custom list is needed.

use crate::context::ExecutionContext;
use crate::executor::CommandHandler;
use crate::parser::ParsedArgs;
use crate::plugin::Plugin;
use crate::Result;

/// Case-insensitive substrings that mark an environment variable name as
/// sensitive. See the module-level docs for the rationale.
const SENSITIVE_NAME_SUBSTRINGS: &[&str] = &[
    "SECRET",
    "TOKEN",
    "KEY",
    "PASS",
    "CREDENTIAL",
    "AUTH",
    "PRIVATE",
];

/// Returns `true` if `name` looks sensitive under the deny-list rule
/// documented at the module level.
fn is_sensitive_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    SENSITIVE_NAME_SUBSTRINGS
        .iter()
        .any(|needle| upper.contains(needle))
}

/// Standalone plugin providing a single `env` command.
///
/// Prints environment variable names and values, skipping any name that
/// looks sensitive (see the module-level filtering rule). The variable
/// *name* itself is always shown for hidden entries — only its value is
/// withheld — so the count and presence of hidden variables stays
/// visible without leaking their contents.
///
/// # YAML config
///
/// ```yaml
/// commands:
///   - name: env
///     implementation: env_show
///     description: "Show environment variables (sensitive ones hidden)"
///     required: false
///     arguments: []
///     options: []
/// ```
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{EnvPlugin, Plugin};
///
/// let plugin = EnvPlugin::new();
/// assert_eq!(plugin.name(), "env");
///
/// let handlers = plugin.handlers();
/// assert_eq!(handlers.len(), 1);
/// assert_eq!(handlers[0].0, "env_show");
/// ```
pub struct EnvPlugin;

impl EnvPlugin {
    /// Create a new `EnvPlugin`.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{EnvPlugin, Plugin};
    ///
    /// let plugin = EnvPlugin::new();
    /// assert_eq!(plugin.name(), "env");
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for EnvPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for EnvPlugin {
    fn name(&self) -> &str {
        "env"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Filtered environment variable display (feature-gated, #46)"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        vec![("env_show".to_string(), Box::new(EnvShowHandler))]
    }
}

/// Handler for `env_show` — prints environment variables, hiding
/// sensitive-looking ones. See the module-level filtering rule.
struct EnvShowHandler;

impl CommandHandler for EnvShowHandler {
    fn execute(&self, _ctx: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let mut vars: Vec<(String, String)> = std::env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));

        let mut hidden_count = 0usize;

        for (name, value) in &vars {
            if is_sensitive_name(name) {
                hidden_count += 1;
                println!("{name} = <hidden>");
            } else {
                println!("{name} = {value}");
            }
        }

        if hidden_count > 0 {
            println!("\n{hidden_count} variable(s) hidden (name looked sensitive)");
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

    #[test]
    fn test_env_plugin_metadata() {
        let p = EnvPlugin::new();
        assert_eq!(p.name(), "env");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    fn test_env_plugin_default() {
        let p = EnvPlugin::default();
        assert_eq!(p.name(), "env");
    }

    #[test]
    fn test_env_plugin_handler_name() {
        let handlers = EnvPlugin::new().handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "env_show");
    }

    #[test]
    fn test_env_handler_executes() {
        let handlers = EnvPlugin::new().handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_env_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(EnvPlugin::new());
    }

    #[test]
    fn test_is_sensitive_name_matches_expected_patterns() {
        assert!(is_sensitive_name("API_SECRET"));
        assert!(is_sensitive_name("AWS_ACCESS_KEY_ID"));
        assert!(is_sensitive_name("DATABASE_PASSWORD"));
        assert!(is_sensitive_name("GITHUB_TOKEN"));
        assert!(is_sensitive_name("MY_APP_CREDENTIAL"));
        assert!(is_sensitive_name("BASIC_AUTH_HEADER"));
        assert!(is_sensitive_name("SSH_PRIVATE_KEY_PATH"));
        // Case-insensitivity
        assert!(is_sensitive_name("api_secret"));
        assert!(is_sensitive_name("Api_Secret"));
    }

    #[test]
    fn test_is_sensitive_name_leaves_ordinary_vars_alone() {
        assert!(!is_sensitive_name("PATH"));
        assert!(!is_sensitive_name("HOME"));
        assert!(!is_sensitive_name("LANG"));
        assert!(!is_sensitive_name("EDITOR"));
        assert!(!is_sensitive_name("RUST_LOG"));
    }
}
