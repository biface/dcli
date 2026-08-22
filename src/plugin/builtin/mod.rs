//! Standalone, single-command plugins.
//!
//! `SystemPlugin` (in [`crate::builtin::system`]) bundles `help`, `version`,
//! and `exit` into a single builtin. This module offers the same three
//! commands as **independent** plugins — [`HelpPlugin`], [`VersionPlugin`],
//! [`ExitPlugin`] — for applications that want to compose only the ones
//! they need (e.g. `version` alone, without `help`/`exit`).
//!
//! # Relationship to `SystemPlugin`
//!
//! Each builtin here reuses the exact same handler logic as its
//! `SystemPlugin` counterpart (`SystemHelpHandler`, `SystemVersionHandler`,
//! `SystemExitHandler` in [`crate::builtin::system`]) — this is a pure
//! internal refactor (#44 / DD-025). `SystemPlugin`'s public API, its
//! `handlers()` output, and its existing test suite are unaffected.
//!
//! | Plugin          | Implementation name | Behaviour                                   |
//! |------------------|---------------------|----------------------------------------------|
//! | [`HelpPlugin`]    | `system_help`       | Same as `SystemPlugin`'s `system_help`        |
//! | [`VersionPlugin`] | `system_version`    | Same as `SystemPlugin`'s `system_version`     |
//! | [`ExitPlugin`]    | `system_exit`       | Same as `SystemPlugin`'s `system_exit`        |
//!
//! Implementation names are unchanged, so existing YAML configs that
//! reference `system_help` / `system_version` / `system_exit` keep working
//! whether the commands are wired through `SystemPlugin` or through these
//! granular plugins.
//!
//! # Example
//!
//! ```
//! use dynamic_cli::plugin::{Plugin, VersionPlugin};
//!
//! // Register only `version`, without `help` or `exit`.
//! let builtin = VersionPlugin::new();
//! assert_eq!(builtin.handlers().len(), 1);
//! ```

mod exit;
mod help;
mod version;

pub use exit::ExitPlugin;
pub use help::HelpPlugin;
pub use version::VersionPlugin;
