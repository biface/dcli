//! # Simple RPN calculator example
//!
//! A reverse polish notation calculator demonstrating dynamic-cli capabilities
//!
//! ## Features
//!
//! - Interactive REPL mode
//! - Stack-based simple arithmetic operation
//! - Last x register (à la mode HP 41CX)
//! - State management via ExecutionContext
//! - Configuration using YAML configuration file
//!
//! ## Usage
//! ```bash
//! # REPL mode (interactive)
//! cargo run --example rpn_calculator
//!
//! # CLI mode (simple command)
//! cargo run --exemple rpn_calculator -- push 5
//! cargo run --exemple rpn_calculator -- push 4
//! cargo run --exemple rpn_calculator -- add
//!```
//!
//! ## RPN Basics
//!
//! Reverse Polish Notation is a postfix notation in which operators follows their operands
//!
//! ```text
//! Infix : 3 + 4
//! RPN   : 3 4 +
//!
//! Infix : (3 + 4) * 5
//! RPN   : 3 4 + 5 *
//!
//! With an advanced stack
//! RPN   : 5 3 4 + *
//! ```

use dynamic_cli::prelude::*;
use std::any::Any;
use std::collections::HashMap;

/// ================================================================================================
/// Execution context
/// ================================================================================================

/// Execution context for the simple rpn calculator
///
/// SimpleRpnContext maintains the calculation stack across commands in REPL mode.
#[derive(Default)]
struct SimpleRpnContext {
    /// Calculation stack : top of the stack is the last element pushed
    stack: Vec<f64>,
    /// Last x is the register containing the last element pushed
    last_x: f64,
}

impl SimpleRpnContext {
    /// Push a value onto the stack
    fn push(&mut self, value: f64) {
        self.stack.push(value);
        println!("  → Pushed {:?}", self.stack.last());
    }

    /// Push a value onto the stack and lastx register
    fn push_x(&mut self, value: f64) {
        self.last_x = value.clone();
        self.push(value);
    }

    /// Pop a value from the stack
    fn pop(&mut self) -> Result<f64> {
        self.stack.pop().ok_or_else(|| {
            DynamicCliError::Execution(dynamic_cli::error::ExecutionError::CommandFailed(
                anyhow::anyhow!("Stack is empty"),
            ))
        })
    }

    /// Show the last x register
    fn last_x(&self) -> f64 {
        self.last_x.clone()
    }

    /// Swap registers
    fn swap(&mut self) {
        if self.stack.len() < 2 {
            println!("  → Not enough elements on stack to swap");
            return;
        }

        let x = self.stack.pop().unwrap();
        let y = self.stack.pop().unwrap();

        self.stack.push(x);
        self.stack.push(y);

        println!("  → Swapped the top two elements");
    }

    /// Peek at the top of the stack without removing it
    fn peek(&self) -> Option<f64> {
        self.stack.last().cloned()
    }

    /// Clear the stack
    fn clear(&mut self) {
        self.stack.clear();
        println!("  → Stack cleared")
    }

    /// Display the stack content
    fn display(&self) {
        if self.stack.is_empty() {
            println!("    Stack is empty");
        } else {
            println!("  Stack: {:?}", self.stack);
            println!("  Last X: {}", self.last_x);
        }
    }

    fn binary_op<F>(&mut self, operation: F, operator_name: &str) -> Result<()>
    where F: FnOnce(f64, f64) -> f64 {
        let x = self.pop()?;
        let y = self.pop()?;
        let result = operation(x, y);
        self.push(result);
        println!("  → {} {} {} = {}", x, y, operator_name, result);
        Ok(())
    }

    fn single_op<F>(&mut self, operation: F, operator_name: &str) -> Result<()>
    where F: FnOnce(f64) -> f64 {
        let x = self.pop()?;
        let result = operation(x);
        self.push(result);
        println!("  → {} {} = {}", x, operator_name, result);
        Ok(())
    }
}

impl ExecutionContext for SimpleRpnContext {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// ================================================================================================
/// Command handlers
///
///  - Stack commands
///  - Arithmetic functions
///
/// ================================================================================================

/// handler for push command
///
/// Push a number onto the stack
struct PushCommand;

impl CommandHandler for PushCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_context = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                expected_type: "RPN Calculator context".to_string(),
            })
        })?;

        // Parse the value arguments
        let cli_value = args.get("value").ok_or_else(|| {
            DynamicCliError::Parse(
                dynamic_cli::error::ParseError::MissingArgument {
                    argument: "value".to_string(),
                    command: "push".to_string(),
                }
            )
        })?;

        // convert in float
        let value = cli_value.parse::<f64>().map_err(|_| {
            DynamicCliError::Parse(
                dynamic_cli::error::ParseError::TypeParseError {
                    arg_name: "value".to_string(),
                    expected_type: "float".to_string(),
                    value: cli_value.clone(),
                    details: Some("not a valid number".to_string()),
                }
            )
        })?;

        rpn_context.push_x(value);
        rpn_context.display();
        Ok(())
    }
}

/// Handler for pop command
///
/// Removes and display the top value

struct PopCommand;

impl CommandHandler for PopCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        let value = rpn_ctx.pop();
        println!("  → Popped {:?}", value);
        rpn_ctx.display();
        Ok(())
    }
}

/// Handler for lastx command
///
/// Displays the last x register which stores the last value pushed

struct LastXCommand;

impl CommandHandler for LastXCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        let value = rpn_ctx.last_x();
        println!("  → LastX {:?}", value);
        rpn_ctx.display();
        Ok(())
    }
}

/// Handler for swap command
///
/// Exchange x register and y register in the stack

struct SwapCommand;

impl CommandHandler for SwapCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {

        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        rpn_ctx.swap();
        rpn_ctx.display();

        Ok(())
    }
}

/// Handler for peek command
///
/// Displays the top value without removes it

struct PeekCommand;

impl CommandHandler for PeekCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        match rpn_ctx.peek() {
            Some(value) => println!(" → Top value: {}", value),
            None => println!("  → The stack is empty"),
        }
        rpn_ctx.display();
        Ok(())
    }
}

/// Handler for show command
///
/// shows the entire stack as a list

struct ShowCommand;

impl CommandHandler for ShowCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        rpn_ctx.display();
        Ok(())
    }
}

/// Handler for clear command
///
/// sets the rpn context to default values

struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;
        rpn_ctx.clear();
        rpn_ctx.display();
        Ok(())
    }
}

/// Handler for add function
///
/// Pops two values and pushes their sum

struct AddCommand;

impl CommandHandler for AddCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;
        rpn_ctx.binary_op(|a , b| a + b, "+")?;
        Ok(())
    }
}

/// Handler for sub function
///
/// Pops two value and pushes their difference

struct SubCommand;

impl CommandHandler for SubCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        rpn_ctx.binary_op(|a , b| a - b, "-")?;
        Ok(())
    }
}

/// Handler for mul function
///
/// Pops two values and pushed their product

struct MulCommand;

impl CommandHandler for MulCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;
        rpn_ctx.binary_op(|a , b| a * b, "*")?;
        Ok(())
    }
}

/// Handler for div function
///
/// Pops two values and pushes their quotient

struct DivCommand;

impl CommandHandler for DivCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        rpn_ctx.binary_op(|a , b| a / b, "/")?;
        Ok(())
    }
}

/// Hander fon natural logarithm
///
/// Pops the value and pushes the natural logarithm

struct LnFunction;

impl CommandHandler for LnFunction {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let rpn_ctx = downcast_mut::<SimpleRpnContext>(context).ok_or_else(|| {
            DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "RPN Calculator context".to_string(),
                }
            )
        })?;

        rpn_ctx.single_op(|a | a.ln(), "ln")?;
        Ok(())
    }
}

/// ================================================================================================
/// Main application
///
///  - Load the configuration file
///  - Register command and function
///  - Build and run the app
/// ================================================================================================
fn main() -> Result<()> {
    println!("🔢 Simple RPN Calculator - Powered by dynamic-cli");
    println!("═════════════════════════════════════════════════\n");

    let app = CliBuilder::new()
        .config_file("examples/configs/simple_rpn.yaml")
        .context(Box::new(SimpleRpnContext::default()))
        .register_handler("push_command", Box::new(PushCommand))
        .register_handler("pop_command", Box::new(PopCommand))
        .register_handler("lastx_command", Box::new(LastXCommand))
        .register_handler("swap_command", Box::new(SwapCommand))
        .register_handler("peek_command", Box::new(PeekCommand))
        .register_handler("show_command", Box::new(ShowCommand))
        .register_handler("clear_command", Box::new(ClearCommand))
        .register_handler("add_function", Box::new(AddCommand))
        .register_handler("sub_function", Box::new(SubCommand))
        .register_handler("mul_function", Box::new(MulCommand))
        .register_handler("div_function", Box::new(DivCommand))
        .register_handler("ln_function", Box::new(LnFunction))
        .build()?;

    app.run()
}

/// ================================================================================================
/// Tests
/// ================================================================================================

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;
    use super::*;
    #[test]
    fn test_rpn_context_push_pop() {
        let mut ctx_test = SimpleRpnContext::default();

        ctx_test.push(35.0);
        ctx_test.push(10.0);
        assert_eq!(ctx_test.pop().unwrap(), 10.0);
        assert_eq!(ctx_test.pop().unwrap(), 35.0);
        assert!(ctx_test.pop().is_err()); // stack is empty
    }

    #[test]
    fn test_rpn_context_push_x() {
        let mut ctx_test = SimpleRpnContext::default();
        ctx_test.push_x(35.0);
        ctx_test.push(10.0);
        assert_eq!(ctx_test.last_x(), 35.0);
        assert_eq!(ctx_test.pop().unwrap(), 10.0);
        ctx_test.push_x(15.0);
        assert_eq!(ctx_test.last_x(), 15.0);
        assert_eq!(ctx_test.pop().unwrap(), 15.0);
        assert_eq!(ctx_test.pop().unwrap(), 35.0);
        assert!(ctx_test.pop().is_err());
    }

    #[test]
    fn test_rpn_context_binary_op() {
        let mut ctx_test = SimpleRpnContext::default();
        ctx_test.push_x(5.0);
        ctx_test.push_x(25.0);
        ctx_test.binary_op(|a, b| a * b, "*").unwrap();
        assert_eq!(ctx_test.peek(), Some(125.0));
    }

    #[test]
    fn test_rpn_context_unary_op() {
        let mut ctx_test = SimpleRpnContext::default();
        ctx_test.push_x(PI);
        ctx_test.single_op(|a| a.cos(), "cos").unwrap();
        assert_eq!(ctx_test.peek(), Some(-1.0));
    }

    #[test]
    fn test_rpn_context_clear() {
        let mut ctx_test = SimpleRpnContext::default();
        ctx_test.push(1.0);
        ctx_test.push(2.0);
        ctx_test.push(3.0);
        ctx_test.push(4.0);
        assert_eq!(ctx_test.stack.len(), 4);
        ctx_test.clear();
        assert_eq!(ctx_test.stack.len(), 0);
    }

    #[test]
    fn test_push_command() {
        let ctx_test = SimpleRpnContext::default();
        let mut rpn_execution_test : Box<dyn ExecutionContext> = Box::new(ctx_test);
        let mut _args = HashMap::new();
        let handler = PushCommand ;

        _args.insert("value".to_string(), "45.0".to_string());
        handler.execute(rpn_execution_test.as_mut(), &_args).unwrap();

        let ctx_test = dynamic_cli::context::downcast_ref::<SimpleRpnContext>(rpn_execution_test.as_ref()).unwrap();
        assert_eq!(ctx_test.peek(), Some(45.0));
    }

    #[test]
    fn test_ln_command() {
        let mut ctx_test = SimpleRpnContext::default();
        ctx_test.push_x(1.0);
        let mut rpn_execution_test : Box<dyn ExecutionContext> = Box::new(ctx_test);
        let args = HashMap::new();
        let handler = LnFunction;

        handler.execute(rpn_execution_test.as_mut(), &args).unwrap();
        let ctx_test = dynamic_cli::context::downcast_ref::<SimpleRpnContext>(rpn_execution_test.as_ref()).unwrap();
        assert_eq!(ctx_test.peek(), Some(0.0));
    }

    #[test]
    fn test_sequence_command() {
        let ctx_test = SimpleRpnContext::default();
        let mut rpn_execution_test : Box<dyn ExecutionContext> = Box::new(ctx_test);
        let mut _args = HashMap::new();

        // Pushes values

        let push_cmd = PushCommand ;
        _args.insert("value".to_string(), "45.0".to_string());
        push_cmd.execute(rpn_execution_test.as_mut(), &_args).unwrap();
        let ctx_copy = dynamic_cli::context::downcast_ref::<SimpleRpnContext>(rpn_execution_test.as_ref()).unwrap();
        assert_eq!(ctx_copy.peek(), Some(45.0));
        _args.insert("value".to_string(), "5".to_string());
        push_cmd.execute(rpn_execution_test.as_mut(), &_args).unwrap();
        let ctx_copy = dynamic_cli::context::downcast_ref::<SimpleRpnContext>(rpn_execution_test.as_ref()).unwrap();
        assert_eq!(ctx_copy.peek(), Some(5.0));
        assert_eq!(ctx_copy.last_x(), 5.0);

        let add_command = AddCommand ;
        add_command.execute(rpn_execution_test.as_mut(), &_args).unwrap();
        let ctx_test = dynamic_cli::context::downcast_ref::<SimpleRpnContext>(rpn_execution_test.as_ref()).unwrap();
        assert_eq!(ctx_test.peek(), Some(50.0));
        assert_eq!(ctx_test.last_x(), 5.0);

    }

}