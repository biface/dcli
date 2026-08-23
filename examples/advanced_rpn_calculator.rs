//! # Advanced RPN calculator example
//!
//! An HP-41CX-flavored extension of the `rpn_calculator` example,
//! demonstrating more of `dynamic-cli`'s surface at once:
//!
//! ## Features (beyond the simple example)
//!
//! - Scientific functions: `sqrt`, `sq`, `inv`, `pow`, `exp`, `log10`,
//!   `sin`, `cos`, `tan`, `chs`, `pi`
//! - A 10-slot memory register bank (`sto`/`rcl`), à la HP-41 — simplified
//!   from the real HP-41CX's 100+ registers
//! - [`SysInfoPlugin`] and [`ConfigPlugin`] registered via
//!   [`CliBuilder::register_plugin`], both feature-gated
//! - Batch execution via [`CliApp::run_script`] (`--script <file>`)
//! - Loading a script from an already-running REPL session via `:load
//!   <file>` — nothing to wire up, this comes for free from
//!   `ReplInterface`
//!
//! ## Usage
//!
//! ```bash
//! # REPL mode (interactive)
//! cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin
//!
//! # CLI mode (single command)
//! cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin -- push 5
//! cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin -- push 4
//! cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin -- add
//!
//! # Batch mode — run a whole script of commands, one per line
//! cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin -- \
//!     --script examples/configs/advanced_rpn_demo.txt
//!
//! # From inside the REPL, load the same script mid-session:
//! #   arpn > :load examples/configs/advanced_rpn_demo.txt
//! ```
//!
//! ## RPN Basics
//!
//! Reverse Polish Notation is a postfix notation in which operators follow
//! their operands.
//!
//! ```text
//! Infix : 3 + 4
//! RPN   : 3 4 +
//!
//! Infix : (3 + 4) * 5
//! RPN   : 3 4 + 5 *
//! ```

use dynamic_cli::error::{ExecutionError, ParseError};
use dynamic_cli::prelude::*;
use std::any::Any;
use std::f64::consts::PI;

// ================================================================================================
// Execution context
// ================================================================================================

// Number of memory registers in the bank (HP-41-style sto/rcl, simplified
// from the real HP-41CX's 100+ registers down to a fixed, easy-to-scan 10).
const REGISTER_COUNT: usize = 10;

/// Execution context for the advanced RPN calculator.
///
/// Extends the simple example's stack + last-x register with a small
/// fixed-size memory bank, addressed by `sto <n>` / `rcl <n>` (`n` in
/// `0..REGISTER_COUNT`).
struct AdvancedRpnContext {
    /// Calculation stack: top of the stack is the last element pushed.
    stack: Vec<f64>,
    /// Last x is the register containing the last value pushed via `push`.
    last_x: f64,
    /// Memory registers, indexed `0..REGISTER_COUNT`.
    registers: [f64; REGISTER_COUNT],
}

impl Default for AdvancedRpnContext {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            last_x: 0.0,
            registers: [0.0; REGISTER_COUNT],
        }
    }
}

impl AdvancedRpnContext {
    /// Push a value onto the stack.
    fn push(&mut self, value: f64) {
        self.stack.push(value);
        println!("  → Pushed {:?}", self.stack.last());
    }

    /// Push a value onto the stack and the last-x register.
    fn push_x(&mut self, value: f64) {
        self.last_x = value;
        self.push(value);
    }

    /// Pop a value from the stack.
    fn pop(&mut self) -> Result<f64> {
        self.stack.pop().ok_or_else(|| {
            DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
                "Stack is empty"
            )))
        })
    }

    /// Show the last-x register.
    fn last_x(&self) -> f64 {
        self.last_x
    }

    /// Swap the top two elements of the stack.
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

    /// Peek at the top of the stack without removing it.
    fn peek(&self) -> Option<f64> {
        self.stack.last().copied()
    }

    /// Clear the stack (registers and last-x are untouched — mirrors the
    /// simple example's `clear`, scoped to the stack only).
    fn clear(&mut self) {
        self.stack.clear();
        println!("  → Stack cleared");
    }

    /// Display the stack content.
    fn display(&self) {
        if self.stack.is_empty() {
            println!("    Stack is empty");
        } else {
            println!("  Stack: {:?}", self.stack);
            println!("  Last X: {}", self.last_x);
        }
    }

    /// Store the top of the stack into register `index`, without popping
    /// it — matches the real HP-41's `STO` behaviour.
    fn store(&mut self, index: usize) -> Result<()> {
        let value = *self.stack.last().ok_or_else(|| {
            DynamicCliError::Execution(ExecutionError::CommandFailed(anyhow::anyhow!(
                "Stack is empty — nothing to store"
            )))
        })?;
        self.registers[index] = value;
        println!("  → Stored {} into register {}", value, index);
        Ok(())
    }

    /// Push the value stored in register `index` onto the stack.
    fn recall(&mut self, index: usize) {
        let value = self.registers[index];
        self.push_x(value);
        println!("  → Recalled register {}: {}", index, value);
    }

    fn binary_op<F>(&mut self, operation: F, operator_name: &str) -> Result<()>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let x = self.pop()?;
        let y = self.pop()?;
        let result = operation(x, y);
        self.push(result);
        println!("  → {} {} {} = {}", y, x, operator_name, result);
        Ok(())
    }

    fn single_op<F>(&mut self, operation: F, operator_name: &str) -> Result<()>
    where
        F: FnOnce(f64) -> f64,
    {
        let x = self.pop()?;
        let result = operation(x);
        self.push(result);
        println!("  → {} {} = {}", operator_name, x, result);
        Ok(())
    }
}

impl ExecutionContext for AdvancedRpnContext {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Downcast the context, or return the same `ContextDowncastFailed` error
/// every handler below would otherwise repeat verbatim.
fn downcast(context: &mut dyn ExecutionContext) -> Result<&mut AdvancedRpnContext> {
    downcast_mut::<AdvancedRpnContext>(context).ok_or_else(|| {
        DynamicCliError::Execution(ExecutionError::ContextDowncastFailed {
            expected_type: "Advanced RPN Calculator context".to_string(),
            suggestion: None,
        })
    })
}

/// Read and bounds-check a `register` argument (`0..REGISTER_COUNT`).
///
/// `ArgumentDefinition.validation` (the YAML `min`/`max` on `sto`/`rcl`)
/// is descriptive only — the parser does not enforce it automatically —
/// so the handler re-validates via [`validate_range`], the same free
/// function the framework exposes for exactly this purpose.
fn parse_register_index(args: &ParsedArgs) -> Result<usize> {
    let raw = args.get_scalar("register").ok_or_else(|| {
        DynamicCliError::Parse(ParseError::MissingArgument {
            argument: "register".to_string(),
            command: "sto/rcl".to_string(),
            suggestion: None,
        })
    })?;

    let index: i64 = raw.parse().map_err(|_| {
        DynamicCliError::Parse(ParseError::TypeParseError {
            arg_name: "register".to_string(),
            expected_type: "integer".to_string(),
            value: raw.to_string(),
            details: Some("not a valid integer".to_string()),
        })
    })?;

    validate_range(
        index as f64,
        "register",
        Some(0.0),
        Some((REGISTER_COUNT - 1) as f64),
    )?;

    Ok(index as usize)
}

// ================================================================================================
// Command handlers — stack manipulation (unchanged from the simple example)
// ================================================================================================

// Handler for push command — push a number onto the stack.
struct PushCommand;

impl CommandHandler for PushCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
        let rpn_context = downcast(context)?;

        let cli_value = args.get_scalar("value").ok_or_else(|| {
            DynamicCliError::Parse(ParseError::MissingArgument {
                argument: "value".to_string(),
                command: "push".to_string(),
                suggestion: None,
            })
        })?;

        let value = cli_value.parse::<f64>().map_err(|_| {
            DynamicCliError::Parse(ParseError::TypeParseError {
                arg_name: "value".to_string(),
                expected_type: "float".to_string(),
                value: cli_value.to_string(),
                details: Some("not a valid number".to_string()),
            })
        })?;

        rpn_context.push_x(value);
        rpn_context.display();
        Ok(())
    }
}

// Handler for pop command — remove and display the top value.
struct PopCommand;

impl CommandHandler for PopCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        let value = rpn_ctx.pop();
        println!("  → Popped {:?}", value);
        rpn_ctx.display();
        Ok(())
    }
}

// Handler for lastx command.
struct LastXCommand;

impl CommandHandler for LastXCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        println!("  → LastX {:?}", rpn_ctx.last_x());
        rpn_ctx.display();
        Ok(())
    }
}

// Handler for swap command.
struct SwapCommand;

impl CommandHandler for SwapCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        rpn_ctx.swap();
        rpn_ctx.display();
        Ok(())
    }
}

// Handler for peek command.
struct PeekCommand;

impl CommandHandler for PeekCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        println!("  → Peek {:?}", rpn_ctx.peek());
        Ok(())
    }
}

// Handler for show command.
struct ShowCommand;

impl CommandHandler for ShowCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        rpn_ctx.display();
        Ok(())
    }
}

// Handler for clear command.
struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        rpn_ctx.clear();
        Ok(())
    }
}

// ================================================================================================
// Command handlers — arithmetic and scientific functions
// ================================================================================================

// Handler for add function — pops two values, pushes their sum.
struct AddCommand;

impl CommandHandler for AddCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.binary_op(|a, b| a + b, "+")
    }
}

// Handler for sub function — pops two values, pushes their difference.
struct SubCommand;

impl CommandHandler for SubCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.binary_op(|a, b| b - a, "-")
    }
}

// Handler for mul function — pops two values, pushes their product.
struct MulCommand;

impl CommandHandler for MulCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.binary_op(|a, b| a * b, "*")
    }
}

// Handler for div function — pops two values, pushes their quotient.
struct DivCommand;

impl CommandHandler for DivCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.binary_op(|a, b| b / a, "/")
    }
}

// Handler for pow function — pops (y, x), pushes y raised to the power x.
struct PowCommand;

impl CommandHandler for PowCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.binary_op(|x, y| y.powf(x), "^")
    }
}

// Handler for ln function — natural logarithm.
struct LnCommand;

impl CommandHandler for LnCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.ln(), "ln")
    }
}

// Handler for log10 function — base-10 logarithm.
struct Log10Command;

impl CommandHandler for Log10Command {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.log10(), "log10")
    }
}

// Handler for exp function — natural exponential.
struct ExpCommand;

impl CommandHandler for ExpCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.exp(), "exp")
    }
}

// Handler for sqrt function — square root.
struct SqrtCommand;

impl CommandHandler for SqrtCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.sqrt(), "sqrt")
    }
}

// Handler for sq function — square (x^2).
struct SqCommand;

impl CommandHandler for SqCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a * a, "sq")
    }
}

// Handler for inv function — reciprocal (1/x).
struct InvCommand;

impl CommandHandler for InvCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| 1.0 / a, "inv")
    }
}

// Handler for chs function — change sign.
struct ChsCommand;

impl CommandHandler for ChsCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| -a, "chs")
    }
}

// Handler for sin function — sine (radians).
struct SinCommand;

impl CommandHandler for SinCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.sin(), "sin")
    }
}

// Handler for cos function — cosine (radians).
struct CosCommand;

impl CommandHandler for CosCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.cos(), "cos")
    }
}

// Handler for tan function — tangent (radians).
struct TanCommand;

impl CommandHandler for TanCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        downcast(context)?.single_op(|a| a.tan(), "tan")
    }
}

// Handler for pi command — push the constant pi.
struct PiCommand;

impl CommandHandler for PiCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, _args: &ParsedArgs) -> Result<()> {
        let rpn_ctx = downcast(context)?;
        rpn_ctx.push_x(PI);
        rpn_ctx.display();
        Ok(())
    }
}

// ================================================================================================
// Command handlers — memory registers
// ================================================================================================

// Handler for sto command — store the top of the stack into a register.
struct StoCommand;

impl CommandHandler for StoCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
        let index = parse_register_index(args)?;
        downcast(context)?.store(index)
    }
}

// Handler for rcl command — push a register's value onto the stack.
struct RclCommand;

impl CommandHandler for RclCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
        let index = parse_register_index(args)?;
        let rpn_ctx = downcast(context)?;
        rpn_ctx.recall(index);
        rpn_ctx.display();
        Ok(())
    }
}

// ================================================================================================
// Main application
//
//  - Load the configuration file (once, shared with ConfigPlugin)
//  - Register command handlers and the SysInfo/Config plugins
//  - Build, then dispatch to CLI, REPL, or batch script mode
// ================================================================================================
fn main() -> Result<()> {
    println!("🔢 Advanced RPN Calculator — HP-41CX-flavored — Powered by dynamic-cli");
    println!("════════════════════════════════════════════════════════════════════\n");

    // Loaded once so the same CommandsConfig can be handed both to the
    // builder and to ConfigPlugin::with_config() — CliBuilder::build()
    // does not attach config to plugins on its own (see #44/#47).
    let config = dynamic_cli::config::load_config("examples/configs/advanced_rpn.yaml")?;

    let app = CliBuilder::new()
        .config(config.clone())
        .context(Box::new(AdvancedRpnContext::default()))
        .register_sync_handler("push_command", Box::new(PushCommand))
        .register_sync_handler("pop_command", Box::new(PopCommand))
        .register_sync_handler("lastx_command", Box::new(LastXCommand))
        .register_sync_handler("swap_command", Box::new(SwapCommand))
        .register_sync_handler("peek_command", Box::new(PeekCommand))
        .register_sync_handler("show_command", Box::new(ShowCommand))
        .register_sync_handler("clear_command", Box::new(ClearCommand))
        .register_sync_handler("add_function", Box::new(AddCommand))
        .register_sync_handler("sub_function", Box::new(SubCommand))
        .register_sync_handler("mul_function", Box::new(MulCommand))
        .register_sync_handler("div_function", Box::new(DivCommand))
        .register_sync_handler("pow_function", Box::new(PowCommand))
        .register_sync_handler("ln_function", Box::new(LnCommand))
        .register_sync_handler("log10_function", Box::new(Log10Command))
        .register_sync_handler("exp_function", Box::new(ExpCommand))
        .register_sync_handler("sqrt_function", Box::new(SqrtCommand))
        .register_sync_handler("sq_function", Box::new(SqCommand))
        .register_sync_handler("inv_function", Box::new(InvCommand))
        .register_sync_handler("chs_function", Box::new(ChsCommand))
        .register_sync_handler("sin_function", Box::new(SinCommand))
        .register_sync_handler("cos_function", Box::new(CosCommand))
        .register_sync_handler("tan_function", Box::new(TanCommand))
        .register_sync_handler("pi_function", Box::new(PiCommand))
        .register_sync_handler("sto_command", Box::new(StoCommand))
        .register_sync_handler("rcl_command", Box::new(RclCommand))
        .register_plugin(Box::new(SysInfoPlugin::new()))
        .register_plugin(Box::new(ConfigPlugin::new().with_config(config)))
        .build()?;

    // `--script <path>` switches to batch mode instead of the usual
    // CLI/REPL auto-detection (#41). Anything else falls through to
    // `app.run()` (which re-reads `std::env::args()` itself), where
    // `:load <path>` remains available from inside an interactive REPL
    // session.
    let cli_args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(script_flag_pos) = cli_args.iter().position(|a| a == "--script") {
        let path = cli_args.get(script_flag_pos + 1).cloned().ok_or_else(|| {
            DynamicCliError::Parse(ParseError::InvalidSyntax {
                details: "--script requires a file path".to_string(),
                hint: Some("Usage: --script <path/to/script.txt>".to_string()),
            })
        })?;

        let outcome = app.run_script(path, ScriptErrorPolicy::Continue)?;
        println!(
            "\n{}/{} script line(s) succeeded",
            outcome.lines_succeeded, outcome.lines_executed
        );
        for (line_number, error) in &outcome.failures {
            eprintln!("  line {}: {}", line_number, error);
        }
        return Ok(());
    }

    app.run()
}

// ================================================================================================
// Tests
// ================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_context_push_pop() {
        let mut ctx = AdvancedRpnContext::default();
        ctx.push(35.0);
        ctx.push(10.0);
        assert_eq!(ctx.pop().unwrap(), 10.0);
        assert_eq!(ctx.pop().unwrap(), 35.0);
        assert!(ctx.pop().is_err());
    }

    #[test]
    fn test_context_sub_and_div_use_hp_convention() {
        // HP RPN convention: "10 ENTER 3 -" = 10 - 3 = 7 (Y - X, not X - Y).
        let mut ctx = AdvancedRpnContext::default();
        ctx.push_x(10.0);
        ctx.push_x(3.0);
        ctx.binary_op(|a, b| b - a, "-").unwrap();
        assert_eq!(ctx.peek(), Some(7.0));

        // "20 ENTER 4 /" = 20 / 4 = 5 (Y / X, not X / Y).
        let mut ctx = AdvancedRpnContext::default();
        ctx.push_x(20.0);
        ctx.push_x(4.0);
        ctx.binary_op(|a, b| b / a, "/").unwrap();
        assert_eq!(ctx.peek(), Some(5.0));
    }

    #[test]
    fn test_context_binary_and_unary_ops() {
        let mut ctx = AdvancedRpnContext::default();
        ctx.push_x(5.0);
        ctx.push_x(25.0);
        ctx.binary_op(|a, b| a * b, "*").unwrap();
        assert_eq!(ctx.peek(), Some(125.0));

        ctx.clear();
        ctx.push_x(PI);
        ctx.single_op(|a| a.cos(), "cos").unwrap();
        assert!((ctx.peek().unwrap() - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_context_sto_rcl_roundtrip() {
        let mut ctx = AdvancedRpnContext::default();
        ctx.push(42.0);
        ctx.store(3).unwrap();
        // store() does not pop.
        assert_eq!(ctx.peek(), Some(42.0));

        ctx.clear();
        assert_eq!(ctx.peek(), None);

        ctx.recall(3);
        assert_eq!(ctx.peek(), Some(42.0));
    }

    #[test]
    fn test_context_store_on_empty_stack_errors() {
        let mut ctx = AdvancedRpnContext::default();
        assert!(ctx.store(0).is_err());
    }

    #[test]
    fn test_context_recall_default_register_is_zero() {
        let mut ctx = AdvancedRpnContext::default();
        ctx.recall(5);
        assert_eq!(ctx.peek(), Some(0.0));
    }

    #[test]
    fn test_parse_register_index_accepts_bounds() {
        let mut args = HashMap::new();
        args.insert("register".to_string(), "0".to_string());
        assert_eq!(
            parse_register_index(&ParsedArgs::from_scalars(args)).unwrap(),
            0
        );

        let mut args = HashMap::new();
        args.insert("register".to_string(), "9".to_string());
        assert_eq!(
            parse_register_index(&ParsedArgs::from_scalars(args)).unwrap(),
            9
        );
    }

    #[test]
    fn test_parse_register_index_rejects_out_of_range() {
        let mut args = HashMap::new();
        args.insert("register".to_string(), "10".to_string());
        assert!(parse_register_index(&ParsedArgs::from_scalars(args)).is_err());

        let mut args = HashMap::new();
        args.insert("register".to_string(), "-1".to_string());
        assert!(parse_register_index(&ParsedArgs::from_scalars(args)).is_err());
    }

    #[test]
    fn test_parse_register_index_rejects_non_integer() {
        let mut args = HashMap::new();
        args.insert("register".to_string(), "abc".to_string());
        assert!(parse_register_index(&ParsedArgs::from_scalars(args)).is_err());
    }

    #[test]
    fn test_pow_command_sequence() {
        let ctx_test = AdvancedRpnContext::default();
        let mut exec: Box<dyn ExecutionContext> = Box::new(ctx_test);

        let push = PushCommand;
        let mut args = HashMap::new();
        args.insert("value".to_string(), "2".to_string());
        push.execute(exec.as_mut(), &ParsedArgs::from_scalars(args))
            .unwrap();

        let mut args = HashMap::new();
        args.insert("value".to_string(), "10".to_string());
        push.execute(exec.as_mut(), &ParsedArgs::from_scalars(args))
            .unwrap();

        // Stack (bottom → top): [2, 10]. pow pops (x=10, y=2) and computes
        // y^x = 2^10 = 1024, matching binary_op's (x, y) pop order.
        PowCommand
            .execute(exec.as_mut(), &ParsedArgs::from_scalars(HashMap::new()))
            .unwrap();

        let ctx = downcast_ref::<AdvancedRpnContext>(exec.as_ref()).unwrap();
        assert_eq!(ctx.peek(), Some(1024.0));
    }

    #[test]
    fn test_sto_rcl_command_roundtrip() {
        let ctx_test = AdvancedRpnContext::default();
        let mut exec: Box<dyn ExecutionContext> = Box::new(ctx_test);

        let push = PushCommand;
        let mut args = HashMap::new();
        args.insert("value".to_string(), "7".to_string());
        push.execute(exec.as_mut(), &ParsedArgs::from_scalars(args))
            .unwrap();

        let mut args = HashMap::new();
        args.insert("register".to_string(), "4".to_string());
        StoCommand
            .execute(exec.as_mut(), &ParsedArgs::from_scalars(args.clone()))
            .unwrap();

        ClearCommand
            .execute(exec.as_mut(), &ParsedArgs::from_scalars(HashMap::new()))
            .unwrap();

        RclCommand
            .execute(exec.as_mut(), &ParsedArgs::from_scalars(args))
            .unwrap();

        let ctx = downcast_ref::<AdvancedRpnContext>(exec.as_ref()).unwrap();
        assert_eq!(ctx.peek(), Some(7.0));
    }

    #[test]
    fn test_sqrt_command() {
        let ctx_test = AdvancedRpnContext::default();
        let mut exec: Box<dyn ExecutionContext> = Box::new(ctx_test);

        let push = PushCommand;
        let mut args = HashMap::new();
        args.insert("value".to_string(), "16".to_string());
        push.execute(exec.as_mut(), &ParsedArgs::from_scalars(args))
            .unwrap();

        SqrtCommand
            .execute(exec.as_mut(), &ParsedArgs::from_scalars(HashMap::new()))
            .unwrap();

        let ctx = downcast_ref::<AdvancedRpnContext>(exec.as_ref()).unwrap();
        assert_eq!(ctx.peek(), Some(4.0));
    }
}
