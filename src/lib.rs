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
//! use dynamic_cli::prelude::*;
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
//!         let default_name = "World".to_string();
//!         let name = args.get("name").unwrap_or(&default_name);
//!         println!("Hello, {}!", name);
//!         Ok(())
//!     }
//! }
//!
//! // 3. Load configuration and register commands
//! # fn main() -> dynamic_cli::Result<()> {
//! use dynamic_cli::config::loader::load_config;
//!
//! let config = load_config("commands.yaml")?;
//! let mut registry = CommandRegistry::new();
//! registry.register(config.commands[0].clone(), Box::new(HelloCommand))?;
//!
//! // 4. Parse and execute
//! let parser = ReplParser::new(&registry);
//! let parsed = parser.parse_line("hello World")?;
//!
//! let mut context = MyContext::default();
//! let handler = registry.get_handler(&parsed.command_name).unwrap();
//! handler.execute(&mut context, &parsed.arguments)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! The framework is organized into modules:
//!
//! - [`error`]: Error types and handling
//! - [`config`]: Configuration file loading and validation
//! - [`context`]: Execution context trait
//! - [`executor`]: Command execution
//! - [`registry`]: Command and handler registry
//! - [`parser`]: CLI and REPL argument parsing
//! - [`validator`]: Argument validation
//!
//! ## Module Status
//!
//! - ✅ Complete: error, config, context, executor, registry, parser, validator, interface, builder
//! - 📋 Planned: utils, examples
//!
//! ## Examples
//!
//! See the documentation for each module for detailed examples.

// ============================================================================
// PUBLIC MODULES (Complete and ready to use)
// ============================================================================

pub mod builder;
pub mod config;
pub mod context;
pub mod error;
pub mod executor;
pub mod interface;
pub mod parser;
pub mod registry;
pub mod utils;
pub mod validator;

// ============================================================================
// PUBLIC RE-EXPORTS (For convenience)
// ============================================================================

// Core traits
pub use context::{downcast_mut, downcast_ref, ExecutionContext};
pub use executor::CommandHandler;

// Error handling
pub use error::{DynamicCliError, Result};

// Configuration types
pub use config::schema::{
    ArgumentDefinition, ArgumentType, CommandDefinition, CommandsConfig, Metadata,
    OptionDefinition, ValidationRule,
};

// Registry
pub use registry::CommandRegistry;

// Parser types
pub use parser::{CliParser, ParsedCommand, ReplParser};

// Validator functions
pub use validator::{validate_file_exists, validate_file_extension, validate_range};

// Interface types
pub use interface::{CliInterface, ReplInterface};

// Builder types
pub use builder::{CliApp, CliBuilder};

// Utility functions
pub use utils::{
    detect_type, format_bytes, format_duration, get_extension, has_extension, is_blank, normalize,
    normalize_path, parse_bool, parse_float, parse_int, truncate,
};

// ============================================================================
// PRELUDE MODULE (Quick imports)
// ============================================================================

/// Prelude module for quickly importing essential types
///
/// This module re-exports the most commonly used types and traits,
/// allowing you to import everything with a single `use` statement.
///
/// # Example
///
/// ```
/// use dynamic_cli::prelude::*;
///
/// // Now you have access to:
/// // - ExecutionContext, downcast_ref, downcast_mut
/// // - CommandHandler
/// // - DynamicCliError, Result
/// // - CommandRegistry
/// // - ParsedCommand, CliParser, ReplParser
/// // - validate_file_exists, validate_file_extension, validate_range
/// // - Common config types (ArgumentType, CommandsConfig)
/// // - CliBuilder, CliApp
/// // - Utility functions (parse_int, parse_bool, is_blank, etc.)
/// ```
pub mod prelude {
    // Context management
    pub use crate::context::{downcast_mut, downcast_ref, ExecutionContext};

    // Command handling
    pub use crate::executor::CommandHandler;

    // Error handling
    pub use crate::error::{DynamicCliError, Result};

    // Configuration
    pub use crate::config::schema::{ArgumentType, CommandsConfig};

    // Registry
    pub use crate::registry::CommandRegistry;

    // Parsing
    pub use crate::parser::{CliParser, ParsedCommand, ReplParser};

    // Validation
    pub use crate::validator::{validate_file_exists, validate_file_extension, validate_range};

    // Interface
    pub use crate::interface::{CliInterface, ReplInterface};

    // Builder
    pub use crate::builder::{CliApp, CliBuilder};

    // Utilities (most commonly used)
    pub use crate::utils::{detect_type, is_blank, normalize, parse_bool, parse_float, parse_int};
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that prelude imports work correctly
    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;

        // If this compiles, prelude imports are working
        let _: Option<&dyn ExecutionContext> = None;
        let _: Option<&dyn CommandHandler> = None;
    }

    /// Verify that individual module imports work
    #[test]
    fn test_module_imports() {
        use crate::config::schema::CommandsConfig;
        use crate::parser::ParsedCommand;
        use crate::registry::CommandRegistry;

        // If this compiles, module structure is correct
        let _config = CommandsConfig::minimal();
        let _registry = CommandRegistry::new();
        let _parsed = ParsedCommand {
            command_name: "test".to_string(),
            arguments: std::collections::HashMap::new(),
        };
    }

    /// Verify that re-exports work
    #[test]
    fn test_reexports() {
        // These should be accessible from the crate root
        let _: Option<&dyn ExecutionContext> = None;
        let _: Option<&dyn CommandHandler> = None;
        let _registry = CommandRegistry::new();

        // If this compiles, re-exports are working
    }
}
