//! Integration test — Async token fetch handler (DD-022 mockup)
//!
//! Exercises the full chain `CliBuilder -> build() -> CliApp -> run_cli()`
//! with an [`AsyncCommandHandler`] that performs a genuinely non-blocking
//! wait (via `futures_timer::Delay`, *not* `std::thread::sleep`) before
//! returning a token — modeling `chrom-rs`'s confirmed need for async
//! network I/O (DD-022).
//!
//! Unit tests in `src/executor/traits.rs` already cover `AsyncCommandHandler`
//! in isolation (calling `.execute()` directly). This file instead verifies
//! dispatch through the *real* path a downstream application uses:
//! `CliInterface::run()` -> `get_handler_async()` ->
//! `futures::executor::block_on()`.
//!
//! The delay here is kept short (tests must stay fast). See
//! `examples/async_token_demo.rs` for a runnable demo using the literal
//! 10-second delay this scenario was originally modeled on.

use async_trait::async_trait;
use dynamic_cli::config::schema::{CommandDefinition, Metadata};
use dynamic_cli::executor::AsyncCommandHandler;
use dynamic_cli::prelude::*;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// Test fixtures
// ============================================================================

/// Execution context that records the token(s) received.
///
/// Shared via `Arc<Mutex<_>>` so the test can inspect side effects after
/// `run_cli()` returns (handlers only receive `&mut dyn ExecutionContext`,
/// not an owned, inspectable value).
#[derive(Default)]
struct TokenContext {
    tokens: Arc<Mutex<Vec<String>>>,
}

impl ExecutionContext for TokenContext {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Simulates a remote token fetch: waits `delay` non-blockingly, then
/// writes a token into the shared record.
///
/// Uses `futures_timer::Delay` rather than `std::thread::sleep` —
/// deliberately. A thread sleep inside an `async fn` body blocks the
/// executor thread exactly like a sync call would, which defeats the
/// entire point of DD-022. `futures_timer` yields control back to the
/// executor while waiting, the same way a real async network call would.
struct TokenFetchHandler {
    delay: Duration,
    tokens: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl AsyncCommandHandler for TokenFetchHandler {
    async fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> dynamic_cli::Result<()> {
        futures_timer::Delay::new(self.delay).await;
        self.tokens.lock().unwrap().push("tok_9f3a2b1c".to_string());
        Ok(())
    }
}

/// A minimal config declaring a single `fetch-token` command bound to the
/// async handler, plus an alias to exercise alias resolution.
fn test_config() -> CommandsConfig {
    CommandsConfig {
        metadata: Metadata {
            version: "1.0.0".to_string(),
            prompt: "testapp".to_string(),
            prompt_suffix: " > ".to_string(),
        },
        commands: vec![CommandDefinition {
            name: "fetch-token".to_string(),
            aliases: vec!["ft".to_string()],
            description: "Fetch an auth token".to_string(),
            required: true,
            arguments: vec![],
            options: vec![],
            implementation: "token_handler".to_string(),
        }],
        global_options: vec![],
    }
}

// ============================================================================
// Tests
// ============================================================================

/// End-to-end proof that an `AsyncCommandHandler` reaches execution through
/// the real dispatch path, and that `block_on` genuinely awaits it — the
/// token is only observable *after* `run_cli()` returns, and elapsed time
/// confirms the delay was actually awaited rather than skipped.
#[test]
fn async_token_handler_executes_via_full_chain() {
    let tokens = Arc::new(Mutex::new(Vec::new()));
    let delay = Duration::from_millis(50); // short: this runs on every `cargo test`

    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(TokenContext::default()))
        .register_async_handler(
            "token_handler",
            Box::new(TokenFetchHandler {
                delay,
                tokens: tokens.clone(),
            }),
        )
        .build()
        .expect("build should succeed with an async-only handler");

    let started = Instant::now();
    let result = app.run_cli(vec!["fetch-token".to_string()]);
    let elapsed = started.elapsed();

    assert!(result.is_ok(), "token_handler should execute without error");
    assert!(
        elapsed >= delay,
        "dispatch must actually await the async delay, not return early \
         (elapsed: {elapsed:?}, expected at least {delay:?})"
    );
    assert_eq!(
        tokens.lock().unwrap().as_slice(),
        &["tok_9f3a2b1c".to_string()],
        "the token written inside the async handler should be observable \
         after run_cli() returns — proves block_on drove the future to \
         completion before returning control to the caller"
    );
}

/// The alias declared in the YAML config resolves to the same async
/// handler — `resolve_name` / `get_handler_async` cooperate correctly for
/// async-registered commands, exactly as they do for sync ones.
#[test]
fn async_token_handler_alias_resolves() {
    let tokens = Arc::new(Mutex::new(Vec::new()));

    let app = CliBuilder::new()
        .config(test_config())
        .context(Box::new(TokenContext::default()))
        .register_async_handler(
            "token_handler",
            Box::new(TokenFetchHandler {
                delay: Duration::from_millis(10),
                tokens: tokens.clone(),
            }),
        )
        .build()
        .expect("build should succeed");

    let result = app.run_cli(vec!["ft".to_string()]);
    assert!(result.is_ok(), "alias 'ft' should resolve to token_handler");
    assert_eq!(tokens.lock().unwrap().len(), 1);
}

/// A required command satisfied only by an async handler must build
/// successfully — mirrors the unit-level
/// `test_builder_build_required_command_satisfied_by_async_handler` in
/// `builder.rs`, but through the fully public API surface (no access to
/// private builder fields).
#[test]
fn required_command_satisfied_by_async_handler_only() {
    let app = CliBuilder::new()
        .config(test_config()) // "fetch-token" is `required: true`
        .context(Box::new(TokenContext::default()))
        .register_async_handler(
            "token_handler",
            Box::new(TokenFetchHandler {
                delay: Duration::from_millis(10),
                tokens: Arc::new(Mutex::new(Vec::new())),
            }),
        )
        .build();

    assert!(
        app.is_ok(),
        "a required command must build successfully when only an async \
         handler is registered for it"
    );
}
