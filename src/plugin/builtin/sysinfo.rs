//! Standalone `SysInfoPlugin` (feature-gated).
//!
//! Prints basic runtime/OS information — operating system, architecture,
//! and available parallelism — using only `std`. No new dependency: this
//! is deliberately a minimal baseline (#45 / DD-025). A richer variant
//! backed by the `sysinfo` crate is out of scope here; if pursued later,
//! it stays under this same `sysinfo-plugin` feature flag rather than a
//! second one, per the uniform feature-flag policy decided in the DD-025
//! triage.
//!
//! Only available with the `sysinfo-plugin` feature.

use crate::context::ExecutionContext;
use crate::executor::CommandHandler;
use crate::parser::ParsedArgs;
use crate::plugin::Plugin;
use crate::Result;

/// Standalone plugin providing a single `sysinfo` command.
///
/// Reports the operating system, CPU architecture, and available
/// parallelism (`std::thread::available_parallelism()`) — everything
/// `std` can report without a third-party crate.
///
/// # YAML config
///
/// ```yaml
/// commands:
///   - name: sysinfo
///     implementation: sysinfo_show
///     description: "Show runtime/OS information"
///     required: false
///     arguments: []
///     options: []
/// ```
///
/// # Example
///
/// ```
/// use dynamic_cli::plugin::{Plugin, SysInfoPlugin};
///
/// let plugin = SysInfoPlugin::new();
/// assert_eq!(plugin.name(), "sysinfo");
///
/// let handlers = plugin.handlers();
/// assert_eq!(handlers.len(), 1);
/// assert_eq!(handlers[0].0, "sysinfo_show");
/// ```
pub struct SysInfoPlugin;

impl SysInfoPlugin {
    /// Create a new `SysInfoPlugin`.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::plugin::{Plugin, SysInfoPlugin};
    ///
    /// let plugin = SysInfoPlugin::new();
    /// assert_eq!(plugin.name(), "sysinfo");
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl Default for SysInfoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SysInfoPlugin {
    fn name(&self) -> &str {
        "sysinfo"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn description(&self) -> &str {
        "Runtime/OS introspection (std-only baseline, feature-gated, #45)"
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        vec![("sysinfo_show".to_string(), Box::new(SysInfoShowHandler))]
    }
}

/// Handler for `sysinfo_show` — prints OS, architecture, and available
/// parallelism.
struct SysInfoShowHandler;

impl CommandHandler for SysInfoShowHandler {
    fn execute(&self, _ctx: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        println!("OS:                  {}", std::env::consts::OS);
        println!("Architecture:        {}", std::env::consts::ARCH);
        println!("Available parallelism: {}", parallelism);

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
    fn test_sysinfo_plugin_metadata() {
        let p = SysInfoPlugin::new();
        assert_eq!(p.name(), "sysinfo");
        assert!(!p.version().is_empty());
        assert!(!p.description().is_empty());
    }

    #[test]
    // SysInfoPlugin::default() is what's under test here (the Default impl
    // itself, kept for clippy::new_without_default compliance); rewriting
    // it to the bare unit-struct literal per clippy's own suggestion would
    // stop exercising that impl and defeat the test's purpose.
    #[allow(clippy::default_constructed_unit_structs)]
    fn test_sysinfo_plugin_default() {
        let p = SysInfoPlugin::default();
        assert_eq!(p.name(), "sysinfo");
    }

    #[test]
    fn test_sysinfo_plugin_handler_name() {
        let handlers = SysInfoPlugin::new().handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "sysinfo_show");
    }

    #[test]
    fn test_sysinfo_handler_executes() {
        let handlers = SysInfoPlugin::new().handlers();
        let (_, handler) = &handlers[0];
        let mut ctx = TestContext;
        assert!(handler
            .execute(&mut ctx, &ParsedArgs::from_scalars(Default::default()))
            .is_ok());
    }

    #[test]
    fn test_sysinfo_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>(_: T) {}
        assert_send_sync(SysInfoPlugin::new());
    }
}
