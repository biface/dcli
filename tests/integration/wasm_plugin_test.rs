//! Integration test — WasmPlugin (Option C, DD-021)
//!
//! Exercises the full chain `CliBuilder -> register_wasm_plugin() -> build()
//! -> CliApp -> run_cli()` using only the crate's public API, with real
//! `.wasm` modules written to disk via `tempfile::NamedTempFile` — exactly
//! as a downstream application loading a third-party plugin would.
//!
//! Unit tests in `src/plugin/wasm.rs` already cover `WasmPlugin` and
//! `WasmHandler` in isolation, using `WasmPlugin::from_bytes()` directly.
//! This file instead verifies the public entry point
//! (`CliBuilder::register_wasm_plugin`, which loads from a file path) and
//! that both mandatory and optional parts of the ABI contract
//! (`WASM_PLUGIN_INTERFACE.md`) propagate correctly all the way up to
//! `run_cli()`'s `Result`.
//!
//! Covered ABI surface:
//! - Mandatory: `memory`, `dcli_alloc`, `dcli_dealloc`, the mapped business
//!   function.
//! - Optional: `dcli_last_error_message`.

#![cfg(feature = "wasm-plugins")]

use dynamic_cli::config::schema::{CommandDefinition, Metadata};
use dynamic_cli::error::{DynamicCliError, WasmError};
use dynamic_cli::prelude::*;
use std::any::Any;
use std::io::Write;
use tempfile::NamedTempFile;

// ============================================================================
// Test fixtures
// ============================================================================

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

/// A user-defined handler, registered alongside a WASM plugin, to verify
/// `register_wasm_plugin` and `register_handler` coexist in `build()`.
struct NativeHandler;

impl CommandHandler for NativeHandler {
    fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        _args: &ParsedArgs,
    ) -> dynamic_cli::Result<()> {
        Ok(())
    }
}

/// Mandatory-only contract: `memory`, `dcli_alloc`, `dcli_dealloc`, and a
/// business function `ok_handler` that always succeeds.
///
/// Allocation/deallocation use a fixed bump pointer at offset 1024 — valid
/// for ABI-contract testing, not a realistic allocator.
const WAT_SUCCESS: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "dcli_alloc") (param i32) (result i32)
            i32.const 1024)
        (func (export "dcli_dealloc") (param i32 i32))
        (func (export "ok_handler") (param i32 i32) (result i32)
            i32.const 0)
    )
"#;

/// Mandatory-only contract, business function always fails with code 1,
/// no optional `dcli_last_error_message` export.
const WAT_ERROR_NO_MESSAGE: &str = r#"
    (module
        (memory (export "memory") 1)
        (func (export "dcli_alloc") (param i32) (result i32)
            i32.const 1024)
        (func (export "dcli_dealloc") (param i32 i32))
        (func (export "err_handler") (param i32 i32) (result i32)
            i32.const 1)
    )
"#;

/// Full contract: mandatory exports plus the optional
/// `dcli_last_error_message`, pointing at a fixed string in a data segment.
const WAT_ERROR_WITH_MESSAGE: &str = r#"
    (module
        (memory (export "memory") 1)
        (data (i32.const 2048) "disk write failed")
        (func (export "dcli_alloc") (param i32) (result i32)
            i32.const 1024)
        (func (export "dcli_dealloc") (param i32 i32))
        (func (export "err_handler") (param i32 i32) (result i32)
            i32.const 1)
        (func (export "dcli_last_error_message") (result i32 i32)
            i32.const 2048
            i32.const 17)
    )
"#;

/// Writes a WAT module to a temporary `.wasm` file and returns the handle.
///
/// `NamedTempFile` is used rather than `TempDir` + `File::create`, per the
/// project's established testing convention (eliminates path race
/// conditions under parallel test execution). The file extension does not
/// matter to `wasmtime` — it accepts WAT text directly — but `.wasm` is used
/// for realism, since real plugin files will have that extension.
fn write_wat_to_temp_file(wat: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("failed to create temp file for WAT fixture");
    file.write_all(wat.as_bytes())
        .expect("failed to write WAT fixture to temp file");
    file.flush().expect("failed to flush WAT fixture");
    file
}

/// Builds a minimal config declaring a single command `run` bound to the
/// given WASM implementation name.
///
/// Used by the success and error-path tests, which only need the WASM
/// plugin's command to be present — declaring an extra required command
/// here would force every test to also register a handler for it.
fn test_config(wasm_implementation: &str) -> CommandsConfig {
    CommandsConfig {
        metadata: Metadata {
            version: "1.0.0".to_string(),
            prompt: "testapp".to_string(),
            prompt_suffix: " > ".to_string(),
        },
        commands: vec![CommandDefinition {
            name: "run".to_string(),
            aliases: vec![],
            description: "Invoke the WASM plugin".to_string(),
            required: true,
            arguments: vec![],
            options: vec![],
            implementation: wasm_implementation.to_string(),
        }],
        global_options: vec![],
    }
}

/// Builds a config declaring both the WASM-backed `run` command and a
/// `native` command bound to `NativeHandler`.
///
/// Used only by the coexistence test, which is the sole scenario requiring
/// both a WASM plugin and a directly-registered handler in the same
/// `CliApp`.
fn test_config_with_native(wasm_implementation: &str) -> CommandsConfig {
    CommandsConfig {
        metadata: Metadata {
            version: "1.0.0".to_string(),
            prompt: "testapp".to_string(),
            prompt_suffix: " > ".to_string(),
        },
        commands: vec![
            CommandDefinition {
                name: "run".to_string(),
                aliases: vec![],
                description: "Invoke the WASM plugin".to_string(),
                required: true,
                arguments: vec![],
                options: vec![],
                implementation: wasm_implementation.to_string(),
            },
            CommandDefinition {
                name: "native".to_string(),
                aliases: vec![],
                description: "A directly-registered native command".to_string(),
                required: true,
                arguments: vec![],
                options: vec![],
                implementation: "native_handler".to_string(),
            },
        ],
        global_options: vec![],
    }
}

// ============================================================================
// Tests — mandatory contract, success path
// ============================================================================

/// A WASM module implementing only the mandatory ABI exports executes
/// successfully through the full chain: `register_wasm_plugin` -> `build`
/// -> `run_cli`.
#[test]
fn wasm_plugin_success_executes_via_full_chain() {
    let wasm_file = write_wat_to_temp_file(WAT_SUCCESS);

    let app = CliBuilder::new()
        .config(test_config("ok_handler"))
        .context(Box::new(TestContext))
        .register_wasm_plugin(wasm_file.path(), &[("ok_handler", "ok_handler")])
        .expect("loading a module with all mandatory exports should succeed")
        .build()
        .expect("build should succeed with a valid WASM plugin registered");

    let result = app.run_cli(vec!["run".to_string()]);
    assert!(
        result.is_ok(),
        "ok_handler should execute successfully end-to-end"
    );
}

// ============================================================================
// Tests — mandatory contract, error path (no optional export)
// ============================================================================

/// A WASM module that returns a non-zero error code, without exporting the
/// optional `dcli_last_error_message`, propagates a `GuestError` with
/// `message: None` all the way up to `run_cli()`'s `Result`.
#[test]
fn wasm_plugin_guest_error_without_message_propagates_via_full_chain() {
    let wasm_file = write_wat_to_temp_file(WAT_ERROR_NO_MESSAGE);

    let app = CliBuilder::new()
        .config(test_config("err_handler"))
        .context(Box::new(TestContext))
        .register_wasm_plugin(wasm_file.path(), &[("err_handler", "err_handler")])
        .expect("loading a module with all mandatory exports should succeed")
        .build()
        .expect("build should succeed — mandatory contract is satisfied");

    let result = app.run_cli(vec!["run".to_string()]);
    assert!(result.is_err(), "err_handler should propagate as an error");

    match result.unwrap_err() {
        DynamicCliError::Wasm(WasmError::GuestError { code, message }) => {
            assert_eq!(code, 1);
            assert!(
                message.is_none(),
                "no dcli_last_error_message export — message must be None, not invented"
            );
        }
        other => panic!("expected DynamicCliError::Wasm(GuestError), got: {other:?}"),
    }
}

// ============================================================================
// Tests — full contract, error path (optional export present)
// ============================================================================

/// A WASM module that returns a non-zero error code AND exports the
/// optional `dcli_last_error_message` propagates the detailed message all
/// the way up to `run_cli()`'s `Result` — not just at the `WasmHandler`
/// unit level.
#[test]
fn wasm_plugin_guest_error_with_message_propagates_via_full_chain() {
    let wasm_file = write_wat_to_temp_file(WAT_ERROR_WITH_MESSAGE);

    let app = CliBuilder::new()
        .config(test_config("err_handler"))
        .context(Box::new(TestContext))
        .register_wasm_plugin(wasm_file.path(), &[("err_handler", "err_handler")])
        .expect("loading a module with mandatory + optional exports should succeed")
        .build()
        .expect("build should succeed");

    let result = app.run_cli(vec!["run".to_string()]);
    assert!(result.is_err(), "err_handler should propagate as an error");

    match result.unwrap_err() {
        DynamicCliError::Wasm(WasmError::GuestError { code, message }) => {
            assert_eq!(code, 1);
            assert_eq!(
                message,
                Some("disk write failed".to_string()),
                "dcli_last_error_message export should be read and surfaced end-to-end"
            );
        }
        other => panic!("expected DynamicCliError::Wasm(GuestError), got: {other:?}"),
    }
}

// ============================================================================
// Tests — coexistence with a native handler
// ============================================================================

/// `register_wasm_plugin` and `register_handler` coexist in the same
/// `CliApp`, each dispatching to its own implementation correctly.
#[test]
fn wasm_plugin_coexists_with_native_handler() {
    let wasm_file = write_wat_to_temp_file(WAT_SUCCESS);

    let app = CliBuilder::new()
        .config(test_config_with_native("ok_handler"))
        .context(Box::new(TestContext))
        .register_wasm_plugin(wasm_file.path(), &[("ok_handler", "ok_handler")])
        .expect("loading a module with all mandatory exports should succeed")
        .register_sync_handler("native_handler", Box::new(NativeHandler))
        .build()
        .expect("build should succeed with both a WASM plugin and a native handler");

    assert!(
        app.run_cli(vec!["run".to_string()]).is_ok(),
        "WASM-backed 'run' command should execute"
    );
}
