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
//! use dynamic_cli::executor::CommandHandler;
//! use std::collections::HashMap;
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
//! };
//!
//! // Create a handler
//! struct HelloCommand;
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _ctx: &mut dyn dynamic_cli::context::ExecutionContext,
//!         _args: &HashMap<String, String>,
//!     ) -> dynamic_cli::Result<()> {
//!         println!("Hello!");
//!         Ok(())
//!     }
//! }
//!
//! // Register the command
//! registry.register(definition, Box::new(HelloCommand))?;
//!
//! // Retrieve by name
//! assert!(registry.get_handler("hello").is_some());
//!
//! // Retrieve by alias
//! assert_eq!(registry.resolve_name("hi"), Some("hello"));
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::config::schema::CommandDefinition;
use crate::error::{RegistryError, Result};
use crate::executor::CommandHandler;
use std::collections::HashMap;

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
/// # };
/// # struct TestCommand;
/// # impl CommandHandler for TestCommand {
/// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
/// # }
/// registry.register(definition, Box::new(TestCommand))?;
///
/// // Use throughout the application
/// if let Some(handler) = registry.get_handler("test") {
///     // Execute the command
/// }
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub struct CommandRegistry {
    /// Map of command names to their data
    /// Key: canonical command name
    /// Value: (CommandDefinition, Box<dyn CommandHandler>)
    commands: HashMap<String, (CommandDefinition, Box<dyn CommandHandler>)>,

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

    /// Register a command with its handler
    ///
    /// This method registers a command definition along with its handler.
    /// It also registers all aliases for the command.
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
    ///   - A command with the same name is already registered
    ///   - An alias conflicts with an existing command or alias
    ///
    /// # Errors
    ///
    /// - [`RegistryError::DuplicateRegistration`] if the command name already exists
    /// - [`RegistryError::DuplicateAlias`] if an alias is already in use
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
    /// let definition = CommandDefinition {
    ///     name: "simulate".to_string(),
    ///     aliases: vec!["sim".to_string(), "run".to_string()],
    ///     description: "Run simulation".to_string(),
    ///     required: false,
    ///     arguments: vec![],
    ///     options: vec![],
    ///     implementation: "sim_handler".to_string(),
    /// };
    ///
    /// struct SimCommand;
    /// impl CommandHandler for SimCommand {
    ///     fn execute(
    ///         &self,
    ///         _: &mut dyn dynamic_cli::context::ExecutionContext,
    ///         _: &HashMap<String, String>,
    ///     ) -> dynamic_cli::Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// // Register the command
    /// registry.register(definition, Box::new(SimCommand))?;
    ///
    /// // Can now access by name or alias
    /// assert!(registry.get_handler("simulate").is_some());
    /// assert_eq!(registry.resolve_name("sim"), Some("simulate"));
    /// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
    /// ```
    pub fn register(
        &mut self,
        definition: CommandDefinition,
        handler: Box<dyn CommandHandler>,
    ) -> Result<()> {
        let cmd_name = &definition.name;

        // Check if command name is already registered
        if self.commands.contains_key(cmd_name) {
            return Err(RegistryError::DuplicateRegistration {
                name: cmd_name.clone(),
            }
            .into());
        }

        // Check if command name conflicts with existing alias
        if self.aliases.contains_key(cmd_name) {
            let existing_cmd = self.aliases.get(cmd_name).unwrap();
            return Err(RegistryError::DuplicateAlias {
                alias: cmd_name.clone(),
                existing_command: existing_cmd.clone(),
            }
            .into());
        }

        // Check all aliases for conflicts
        for alias in &definition.aliases {
            // Check if alias conflicts with existing command name
            if self.commands.contains_key(alias) {
                return Err(RegistryError::DuplicateAlias {
                    alias: alias.clone(),
                    existing_command: alias.clone(),
                }
                .into());
            }

            // Check if alias conflicts with existing alias
            if self.aliases.contains_key(alias) {
                let existing_cmd = self.aliases.get(alias).unwrap();
                return Err(RegistryError::DuplicateAlias {
                    alias: alias.clone(),
                    existing_command: existing_cmd.clone(),
                }
                .into());
            }
        }

        // Register all aliases
        for alias in &definition.aliases {
            self.aliases.insert(alias.clone(), cmd_name.clone());
        }

        // Register the command
        self.commands.insert(cmd_name.clone(), (definition, handler));

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
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register(definition, Box::new(TestCmd)).unwrap();
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
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register(definition, Box::new(TestCmd)).unwrap();
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

    /// Get the handler of a command by name or alias
    ///
    /// This is the primary method used during command execution to
    /// retrieve the handler that will execute the command.
    ///
    /// # Arguments
    ///
    /// * `name` - The command name or alias
    ///
    /// # Returns
    ///
    /// - `Some(&Box<dyn CommandHandler>)` if the command exists
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
    /// #     name: "exec".to_string(),
    /// #     aliases: vec!["x".to_string()],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// # };
    /// # struct ExecCmd;
    /// # impl CommandHandler for ExecCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register(definition, Box::new(ExecCmd)).unwrap();
    /// // Get handler by name
    /// if let Some(handler) = registry.get_handler("exec") {
    ///     // Use handler for execution
    /// }
    ///
    /// // Get handler by alias
    /// if let Some(handler) = registry.get_handler("x") {
    ///     // Same handler
    /// }
    /// ```
    pub fn get_handler(&self, name: &str) -> Option<&Box<dyn CommandHandler>> {
        let canonical_name = self.resolve_name(name)?;
        self.commands.get(canonical_name).map(|(_, handler)| handler)
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
    /// # };
    /// # let def2 = CommandDefinition {
    /// #     name: "cmd2".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register(def1, Box::new(TestCmd)).unwrap();
    /// # registry.register(def2, Box::new(TestCmd)).unwrap();
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
    /// # };
    /// # struct TestCmd;
    /// # impl CommandHandler for TestCmd {
    /// #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
    /// # }
    /// # registry.register(definition, Box::new(TestCmd)).unwrap();
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
    use std::any::Any;

    // Test fixtures
    #[derive(Default)]
    struct TestContext;

    impl crate::context::ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    struct TestHandler;

    impl CommandHandler for TestHandler {
        fn execute(
            &self,
            _context: &mut dyn crate::context::ExecutionContext,
            _args: &HashMap<String, String>,
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

        let result = registry.register(definition, Box::new(TestHandler));

        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_register_command_with_aliases() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi", "greet"]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

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

        registry.register(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register(def2, Box::new(TestHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::DuplicateRegistration {
                name,
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

        registry.register(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register(def2, Box::new(TestHandler));

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Registry(RegistryError::DuplicateAlias {
                alias,
                existing_command,
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

        registry.register(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register(def2, Box::new(TestHandler));

        assert!(result.is_err());
    }

    #[test]
    fn test_command_name_conflicts_with_alias() {
        let mut registry = CommandRegistry::new();
        let def1 = create_test_definition("cmd1", vec!["other"]);
        let def2 = create_test_definition("other", vec![]);

        registry.register(def1, Box::new(TestHandler)).unwrap();
        let result = registry.register(def2, Box::new(TestHandler));

        assert!(result.is_err());
    }

    // Resolve name tests
    #[test]
    fn test_resolve_command_name() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

        assert_eq!(registry.resolve_name("test"), Some("test"));
    }

    #[test]
    fn test_resolve_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi", "greet"]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

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

        registry.register(definition, Box::new(TestHandler)).unwrap();

        let retrieved = registry.get_definition("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_get_definition_by_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi"]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

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

        registry.register(definition, Box::new(TestHandler)).unwrap();

        let handler = registry.get_handler("test");
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_handler_by_alias() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("hello", vec!["hi"]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

        let handler = registry.get_handler("hi");
        assert!(handler.is_some());
    }

    #[test]
    fn test_get_handler_unknown() {
        let registry = CommandRegistry::new();
        assert!(registry.get_handler("unknown").is_none());
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
            .register(create_test_definition("cmd1", vec![]), Box::new(TestHandler))
            .unwrap();
        registry
            .register(create_test_definition("cmd2", vec![]), Box::new(TestHandler))
            .unwrap();
        registry
            .register(create_test_definition("cmd3", vec![]), Box::new(TestHandler))
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

        registry.register(def1, Box::new(TestHandler)).unwrap();
        registry.register(def2, Box::new(TestHandler)).unwrap();
        registry.register(def3, Box::new(TestHandler)).unwrap();

        // Verify registry state
        assert_eq!(registry.len(), 3);

        // Verify all names resolve correctly
        assert_eq!(registry.resolve_name("simulate"), Some("simulate"));
        assert_eq!(registry.resolve_name("sim"), Some("simulate"));
        assert_eq!(registry.resolve_name("validate"), Some("validate"));
        assert_eq!(registry.resolve_name("val"), Some("validate"));

        // Verify handlers are accessible
        assert!(registry.get_handler("simulate").is_some());
        assert!(registry.get_handler("sim").is_some());
        assert!(registry.get_handler("h").is_some());

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

        registry.register(definition, Box::new(TestHandler)).unwrap();

        assert!(registry.contains("test"));
        assert!(registry.contains("t"));
        assert!(!registry.contains("unknown"));
    }

    #[test]
    fn test_multiple_aliases_same_command() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("command", vec!["c", "cmd", "com"]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

        // All aliases should resolve to the same command
        assert_eq!(registry.resolve_name("c"), Some("command"));
        assert_eq!(registry.resolve_name("cmd"), Some("command"));
        assert_eq!(registry.resolve_name("com"), Some("command"));

        // All should return the same handler
        let handler1 = registry.get_handler("c");
        let handler2 = registry.get_handler("cmd");
        assert!(handler1.is_some());
        assert!(handler2.is_some());
    }

    #[test]
    fn test_case_sensitivity() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("Test", vec![]);

        registry.register(definition, Box::new(TestHandler)).unwrap();

        // Case matters
        assert!(registry.contains("Test"));
        assert!(!registry.contains("test"));
        assert!(!registry.contains("TEST"));
    }

    #[test]
    fn test_empty_alias_list() {
        let mut registry = CommandRegistry::new();
        let definition = create_test_definition("test", vec![]);

        let result = registry.register(definition, Box::new(TestHandler));

        assert!(result.is_ok());
        assert!(registry.contains("test"));
    }
}
