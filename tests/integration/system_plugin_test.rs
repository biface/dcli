//! Integration test — SystemPlugin (Option A, DD-021)
//!
//! Exercises the full chain `CliBuilder -> build() -> CliApp -> run_cli()`
//! with [`SystemPlugin`] registered alongside a directly-registered handler,
//! using only the crate's public API — exactly as a downstream application
//! would.
//!
//! Unit tests in `src/plugin/system.rs` already cover each `SystemPlugin`
//! handler in isolation. This file instead verifies that the plugin
//! integrates correctly through the builder: conflict-free registration,
//! correct dispatch via the YAML-declared `implementation` name, and
//! coexistence with a user-registered handler in the same application.

use dynamic_cli::config::schema::{CommandDefinition, Metadata};
use dynamic_cli::plugin::SystemPlugin;
use dynamic_cli::prelude::*;
use std::any::Any;
use std::sync::{Arc, Mutex};

// ============================================================================
// Test fixtures
// ============================================================================

/// Execution context that records invocations of the user-defined handler.
///
/// Shared via `Arc<Mutex<_>>` so the test can inspect side effects after
/// `run_cli()` returns (handlers only receive `&mut dyn ExecutionContext`,
/// not an owned, inspectable value).
#[derive(Default)]
struct RecordingContext {
    greeted: Arc<Mutex<Vec<String>>>,
}

impl ExecutionContext for RecordingContext {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A user-defined handler, registered alongside `SystemPlugin`, to verify
/// the two registration paths (`register_handler` and `register_plugin`)
/// coexist without interference.
struct GreetHandler {
    greeted: Arc<Mutex<Vec<String>>>,
}

impl CommandHandler for GreetHandler {
    fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        args: &ParsedArgs,
    ) -> dynamic_cli::Result<()> {
        let name = args.get_scalar("name").unwrap_or("World").to_string();
        self.greeted.lock().unwrap().push(name);
        Ok(())
    }
}

/// Builds a minimal config declaring the three `SystemPlugin` commands plus
/// one user command (`greet`) bound to `GreetHandler`.
fn test_config() -> CommandsConfig {
    CommandsConfig {
        metadata: Metadata {
            version: "1.2.3".to_string(),
            prompt: "testapp".to_string(),
            prompt_suffix: " > ".to_string(),
        },
        commands: vec![
            CommandDefinition {
                name: "version".to_string(),
                aliases: vec![],
                description: "Show version".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "system_version".to_string(),
            },
            CommandDefinition {
                name: "help".to_string(),
                aliases: vec!["h".to_string()],
                description: "Show help".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "system_help".to_string(),
            },
            CommandDefinition {
                name: "exit".to_string(),
                aliases: vec!["quit".to_string()],
                description: "Exit the application".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "system_exit".to_string(),
            },
            CommandDefinition {
                name: "greet".to_string(),
                aliases: vec![],
                description: "Greet someone".to_string(),
                required: true,
                arguments: vec![],
                options: vec![],
                implementation: "greet_handler".to_string(),
            },
        ],
        global_options: vec![],
    }
}

// ============================================================================
// Tests
// ============================================================================

/// `SystemPlugin` handlers are reachable through the full builder chain,
/// dispatched by their YAML-declared `implementation` name.
#[test]
fn system_plugin_version_command_executes_via_full_chain() {
    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(RecordingContext::default()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_sync_handler(
            "greet_handler",
            Box::new(GreetHandler {
                greeted: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .build()
        .expect("build should succeed with SystemPlugin registered");

    let result = app.run_cli(vec!["version".to_string()]);
    assert!(
        result.is_ok(),
        "system_version handler should execute without error"
    );
}

/// `system_help` executes without error when invoked through the registry
/// (as opposed to the `--help` interception path tested elsewhere).
#[test]
fn system_plugin_help_command_executes_via_full_chain() {
    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(RecordingContext::default()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_sync_handler(
            "greet_handler",
            Box::new(GreetHandler {
                greeted: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .build()
        .expect("build should succeed");

    let result = app.run_cli(vec!["help".to_string()]);
    assert!(
        result.is_ok(),
        "system_help handler should execute without error"
    );
}

/// `SystemPlugin`'s handlers and a directly-registered user handler coexist
/// in the same registry without interfering with each other.
#[test]
fn system_plugin_coexists_with_user_registered_handler() {
    let greeted = Arc::new(Mutex::new(Vec::new()));

    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(RecordingContext::default()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_sync_handler(
            "greet_handler",
            Box::new(GreetHandler {
                greeted: greeted.clone(),
            }),
        )
        .build()
        .expect("build should succeed");

    let result = app.run_cli(vec!["greet".to_string()]);
    assert!(
        result.is_ok(),
        "user-registered greet_handler should execute"
    );
    assert_eq!(
        greeted.lock().unwrap().as_slice(),
        &["World".to_string()],
        "user handler should have recorded its invocation"
    );
}

/// `SystemPlugin` aliases declared in the YAML config (`h` for `help`,
/// `quit` for `exit`) resolve correctly through the registry built by
/// `CliBuilder`.
#[test]
fn system_plugin_alias_resolves_to_same_handler() {
    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(RecordingContext::default()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_sync_handler(
            "greet_handler",
            Box::new(GreetHandler {
                greeted: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .build()
        .expect("build should succeed");

    let result = app.run_cli(vec!["h".to_string()]);
    assert!(result.is_ok(), "alias 'h' should resolve to system_help");
}

/// Registering `SystemPlugin` twice (or alongside a direct handler with the
/// same `implementation` name) produces a build-time error rather than
/// silently overwriting a handler — the conflict-detection path exercised
/// at unit level in `builder.rs` also holds through the public API.
#[test]
fn duplicate_plugin_registration_fails_at_build() {
    let result = CliBuilder::new()
        .config(test_config())
        .context(Box::new(RecordingContext::default()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_plugin(Box::new(SystemPlugin::new()))
        .register_sync_handler(
            "greet_handler",
            Box::new(GreetHandler {
                greeted: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .build();

    assert!(
        result.is_err(),
        "registering the same plugin twice should fail at build() with a conflict error"
    );
}
