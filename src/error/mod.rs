//! Error handling for dynamic-cli
//!
//! This module provides a hierarchical error system with clear and
//! contextual messages to facilitate debugging.
//!
//! ## Error Types
//!
//! - [`DynamicCliError`] : Main error encompassing all categories
//! - [`ConfigError`] : Configuration errors
//! - [`ParseError`] : Parsing errors
//! - [`ValidationError`] : Validation errors
//! - [`ExecutionError`] : Execution errors
//! - [`RegistryError`] : Registry errors
//!
//! ## Example
//!
//! ```
//! use dynamic_cli::error::{DynamicCliError, ConfigError};
//!
//! fn load_config() -> Result<(), DynamicCliError> {
//!     Err(ConfigError::FileNotFound {
//!         path: "config.yaml".into(),
//!     }.into())
//! }
//! ```

mod types;
mod display;
mod suggestions;

// Public re-exports
pub use types::*;
pub use display::{display_error, format_error};
pub use suggestions::find_similar_strings;

/// Specialized Result type for dynamic-cli
///
/// Uses [`DynamicCliError`] as the default error type.
pub type Result<T> = std::result::Result<T, DynamicCliError>;
