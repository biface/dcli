//! Runnable demo of an [`AsyncCommandHandler`] performing a genuinely
//! non-blocking 10-second wait before returning a token — the literal
//! scenario this mockup was built around, modeling `chrom-rs`'s confirmed
//! need for async network I/O / streaming (DD-022).
//!
//! The automated test suite uses a much shorter delay for speed — see
//! `tests/integration/async_token_test.rs`. Run this binary instead to
//! actually watch the 10 seconds elapse:
//!
//! ```text
//! cargo run --example async_token_demo
//! ```

use async_trait::async_trait;
use dynamic_cli::config::schema::{CommandDefinition, Metadata};
use dynamic_cli::executor::AsyncCommandHandler;
use dynamic_cli::prelude::*;
use std::any::Any;
use std::time::{Duration, Instant};

#[derive(Default)]
struct DemoContext;

impl ExecutionContext for DemoContext {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Simulates a remote token fetch: a real 10-second wait, non-blocking
/// (`futures_timer::Delay`, not `std::thread::sleep` — see
/// `tests/integration/async_token_test.rs` for why that distinction
/// matters: a thread sleep here would block the executor exactly like a
/// sync call, defeating the entire point of DD-022).
struct TokenFetchHandler;

#[async_trait]
impl AsyncCommandHandler for TokenFetchHandler {
    async fn execute(
        &self,
        _ctx: &mut dyn ExecutionContext,
        _args: &ParsedArgs,
    ) -> dynamic_cli::Result<()> {
        println!("Fetching token... (this genuinely takes 10 seconds)");
        let started = Instant::now();
        futures_timer::Delay::new(Duration::from_secs(10)).await;
        println!(
            "Token received after {:.1}s: tok_9f3a2b1c",
            started.elapsed().as_secs_f32()
        );
        Ok(())
    }
}

fn config() -> CommandsConfig {
    CommandsConfig {
        metadata: Metadata {
            version: "1.0.0".to_string(),
            prompt: "demo".to_string(),
            prompt_suffix: " > ".to_string(),
        },
        commands: vec![CommandDefinition {
            name: "fetch-token".to_string(),
            aliases: vec![],
            description: "Fetch an auth token (simulated 10s network call)".to_string(),
            required: true,
            arguments: vec![],
            options: vec![],
            implementation: "token_handler".to_string(),
        }],
        global_options: vec![],
    }
}

fn main() -> dynamic_cli::Result<()> {
    let app = CliBuilder::new()
        .config(config())
        .context(Box::new(DemoContext))
        .register_async_handler("token_handler", Box::new(TokenFetchHandler))
        .build()?;

    app.run_cli(vec!["fetch-token".to_string()])
}
