//! Command registry implementation
//!
//! This module provides the central registry for storing and retrieving
//! command definitions and their associated handlers.
//!
//! # Architecture
//!
//! The registry maintains two main data structures:
//! - A map of command names to their definitions and handlers
//! - A map of aliases to canonical command names
//!
//! This design allows O(1) lookup by both command name and alias.
//!
//! # Example
//!
//! ```
//! use dynamic_cli::registry::CommandRegistry;
//! use dynamic_cli::config::schema::CommandDefinition;
//! use dynamic_cli::executor::{CommandHandler, ParsedArgs};
//!
//! // Create a registry
//! let mut registry = CommandRegistry::new();
//!
//! // Define a command
//! let definition = CommandDefinition {
//!     name: "hello".to_string(),
//!     aliases: vec!["hi".to_string(), "greet".to_string()],
//!     description: "Say hello".to_string(),
//!     required: false,
//!     arguments: vec![],
//!     options: vec![],
//!     implementation: "hello_handler".to_string(),
//!     continue_on_failure: false,
//!     requires_success: false,
//! };
//!
//! // Create a handler
//! struct HelloCommand;
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _ctx: &mut dyn dynamic_cli::context::ExecutionContext,
//!         _args: &ParsedArgs,
//!     ) -> dynamic_cli::Result<()> {
//!         println!("Hello!");
//!         Ok(())
//!     }
//! }
//!
//! // Register the command
//! registry.register_sync(definition, Box::new(HelloCommand))?;
//!
//! // Retrieve by name
//! assert!(registry.get_handler_sync("hello").is_some());
//!
//! // Retrieve by alias
//! assert_eq!(registry.resolve_name("hi"), Some("hello"));
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::config::schema::CommandDefinition;
use crate::error::{RegistryError, Result};
use crate::executor::{AsyncCommandHandler, CommandHandler};
use std::collections::HashMap;

/// Internal storage for a single registered command's handler.
///
/// Private — never leaks into the public API. `get_handler_sync()` /
/// `get_handler_async()` return `None` when queried against the wrong
/// variant, so callers never need to know this enum exists. See DD-022 for
/// the rationale behind unifying sync and async storage in one map instead
/// of two parallel `HashMap`s.
enum StoredHandler {
    Sync(Box<dyn CommandHandler>),
    Async(Box<dyn AsyncCommandHandler>),
}
/// Central registry for commands and their handlers
///
/// The registry stores all registered commands along with their definitions
/// and handlers. It provides efficient lookup by both command name and alias.
///
/// # Thread Safety
///
/// The registry is designed to be constructed once during application startup
/// and then shared immutably across the application. For multi-threaded access,
/// wrap it in `Arc<CommandRegistry>`.
///
/// # Example
///
/// ```
/// use dynamic_cli::registry::CommandRegistry;
/// use dynamic_cli::config::schema::CommandDefinition;
/// use dynamic_cli::executor::CommandHandler;
/// use std::collections::HashMap;
///
/// let mut registry = CommandRegistry::new();
///
/// // Register commands during initialization
/// # let definition = CommandDefinition {
/// #     name: "test".to_string(),
/// #     aliases: vec![],
/// #     description: "Test".to_string(),
/// #     required: false,
/// #     arguments: vec![],
/// #     options: vec![],
/// #     implementation: "test_handler".to_string(),
/// #     continue_on_failure: false,
/// #     requires_success: false,
/// # };
/// # struct TestCommand;
/// # impl CommandHandler for TestCommand {
/// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
/// # }
/// registry.register_sync(definition, Box::new(TestCommand))?;
///
/// // Use throughout the application
/// if let Some(handler) = registry.get_handler_sync("test") {
///     // Execute the command
/// }
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub struct CommandRegistry {
    /// Map of command names to their data
    /// Key: canonical command name
    /// Value: (CommandDefinition, Box<dyn CommandHandler>)
    commands: HashMap<String, (CommandDefinition, StoredHandler)>,

    /// Map of aliases to canonical command names
    /// Key: alias
    /// Value: canonical command name
    ///
    /// This allows O(1) resolution of aliases to command names.
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    /// Create a new empty registry
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    ///
    /// let registry = CommandRegistry::new();
    /// assert_eq!(registry.list_commands().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Checks that `name` is free to use as a command name or alias.
    ///
    /// Shared by [`register_sync`][Self::register_sync] and
    /// [`register_async`][Self::register_async] — a name can never belong
    /// to both a sync and an async handler, nor be duplicated as a command
    /// or an alias. Checked against the single unified `commands` map, so
    /// this one call covers both storage kinds.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::DuplicateRegistration`] if `name` is already a
    ///   registered command (sync or async).
    /// - [`RegistryError::DuplicateAlias`] if `name` is already registered
    ///   as an alias of another command.
    fn check_name_available(&self, name: &str) -> Result<()> {
        if self.commands.contains_key(name) {
            return Err(RegistryError::DuplicateRegistration {
                name: name.to_string(),
                suggestion: None,
            }
            .into());
        }

        if let Some(existing_cmd) = self.aliases.get(name) {
            return Err(RegistryError::DuplicateAlias {
                alias: name.to_string(),
                existing_command: existing_cmd.clone(),
                suggestion: None,
            }
            .into());
        }

        Ok(())
    }

    /// Registers every alias declared in `definition` as pointing to
    /// `definition.name`. Called by both `register_sync` and
    /// `register_async` after `check_name_available` has confirmed there is
    /// no conflict.
    fn insert_aliases(&mut self, definition: CommandDefinition) {
        for alias in &definition.aliases {
            self.aliases.insert(alias.clone(), definition.name.clone());
        }
    }

    /// Checks a handler's declared [`expected_fault_tolerance()`][ceft]
    /// against `definition.continue_on_failure` (DD-028).
    ///
    /// Called by both `register_sync` and `register_async` after
    /// `check_name_available` has confirmed there is no conflict, and
    /// before the definition/handler pair is actually stored. Does nothing
    /// when the handler expresses no opinion (`None`, the default) —
    /// existing handlers that never heard of DD-028 are entirely
    /// unaffected.
    ///
    /// [ceft]: crate::executor::CommandHandler::expected_fault_tolerance
    ///
    /// # Errors
    ///
    /// [`RegistryError::FaultToleranceMismatch`] if the handler declared
    /// `Some(expected)` and `expected != definition.continue_on_failure`.
    fn check_fault_tolerance(definition: &CommandDefinition, expected: Option<bool>) -> Result<()> {
        if let Some(expected) = expected {
            if expected != definition.continue_on_failure {
                return Err(RegistryError::fault_tolerance_mismatch(
                    &definition.name,
                    expected,
                    definition.continue_on_failure,
                )
                .into());
            }
        }
        Ok(())
    }

    /// Register a command with its (sync) handler
    ///
    /// This method registers a command definition along with its handler.
    /// It also registers all aliases for the command.
    ///
    /// Renamed from `register()` in v0.5.0 for symmetry with
    /// [`register_async`][Self::register_async]. `register()` remains
    /// available as a deprecated alias until v1.0.0 (DD-022).
    ///
    /// # Arguments
    ///
    /// * `definition` - The command definition from the configuration
    /// * `handler` - The handler implementation for this command
    ///
    /// # Returns
    ///
    /// - `Ok(())` if registration succeeds
    /// - `Err(RegistryError)` if:
    ///   - A command with the same name is already registered (sync or async)
    ///   - An alias conflicts with an existing command or alias
    ///   - The handler's declared `expected_fault_tolerance()` (DD-028)
    ///     contradicts `definition.continue_on_failure`
    ///
    /// # Errors
    ///
    /// - [`RegistryError::DuplicateRegistration`] if the command name already exists
    /// - [`RegistryError::DuplicateAlias`] if an alias is already in use
    /// - [`RegistryError::FaultToleranceMismatch`] if the handler's
    ///   [`expected_fault_tolerance()`][crate::executor::CommandHandler::expected_fault_tolerance]
    ///   disagrees with `definition.continue_on_failure` (DD-028)
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    /// use dynamic_cli::config::schema::CommandDefinition;
    /// use dynamic_cli::executor::{CommandHandler, ParsedArgs};
    ///
    /// let mut registry = CommandRegistry::new();
    ///
    /// let definition = CommandDefinition {
    ///     name: "simulate".to_string(),
    ///     aliases: vec!["sim".to_string(), "run".to_string()],
    ///     description: "Run simulation".to_string(),
    ///     required: false,
    ///     arguments: vec![],
    ///     options: vec![],
    ///     implementation: "sim_handler".to_string(),
    ///     continue_on_failure: false,
    ///     requires_success: false,
    /// };
    ///
    /// struct SimCommand;
    /// impl CommandHandler for SimCommand {
    ///     fn execute(
    ///         &self,
    ///         _: &mut dyn dynamic_cli::context::ExecutionContext,
    ///         _: &ParsedArgs,
    ///     ) -> dynamic_cli::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Register the command
    /// registry.register_sync(definition, Box::new(SimCommand))?;
    ///
    /// // Can now access by name or alias
    /// assert!(registry.get_handler_sync("simulate").is_some());
    /// assert_eq!(registry.resolve_name("sim"), Some("simulate"));
    /// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
    /// ```
    pub fn register_sync(
        &mut self,
        definition: CommandDefinition,
        handler: Box<dyn CommandHandler>,
    ) -> Result<()> {
        self.check_name_available(&definition.name)?;
        for alias in &definition.aliases {
            self.check_name_available(alias)?;
        }
        Self::check_fault_tolerance(&definition, handler.expected_fault_tolerance())?;
        self.insert_aliases(definition.clone());
        self.commands.insert(
            definition.name.clone(),
            (definition, StoredHandler::Sync(handler)),
        );
        Ok(())
    }

    /// Deprecated alias for [`register_sync`][Self::register_sync].
    ///
    /// Kept for backward compatibility with pre-0.5.0 consumers (e.g.
    /// `chrom-rs`). Scheduled for removal in v1.0.0, batched with the other
    /// breaking changes tracked in the v1.0.0 API cleanup issue.
    #[deprecated(
        since = "0.5.0",
        note = "renamed to `register_sync` for symmetry with `register_async`; \
                will be removed in 1.0.0"
    )]
    pub fn register(
        &mut self,
        definition: CommandDefinition,
        handler: Box<dyn CommandHandler>,
    ) -> Result<()> {
        self.register_sync(definition, handler)
    }

    /// Register a command with its async handler (DD-022)
    ///
    /// Additive counterpart of [`register_sync`][Self::register_sync] —
    /// same conflict-detection rules (checked against both sync and async
    /// registrations sharing the unified internal storage, plus aliases),
    /// same alias handling.
    ///
    /// # Errors
    ///
    /// - [`RegistryError::DuplicateRegistration`] if the command name already exists
    /// - [`RegistryError::DuplicateAlias`] if an alias is already in use
    /// - [`RegistryError::FaultToleranceMismatch`] if the handler's
    ///   [`expected_fault_tolerance()`][crate::executor::AsyncCommandHandler::expected_fault_tolerance]
    ///   disagrees with `definition.continue_on_failure` (DD-028)
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    /// use dynamic_cli::config::schema::CommandDefinition;
    /// use dynamic_cli::executor::{AsyncCommandHandler, ParsedArgs};
    /// use async_trait::async_trait;
    ///
    /// let mut registry = CommandRegistry::new();
    ///
    /// let definition = CommandDefinition {
    ///     name: "fetch".to_string(),
    ///     aliases: vec![],
    ///     description: "Fetch remote data".to_string(),
    ///     required: false,
    ///     arguments: vec![],
    ///     options: vec![],
    ///     implementation: "fetch_handler".to_string(),
    ///     continue_on_failure: false,
    ///     requires_success: false,
    /// };
    ///
    /// struct FetchCommand;
    /// #[async_trait]
    /// impl AsyncCommandHandler for FetchCommand {
    ///     async fn execute(
    ///         &self,
    ///         _: &mut dyn dynamic_cli::context::ExecutionContext,
    ///         _: &ParsedArgs,
    ///     ) -> dynamic_cli::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// registry.register_async(definition, Box::new(FetchCommand))?;
    /// assert!(registry.get_handler_async("fetch").is_some());
    /// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
    /// ```
    pub fn register_async(
        &mut self,
        definition: CommandDefinition,
        handler: Box<dyn AsyncCommandHandler>,
    ) -> Result<()> {
        self.check_name_available(&definition.name)?;
        for alias in &definition.aliases {
            self.check_name_available(alias)?;
        }
        Self::check_fault_tolerance(&definition, handler.expected_fault_tolerance())?;
        self.insert_aliases(definition.clone());
        self.commands.insert(
            definition.name.clone(),
            (definition, StoredHandler::Async(handler)),
        );
        Ok(())
    }

    /// Resolve a name (command or alias) to the canonical command name
    ///
    /// This method checks if the given name is either:
    /// - A registered command name (returns the name itself)
    /// - An alias (returns the canonical command name)
    ///
    /// # Arguments
    ///
    /// * `name` - The name or alias to resolve
    ///
    /// # Returns
    ///
    /// - `Some(&str)` - The canonical command name
    /// - `None` - If the name is not registered
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use std::collections::HashMap;
    ///
    /// let mut registry = CommandRegistry::new();
    ///
    /// # let definition = CommandDefinition {
    /// #     name: "hello".to_string(),
    /// #     aliases: vec!["hi".to_string()],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_sync(definition, Box::new(TestCmd)).unwrap();
    /// // Resolve command name
    /// assert_eq!(registry.resolve_name("hello"), Some("hello"));
    ///
    /// // Resolve alias
    /// assert_eq!(registry.resolve_name("hi"), Some("hello"));
    ///
    /// // Unknown name
    /// assert_eq!(registry.resolve_name("unknown"), None);
    /// ```
    pub fn resolve_name(&self, name: &str) -> Option<&str> {
        // First check if it's a command name
        // Return reference to the stored name, not the parameter
        if let Some((cmd_def, _)) = self.commands.get(name) {
            return Some(cmd_def.name.as_str());
        }

        // Then check if it's an alias
        self.aliases.get(name).map(|s| s.as_str())
    }

    /// Get the definition of a command by name or alias
    ///
    /// # Arguments
    ///
    /// * `name` - The command name or alias
    ///
    /// # Returns
    ///
    /// - `Some(&CommandDefinition)` if the command exists
    /// - `None` if the command is not registered
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use std::collections::HashMap;
    /// # let mut registry = CommandRegistry::new();
    /// # let definition = CommandDefinition {
    /// #     name: "test".to_string(),
    /// #     aliases: vec!["t".to_string()],
    /// #     description: "Test command".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_sync(definition, Box::new(TestCmd)).unwrap();
    /// // Get by name
    /// if let Some(def) = registry.get_definition("test") {
    ///     assert_eq!(def.name, "test");
    ///     assert_eq!(def.description, "Test command");
    /// }
    ///
    /// // Get by alias
    /// if let Some(def) = registry.get_definition("t") {
    ///     assert_eq!(def.name, "test");
    /// }
    /// ```
    pub fn get_definition(&self, name: &str) -> Option<&CommandDefinition> {
        let canonical_name = self.resolve_name(name)?;
        self.commands.get(canonical_name).map(|(def, _)| def)
    }

    /// Get the (sync) handler of a command by name or alias
    ///
    /// This is the primary method used during CLI/REPL dispatch to
    /// retrieve the handler that will execute the command. Returns `None`
    /// both when the name isn't registered at all, and when it resolves to
    /// an *async* handler (query [`get_handler_async`][Self::get_handler_async]
    /// instead in that case) — dispatch sites try both in sequence.
    ///
    /// Renamed from `get_handler()` in v0.5.0 for symmetry with
    /// [`get_handler_async`][Self::get_handler_async]. `get_handler()`
    /// remains available as a deprecated alias until v1.0.0 (DD-022).
    ///
    /// # Arguments
    ///
    /// * `name` - The command name or alias
    ///
    /// # Returns
    ///
    /// - `Some(&dyn CommandHandler)` if a sync handler is registered under this name
    /// - `None` if unregistered, or if registered as an async handler
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use std::collections::HashMap;
    /// # let mut registry = CommandRegistry::new();
    /// # let definition = CommandDefinition {
    /// #     name: "exec".to_string(),
    /// #     aliases: vec!["x".to_string()],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct ExecCmd;
    /// # impl CommandHandler for ExecCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_sync(definition, Box::new(ExecCmd)).unwrap();
    /// // Get handler by name
    /// if let Some(handler) = registry.get_handler_sync("exec") {
    ///     // Use handler for execution
    /// }
    ///
    /// // Get handler by alias
    /// if let Some(handler) = registry.get_handler_sync("x") {
    ///     // Same handler
    /// }
    /// ```
    // The return type &dyn CommandHandler is intentional: callers receive a
    // reference to the handler, which preserves the indirection needed for
    // dynamic dispatch without transferring ownership.
    pub fn get_handler_sync(&self, name: &str) -> Option<&dyn CommandHandler> {
        let canonical = self.resolve_name(name)?;
        match &self.commands.get(canonical)?.1 {
            StoredHandler::Sync(h) => Some(h.as_ref()),
            StoredHandler::Async(_) => None,
        }
    }

    /// Deprecated alias for [`get_handler_sync`][Self::get_handler_sync].
    /// Scheduled for removal in v1.0.0.
    #[deprecated(
        since = "0.5.0",
        note = "renamed to `get_handler_sync` for symmetry with `get_handler_async`; \
                will be removed in 1.0.0"
    )]
    pub fn get_handler(&self, name: &str) -> Option<&dyn CommandHandler> {
        self.get_handler_sync(name)
    }

    /// Get the async handler of a command by name or alias (DD-022)
    ///
    /// Additive counterpart of [`get_handler_sync`][Self::get_handler_sync].
    /// Returns `None` both when the name isn't registered at all, and when
    /// it resolves to a *sync* handler.
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::AsyncCommandHandler;
    /// # use std::collections::HashMap;
    /// # use async_trait::async_trait;
    /// # let mut registry = CommandRegistry::new();
    /// # let definition = CommandDefinition {
    /// #     name: "fetch".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct FetchCmd;
    /// # #[async_trait]
    /// # impl AsyncCommandHandler for FetchCmd {
    /// #     async fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_async(definition, Box::new(FetchCmd)).unwrap();
    /// assert!(registry.get_handler_async("fetch").is_some());
    /// assert!(registry.get_handler_sync("fetch").is_none()); // wrong accessor
    /// ```
    pub fn get_handler_async(&self, name: &str) -> Option<&dyn AsyncCommandHandler> {
        let canonical = self.resolve_name(name)?;
        match &self.commands.get(canonical)?.1 {
            StoredHandler::Async(h) => Some(h.as_ref()),
            StoredHandler::Sync(_) => None,
        }
    }

    /// List all registered command definitions
    ///
    /// Returns a vector of references to all command definitions in the registry.
    /// The order is not guaranteed.
    ///
    /// # Returns
    ///
    /// Vector of command definition references
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use std::collections::HashMap;
    /// # let mut registry = CommandRegistry::new();
    /// # let def1 = CommandDefinition {
    /// #     name: "cmd1".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # let def2 = CommandDefinition {
    /// #     name: "cmd2".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_sync(def1, Box::new(TestCmd)).unwrap();
    /// # registry.register_sync(def2, Box::new(TestCmd)).unwrap();
    /// let commands = registry.list_commands();
    /// assert_eq!(commands.len(), 2);
    ///
    /// // Use for help text, command completion, etc.
    /// for cmd in commands {
    ///     println!("{}: {}", cmd.name, cmd.description);
    /// }
    /// ```
    pub fn list_commands(&self) -> Vec<&CommandDefinition> {
        self.commands.values().map(|(def, _)| def).collect()
    }

    /// Get the number of registered commands
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    ///
    /// let registry = CommandRegistry::new();
    /// assert_eq!(registry.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the registry is empty
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::registry::CommandRegistry;
    ///
    /// let registry = CommandRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Check if a command is registered (by name or alias)
    ///
    /// # Example
    ///
    /// ```
    /// # use dynamic_cli::registry::CommandRegistry;
    /// # use dynamic_cli::config::schema::CommandDefinition;
    /// # use dynamic_cli::executor::CommandHandler;
    /// # use std::collections::HashMap;
    /// # let mut registry = CommandRegistry::new();
    /// # let definition = CommandDefinition {
    /// #     name: "test".to_string(),
    /// #     aliases: vec!["t".to_string()],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &dynamic_cli::parser::ParsedArgs) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register_sync(definition, Box::new(TestCmd)).unwrap();
    /// assert!(registry.contains("test"));
    /// assert!(registry.contains("t"));
    /// assert!(!registry.contains("unknown"));
    /// ```
    pub fn contains(&self, name: &str) -> bool {
        self.resolve_name(name).is_some()
    }
}

// Implement Default for convenience
impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ParsedArgs;

    // Test fixtures

    struct TestHandler;

    impl CommandHandler for TestHandler {
        fn execute(
            &self,
            _context: &mut dyn crate::context::ExecutionContext,
            _args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            Ok(())
        }
    }

    struct TestAsyncHandler;

    #[async_trait::async_trait]
    impl AsyncCommandHandler for TestAsyncHandler {
        async fn execute(
            &self,
            _context: &mut dyn crate::context::ExecutionContext,
            _args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            Ok(())
        }
    }

    fn create_test_definition(name: &str, aliases: Vec<&str>) -> CommandDefinition {
        CommandDefinition {
            name: name.to_string(),
            aliases: aliases.iter().map(|s| s.to_string()).collect(),
            description: format!("{} command", name),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: format!("{}_handler", name),
            continue_on_failure: false,
            requires_success: false,
        }
    }

    // Basic functionality tests
    #[test]
    fn test_new_registry_is_empty() {
        let registry = CommandRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.list_commands().len(), 0);
    }

    #[test]
    fn test_register_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        let result = registry.register_sync(definition, Box::new(TestHandler));

        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    /// Deprecated-alias coverage (DD-022 companion issue): `register()` and
    /// `get_handler()` must keep behaving exactly like `register_sync()` /
    /// `get_handler_sync()` until they're removed in v1.0.0. This is the
    /// only place in the crate allowed to call them directly.
    #[test]
    #[allow(deprecated)]
    fn test_deprecated_register_alias_still_works() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("legacy", vec!["old"]);

        let result = registry.register(definition, Box::new(TestHandler));

        assert!(result.is_ok());
        assert!(registry.get_handler("legacy").is_some());
        assert!(registry.get_handler("old").is_some());
        assert_eq!(registry.resolve_name("old"), Some("legacy"));
    }

    #[test]
    fn test_register_command_with_aliases() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi", "greet"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("hello"));
        assert!(registry.contains("hi"));
        assert!(registry.contains("greet"));
    }

    #[test]
    fn test_register_duplicate_command_fails() {
        let mut registry = CommandRegistry::new();
        let def1 = create_test_definition("test", vec![]);
        let def2 = create_test_definition("test", vec![]);

        registry.register_sync(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register_sync(def2, Box::new(TestHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::DuplicateRegistration {
                name,
                ..
            }) => {
                assert_eq!(name, "test");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_register_duplicate_alias_fails() {
        let mut registry = CommandRegistry::new();
        let def1 = create_test_definition("cmd1", vec!["c"]);
        let def2 = create_test_definition("cmd2", vec!["c"]);

        registry.register_sync(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register_sync(def2, Box::new(TestHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::DuplicateAlias {
                alias,
                existing_command,
                ..
            }) => {
                assert_eq!(alias, "c");
                assert_eq!(existing_command, "cmd1");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_alias_conflicts_with_command_name() {
        let mut registry = CommandRegistry::new();
        let def1 = create_test_definition("test", vec![]);
        let def2 = create_test_definition("other", vec!["test"]);

        registry.register_sync(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register_sync(def2, Box::new(TestHandler));

        assert!(result.is_err());
    }

    #[test]
    fn test_command_name_conflicts_with_alias() {
        let mut registry = CommandRegistry::new();
        let def1 = create_test_definition("cmd1", vec!["other"]);
        let def2 = create_test_definition("other", vec![]);

        registry.register_sync(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register_sync(def2, Box::new(TestHandler));

        assert!(result.is_err());
    }

    // Resolve name tests
    #[test]
    fn test_resolve_command_name() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        assert_eq!(registry.resolve_name("test"), Some("test"));
    }

    #[test]
    fn test_resolve_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi", "greet"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        assert_eq!(registry.resolve_name("hi"), Some("hello"));
        assert_eq!(registry.resolve_name("greet"), Some("hello"));
    }

    #[test]
    fn test_resolve_unknown_name() {
        let registry = CommandRegistry::new();
        assert_eq!(registry.resolve_name("unknown"), None);
    }

    // Get definition tests
    #[test]
    fn test_get_definition_by_name() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        let retrieved = registry.get_definition("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_get_definition_by_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        let retrieved = registry.get_definition("hi");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "hello");
    }

    #[test]
    fn test_get_definition_unknown() {
        let registry = CommandRegistry::new();
        assert!(registry.get_definition("unknown").is_none());
    }

    // Get handler tests
    #[test]
    fn test_get_handler_by_name() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        let handler = registry.get_handler_sync("test");
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_handler_sync_by_name() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        let handler = registry.get_handler_sync("test");
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_handler_by_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        let handler = registry.get_handler_sync("hi");
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_handler_unknown() {
        let registry = CommandRegistry::new();
        assert!(registry.get_handler_sync("unknown").is_none());
    }

    // List commands tests
    #[test]
    fn test_list_commands_empty() {
        let registry = CommandRegistry::new();
        let commands = registry.list_commands();
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_list_commands_multiple() {
        let mut registry = CommandRegistry::new();

        registry
            .register_sync(
                create_test_definition("cmd1", vec![]),
                Box::new(TestHandler),
            )
            .unwrap();
        registry
            .register_sync(
                create_test_definition("cmd2", vec![]),
                Box::new(TestHandler),
            )
            .unwrap();
        registry
            .register_sync(
                create_test_definition("cmd3", vec![]),
                Box::new(TestHandler),
            )
            .unwrap();

        let commands = registry.list_commands();
        assert_eq!(commands.len(), 3);

        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"cmd1"));
        assert!(names.contains(&"cmd2"));
        assert!(names.contains(&"cmd3"));
    }

    // Integration tests
    #[test]
    fn test_complete_workflow() {
        let mut registry = CommandRegistry::new();

        // Register multiple commands with aliases
        let def1 = create_test_definition("simulate", vec!["sim", "run"]);
        let def2 = create_test_definition("validate", vec!["val", "check"]);
        let def3 = create_test_definition("help", vec!["h", "?"]);

        registry.register_sync(def1, Box::new(TestHandler)).unwrap();
        registry.register_sync(def2, Box::new(TestHandler)).unwrap();
        registry.register_sync(def3, Box::new(TestHandler)).unwrap();

        // Verify registry state
        assert_eq!(registry.len(), 3);

        // Verify all names resolve correctly
        assert_eq!(registry.resolve_name("simulate"), Some("simulate"));
        assert_eq!(registry.resolve_name("sim"), Some("simulate"));
        assert_eq!(registry.resolve_name("validate"), Some("validate"));
        assert_eq!(registry.resolve_name("val"), Some("validate"));

        // Verify handlers are accessible
        assert!(registry.get_handler_sync("simulate").is_some());
        assert!(registry.get_handler_sync("sim").is_some());
        assert!(registry.get_handler_sync("h").is_some());

        // Verify definitions are accessible
        let sim_def = registry.get_definition("sim");
        assert!(sim_def.is_some());
        assert_eq!(sim_def.unwrap().name, "simulate");
    }

    #[test]
    fn test_default_trait() {
        let registry: CommandRegistry = Default::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_contains_method() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec!["t"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        assert!(registry.contains("test"));
        assert!(registry.contains("t"));
        assert!(!registry.contains("unknown"));
    }

    #[test]
    fn test_multiple_aliases_same_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("command", vec!["c", "cmd", "com"]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        // All aliases should resolve to the same command
        assert_eq!(registry.resolve_name("c"), Some("command"));
        assert_eq!(registry.resolve_name("cmd"), Some("command"));
        assert_eq!(registry.resolve_name("com"), Some("command"));

        // All should return the same handler
        let handler1 = registry.get_handler_sync("c");
        let handler2 = registry.get_handler_sync("cmd");
        assert!(handler1.is_some());
        assert!(handler2.is_some());
    }

    #[test]
    fn test_case_sensitivity() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("Test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        // Case matters
        assert!(registry.contains("Test"));
        assert!(!registry.contains("test"));
        assert!(!registry.contains("TEST"));
    }

    #[test]
    fn test_empty_alias_list() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        let result = registry.register_sync(definition, Box::new(TestHandler));

        assert!(result.is_ok());
        assert!(registry.contains("test"));
    }

    // ============================================================================
    // AsyncCommandHandler / register_async / get_handler_async TESTS (DD-022)
    // ============================================================================

    #[test]
    fn test_register_async_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("fetch", vec![]);

        let result = registry.register_async(definition, Box::new(TestAsyncHandler));

        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_register_async_command_with_aliases() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("fetch", vec!["f", "get-remote"]);

        registry
            .register_async(definition, Box::new(TestAsyncHandler))
            .unwrap();

        assert!(registry.contains("fetch"));
        assert!(registry.contains("f"));
        assert!(registry.contains("get-remote"));
        assert_eq!(registry.resolve_name("f"), Some("fetch"));
    }

    #[test]
    fn test_get_handler_async_by_name_and_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("fetch", vec!["f"]);

        registry
            .register_async(definition, Box::new(TestAsyncHandler))
            .unwrap();

        assert!(registry.get_handler_async("fetch").is_some());
        assert!(registry.get_handler_async("f").is_some());
        assert!(registry.get_handler_async("unknown").is_none());
    }

    /// The core cross-accessor guarantee DD-022 depends on: querying an
    /// async-registered command through the *sync* accessor returns `None`
    /// (not the wrong handler, not a panic) — dispatch sites rely on this
    /// to fall through from `get_handler_sync` to `get_handler_async`.
    #[test]
    fn test_sync_accessor_returns_none_for_async_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("fetch", vec![]);

        registry
            .register_async(definition, Box::new(TestAsyncHandler))
            .unwrap();

        assert!(registry.get_handler_sync("fetch").is_none());
        assert!(registry.get_handler_async("fetch").is_some());
    }

    /// Symmetric case: querying a sync-registered command through the
    /// *async* accessor returns `None`.
    #[test]
    fn test_async_accessor_returns_none_for_sync_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry
            .register_sync(definition, Box::new(TestHandler))
            .unwrap();

        assert!(registry.get_handler_async("test").is_none());
        assert!(registry.get_handler_sync("test").is_some());
    }

    /// A command name already taken by a sync handler must be rejected for
    /// async registration — the unified storage means one name, one kind.
    #[test]
    fn test_register_async_conflicts_with_existing_sync_name() {
        let mut registry = CommandRegistry::new();
        let sync_def = create_test_definition("dual", vec![]);
        let async_def = create_test_definition("dual", vec![]);

        registry
            .register_sync(sync_def, Box::new(TestHandler))
            .unwrap();
        let result = registry.register_async(async_def, Box::new(TestAsyncHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::DuplicateRegistration {
                name,
                ..
            }) => {
                assert_eq!(name, "dual");
            }
            other => panic!("Expected DuplicateRegistration, got: {:?}", other),
        }
    }

    /// Symmetric case: a name already taken by an async handler must be
    /// rejected for sync registration.
    #[test]
    fn test_register_sync_conflicts_with_existing_async_name() {
        let mut registry = CommandRegistry::new();
        let async_def = create_test_definition("dual", vec![]);
        let sync_def = create_test_definition("dual", vec![]);

        registry
            .register_async(async_def, Box::new(TestAsyncHandler))
            .unwrap();
        let result = registry.register_sync(sync_def, Box::new(TestHandler));

        assert!(result.is_err());
    }

    /// An async command's alias must not collide with an existing sync
    /// command's alias, and vice versa — conflict detection is shared
    /// across both kinds via `check_name_available`.
    #[test]
    fn test_async_alias_conflicts_with_sync_alias() {
        let mut registry = CommandRegistry::new();
        let sync_def = create_test_definition("cmd1", vec!["shared"]);
        let async_def = create_test_definition("cmd2", vec!["shared"]);

        registry
            .register_sync(sync_def, Box::new(TestHandler))
            .unwrap();
        let result = registry.register_async(async_def, Box::new(TestAsyncHandler));

        assert!(result.is_err());
    }

    #[test]
    fn test_get_definition_works_for_async_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("fetch", vec!["f"]);

        registry
            .register_async(definition, Box::new(TestAsyncHandler))
            .unwrap();

        let retrieved = registry.get_definition("f");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "fetch");
    }

    #[test]
    fn test_list_commands_includes_both_sync_and_async() {
        let mut registry = CommandRegistry::new();

        registry
            .register_sync(
                create_test_definition("sync-cmd", vec![]),
                Box::new(TestHandler),
            )
            .unwrap();
        registry
            .register_async(
                create_test_definition("async-cmd", vec![]),
                Box::new(TestAsyncHandler),
            )
            .unwrap();

        let commands = registry.list_commands();
        assert_eq!(commands.len(), 2);
        let names: Vec<&str> = commands.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"sync-cmd"));
        assert!(names.contains(&"async-cmd"));
    }

    #[test]
    fn test_mixed_registry_workflow() {
        // End-to-end: a registry with both sync and async commands behaves
        // consistently across resolve_name / get_definition / len / contains.
        let mut registry = CommandRegistry::new();

        registry
            .register_sync(
                create_test_definition("simulate", vec!["sim"]),
                Box::new(TestHandler),
            )
            .unwrap();
        registry
            .register_async(
                create_test_definition("fetch", vec!["f"]),
                Box::new(TestAsyncHandler),
            )
            .unwrap();

        assert_eq!(registry.len(), 2);
        assert!(registry.contains("sim"));
        assert!(registry.contains("f"));

        assert!(registry.get_handler_sync("simulate").is_some());
        assert!(registry.get_handler_async("fetch").is_some());
        assert!(registry.get_handler_sync("fetch").is_none());
        assert!(registry.get_handler_async("simulate").is_none());
    }

    // ========================================================================
    // Fault-tolerance consistency check (DD-028, #72)
    // ========================================================================

    /// Declares its failures must never be silently continued past.
    struct FaultIntolerantHandler;

    impl CommandHandler for FaultIntolerantHandler {
        fn execute(
            &self,
            _context: &mut dyn crate::context::ExecutionContext,
            _args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            Ok(())
        }

        fn expected_fault_tolerance(&self) -> Option<bool> {
            Some(false)
        }
    }

    /// Async mirror of [`FaultIntolerantHandler`].
    struct AsyncFaultIntolerantHandler;

    #[async_trait::async_trait]
    impl AsyncCommandHandler for AsyncFaultIntolerantHandler {
        async fn execute(
            &self,
            _context: &mut dyn crate::context::ExecutionContext,
            _args: &ParsedArgs,
        ) -> crate::error::Result<()> {
            Ok(())
        }

        fn expected_fault_tolerance(&self) -> Option<bool> {
            Some(false)
        }
    }

    #[test]
    fn test_register_sync_accepts_handler_with_no_opinion() {
        // TestHandler never overrides expected_fault_tolerance() — the
        // overwhelmingly common case, and it must keep working exactly as
        // before DD-028, regardless of the configured continue_on_failure.
        let mut registry = CommandRegistry::new();
        let mut definition = create_test_definition("test", vec![]);
        definition.continue_on_failure = true;

        let result = registry.register_sync(definition, Box::new(TestHandler));

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_sync_accepts_matching_expectation() {
        let mut registry = CommandRegistry::new();
        let mut definition = create_test_definition("solve", vec![]);
        definition.continue_on_failure = false; // matches the handler's Some(false)

        let result = registry.register_sync(definition, Box::new(FaultIntolerantHandler));

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_sync_rejects_contradicting_expectation() {
        let mut registry = CommandRegistry::new();
        let mut definition = create_test_definition("solve", vec![]);
        definition.continue_on_failure = true; // contradicts the handler's Some(false)

        let result = registry.register_sync(definition, Box::new(FaultIntolerantHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::FaultToleranceMismatch {
                command,
                expected,
                configured,
                ..
            }) => {
                assert_eq!(command, "solve");
                assert!(!expected);
                assert!(configured);
            }
            other => panic!("wrong error variant: {other:?}"),
        }
    }

    #[test]
    fn test_register_sync_rejected_handler_is_not_stored() {
        // A rejected registration must not leave a partial trace — the
        // command should be registrable again with a corrected definition.
        let mut registry = CommandRegistry::new();
        let mut bad_definition = create_test_definition("solve", vec![]);
        bad_definition.continue_on_failure = true;

        assert!(registry
            .register_sync(bad_definition, Box::new(FaultIntolerantHandler))
            .is_err());
        assert!(!registry.contains("solve"));
        assert_eq!(registry.len(), 0);

        let mut good_definition = create_test_definition("solve", vec![]);
        good_definition.continue_on_failure = false;
        assert!(registry
            .register_sync(good_definition, Box::new(FaultIntolerantHandler))
            .is_ok());
        assert!(registry.contains("solve"));
    }

    #[test]
    fn test_register_async_accepts_handler_with_no_opinion() {
        let mut registry = CommandRegistry::new();
        let mut definition = create_test_definition("fetch", vec![]);
        definition.continue_on_failure = true;

        let result = registry.register_async(definition, Box::new(TestAsyncHandler));

        assert!(result.is_ok());
    }

    #[test]
    fn test_register_async_rejects_contradicting_expectation() {
        let mut registry = CommandRegistry::new();
        let mut definition = create_test_definition("solve", vec![]);
        definition.continue_on_failure = true; // contradicts the handler's Some(false)

        let result = registry.register_async(definition, Box::new(AsyncFaultIntolerantHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::FaultToleranceMismatch {
                command,
                ..
            }) => {
                assert_eq!(command, "solve");
            }
            other => panic!("wrong error variant: {other:?}"),
        }
    }
}
