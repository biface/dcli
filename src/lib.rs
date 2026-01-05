//! # dynamic-cli
//!
//! A framework for creating configurable CLI and REPL applications via YAML/JSON.
//!
//! ## Overview
//!
//! **dynamic-cli** allows you to define your application's command-line interface
//! in a configuration file rather than coding it manually. The framework
//! automatically generates:
//! - Argument parsing
//! - Input validation
//! - Contextual help
//! - Interactive mode (REPL)
//! - Error handling with suggestions
//!
//! ## Quick Start
//!
//! ```no_run
//! use dynamic_cli::{CliBuilder, CommandHandler, ExecutionContext};
//! use std::collections::HashMap;
//!
//! // 1. Define the execution context
//! #[derive(Default)]
//! struct MyContext;
//!
//! impl ExecutionContext for MyContext {
//!     fn as_any(&self) -> &dyn std::any::Any { self }
//!     fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
//! }
//!
//! // 2. Implement a command handler
//! struct HelloCommand;
//!
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _context: &mut dyn ExecutionContext,
//!         args: &HashMap<String, String>,
//!     ) -> dynamic_cli::Result<()> {
//!         let name = args.get("name").unwrap_or(&"World".to_string());
//!         println!("Hello, {}!", name);
//!         Ok(())
//!     }
//! }
//!
//! // 3. Build and run the application
//! # fn main() -> dynamic_cli::Result<()> {
//! CliBuilder::new()
//!     .config_file("commands.yaml")
//!     .context(Box::new(MyContext))
//!     .register_handler("hello_handler", Box::new(HelloCommand))
//!     .build()?
//!     .run()
//! # }
//! ```
//!
//! ## Architecture
//!
//! The framework is organized into modules:
//!
//! - [`config`]: Configuration file loading and validation
//! - [`registry`]: Command and handler registry
//! - [`parser`]: CLI and REPL argument parsing
//! - [`validator`]: Argument validation
//! - [`executor`]: Command execution
//! - [`context`]: Execution context trait
//! - [`interface`]: CLI and REPL interfaces
//! - [`error`]: Error types and handling
//!
//! ## Examples
//!
//! See the `examples/` directory for complete usage examples.

// Public modules
// pub mod config;
// pub mod registry;
//pub mod parser;
//pub mod validator;
//pub mod executor;
//pub mod context;
//pub mod interface;
pub mod error;
mod config;
mod context;
mod executor;
// Internal modules
//mod builder;
//mod utils;

// Public re-exports for ease of use
//pub use builder::{CliBuilder, CliApp};
//pub use context::{ExecutionContext, ExecutionContextExt};
//pub use executor::CommandHandler;
pub use error::{DynamicCliError, Result};

// Re-exports of common configuration types
//pub use config::schema::{
//    CommandsConfig,
//    CommandDefinition,
//    ArgumentDefinition,
//    OptionDefinition,
//    ArgumentType,
//};

/// Prelude module for quickly importing essential types
///
/// # Example
///
/// ```
/// use dynamic_cli::prelude::*;
/// ```
pub mod prelude {
//    pub use crate::builder::{CliBuilder, CliApp};
    pub use crate::context::{ExecutionContext, downcast_ref, downcast_mut};
    pub use crate::executor::CommandHandler;
    pub use crate::error::{DynamicCliError, Result};
    pub use crate::config::schema::{CommandsConfig, ArgumentType};
}

//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    /// Basic test to verify imports work
//    #[test]
//    fn test_prelude_imports() {
//        use crate::prelude::*;
//
//        // If this compiles, we're good
//        let _builder = CliBuilder::new();
//    }
//}
