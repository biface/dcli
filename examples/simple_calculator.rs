//! Simple Calculator Example
//!
//! Demonstrates basic usage of dynamic-cli with a calculator application.
//! Supports basic arithmetic operations with REPL mode.
//!
//! # Usage
//!
//! ```bash
//! # REPL mode (interactive)
//! cargo run --example simple_calculator
//!
//! # CLI mode (one-shot commands)
//! cargo run --example simple_calculator -- add 5 3
//! cargo run --example simple_calculator -- multiply 4 7
//! ```

use dynamic_cli::prelude::*;
use std::collections::HashMap;

// ============================================================================
// EXECUTION CONTEXT
// ============================================================================

/// Calculator context stores computation history
#[derive(Default)]
struct CalculatorContext {
    history: Vec<String>,
    last_result: Option<f64>,
}

impl ExecutionContext for CalculatorContext {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

/// Handler for addition command
struct AddCommand;

impl CommandHandler for AddCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        // Parse arguments
        let a = dynamic_cli::parse_float(args.get("a").unwrap(), "a")?;
        let b = dynamic_cli::parse_float(args.get("b").unwrap(), "b")?;

        // Calculate
        let result = a + b;
        ctx.last_result = Some(result);
        ctx.history.push(format!("{} + {} = {}", a, b, result));

        // Display
        println!("Result: {}", result);

        Ok(())
    }
}

/// Handler for subtraction command
struct SubtractCommand;

impl CommandHandler for SubtractCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        let a = dynamic_cli::parse_float(args.get("a").unwrap(), "a")?;
        let b = dynamic_cli::parse_float(args.get("b").unwrap(), "b")?;

        let result = a - b;
        ctx.last_result = Some(result);
        ctx.history.push(format!("{} - {} = {}", a, b, result));

        println!("Result: {}", result);

        Ok(())
    }
}

/// Handler for multiplication command
struct MultiplyCommand;

impl CommandHandler for MultiplyCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        let a = dynamic_cli::parse_float(args.get("a").unwrap(), "a")?;
        let b = dynamic_cli::parse_float(args.get("b").unwrap(), "b")?;

        let result = a * b;
        ctx.last_result = Some(result);
        ctx.history.push(format!("{} × {} = {}", a, b, result));

        println!("Result: {}", result);

        Ok(())
    }
}

/// Handler for division command
struct DivideCommand;

impl CommandHandler for DivideCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        let a = dynamic_cli::parse_float(args.get("a").unwrap(), "a")?;
        let b = dynamic_cli::parse_float(args.get("b").unwrap(), "b")?;

        // Check for division by zero
        if b == 0.0 {
            return Err(DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::CommandFailed(
                    anyhow::anyhow!("Division by zero")
                )
            ));
        }

        let result = a / b;
        ctx.last_result = Some(result);
        ctx.history.push(format!("{} ÷ {} = {}", a, b, result));

        println!("Result: {}", result);

        Ok(())
    }
}

/// Handler for history command
struct HistoryCommand;

impl CommandHandler for HistoryCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_ref::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        if ctx.history.is_empty() {
            println!("No history yet.");
        } else {
            println!("Calculation History:");
            println!("{}", dynamic_cli::utils::format_numbered_list(&ctx.history));
        }

        Ok(())
    }
}

/// Handler for clear command
struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        ctx.history.clear();
        ctx.last_result = None;
        println!("History cleared.");

        Ok(())
    }
}

/// Handler for last command
struct LastCommand;

impl CommandHandler for LastCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_ref::<CalculatorContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "CalculatorContext".to_string(),
                })
            })?;

        match ctx.last_result {
            Some(result) => println!("Last result: {}", result),
            None => println!("No previous calculation."),
        }

        Ok(())
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> Result<()> {
    // Create configuration file path
    let config_path = "examples/configs/calculator.yaml";

    // Build and run application
    CliBuilder::new()
        .config_file(config_path)
        .context(Box::new(CalculatorContext::default()))
        .register_handler("add_handler", Box::new(AddCommand))
        .register_handler("subtract_handler", Box::new(SubtractCommand))
        .register_handler("multiply_handler", Box::new(MultiplyCommand))
        .register_handler("divide_handler", Box::new(DivideCommand))
        .register_handler("history_handler", Box::new(HistoryCommand))
        .register_handler("clear_handler", Box::new(ClearCommand))
        .register_handler("last_handler", Box::new(LastCommand))
        .build()?
        .run()
}
