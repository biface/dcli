//! Command registry module
//!
//! This module provides the central registry for storing and managing commands.
//! The registry maintains the mapping between command names/aliases and their
//! definitions and handlers.
//!
//! # Architecture
//!
//! The registry serves as a lookup table for the executor:
//!
//! ```text
//! Configuration → Registry → Executor
//!     (YAML)        (Store)    (Execute)
//! ```
//!
//! ## Flow
//!
//! 1. **Initialization**: Commands are registered during application startup
//! 2. **Lookup**: During execution, the executor queries the registry
//! 3. **Dispatch**: The registry returns the appropriate handler
//!
//! # Design Principles
//!
//! ## Separation of Concerns
//!
//! The registry separates:
//! - **Definition** (from config module): What the command accepts
//! - **Implementation** (from executor module): What the command does
//! - **Lookup** (this module): How to find commands
//!
//! ## Efficient Lookup
//!
//! The registry uses HashMaps for O(1) lookup by:
//! - Command name
//! - Command alias
//!
//! This ensures fast command resolution even with many registered commands.
//!
//! ## Validation
//!
//! The registry validates during registration:
//! - No duplicate command names
//! - No duplicate aliases
//! - No conflicts between names and aliases
//!
//! # Quick Start
//!
//! ```
//! use dynamic_cli::registry::CommandRegistry;
//! use dynamic_cli::config::schema::CommandDefinition;
//! use dynamic_cli::executor::CommandHandler;
//! use std::collections::HashMap;
//!
//! // 1. Create a registry
//! let mut registry = CommandRegistry::new();
//!
//! // 2. Define a command
//! let definition = CommandDefinition {
//!     name: "hello".to_string(),
//!     aliases: vec!["hi".to_string()],
//!     description: "Say hello".to_string(),
//!     required: false,
//!     arguments: vec![],
//!     options: vec![],
//!     implementation: "hello_handler".to_string(),
//! };
//!
//! // 3. Create a handler
//! struct HelloCommand;
//! impl CommandHandler for HelloCommand {
//!     fn execute(
//!         &self,
//!         _ctx: &mut dyn dynamic_cli::context::ExecutionContext,
//!         args: &HashMap<String, String>,
//!     ) -> dynamic_cli::Result<()> {
//!         let default_world = "World".to_string();
//!         let name = args.get("name").unwrap_or(&default_world);
//!         println!("Hello, {}!", name);
//!         Ok(())
//!     }
//! }
//!
//! // 4. Register the command
//! registry.register(definition, Box::new(HelloCommand))?;
//!
//! // 5. Use the registry
//! if let Some(handler) = registry.get_handler("hello") {
//!     // Execute the command
//! }
//!
//! // Works with aliases too!
//! if let Some(handler) = registry.get_handler("hi") {
//!     // Same handler
//! }
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! # Examples
//!
//! ## Basic Registration
//!
//! ```
//! use dynamic_cli::registry::CommandRegistry;
//! # use dynamic_cli::config::schema::CommandDefinition;
//! # use dynamic_cli::executor::CommandHandler;
//! # use std::collections::HashMap;
//!
//! let mut registry = CommandRegistry::new();
//!
//! # let definition = CommandDefinition {
//! #     name: "test".to_string(),
//! #     aliases: vec![],
//! #     description: "".to_string(),
//! #     required: false,
//! #     arguments: vec![],
//! #     options: vec![],
//! #     implementation: "".to_string(),
//! # };
//! # struct TestCmd;
//! # impl CommandHandler for TestCmd {
//! #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
//! # }
//! registry.register(definition, Box::new(TestCmd))?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! ## Command with Multiple Aliases
//!
//! ```
//! # use dynamic_cli::registry::CommandRegistry;
//! # use dynamic_cli::config::schema::CommandDefinition;
//! # use dynamic_cli::executor::CommandHandler;
//! # use std::collections::HashMap;
//! # let mut registry = CommandRegistry::new();
//! let definition = CommandDefinition {
//!     name: "simulate".to_string(),
//!     aliases: vec![
//!         "sim".to_string(),
//!         "run".to_string(),
//!         "exec".to_string(),
//!     ],
//!     description: "Run a simulation".to_string(),
//!     required: false,
//!     arguments: vec![],
//!     options: vec![],
//!     implementation: "simulate_handler".to_string(),
//! };
//!
//! # struct SimCmd;
//! # impl CommandHandler for SimCmd {
//! #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
//! # }
//! registry.register(definition, Box::new(SimCmd))?;
//!
//! // All these work:
//! assert!(registry.contains("simulate"));
//! assert!(registry.contains("sim"));
//! assert!(registry.contains("run"));
//! assert!(registry.contains("exec"));
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```
//!
//! ## Listing All Commands
//!
//! ```
//! # use dynamic_cli::registry::CommandRegistry;
//! # use dynamic_cli::config::schema::CommandDefinition;
//! # use dynamic_cli::executor::CommandHandler;
//! # use std::collections::HashMap;
//! # let mut registry = CommandRegistry::new();
//! # let def1 = CommandDefinition {
//! #     name: "cmd1".to_string(),
//! #     aliases: vec![],
//! #     description: "First command".to_string(),
//! #     required: false,
//! #     arguments: vec![],
//! #     options: vec![],
//! #     implementation: "".to_string(),
//! # };
//! # let def2 = CommandDefinition {
//! #     name: "cmd2".to_string(),
//! #     aliases: vec![],
//! #     description: "Second command".to_string(),
//! #     required: false,
//! #     arguments: vec![],
//! #     options: vec![],
//! #     implementation: "".to_string(),
//! # };
//! # struct TestCmd;
//! # impl CommandHandler for TestCmd {
//! #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
//! # }
//! # registry.register(def1, Box::new(TestCmd)).unwrap();
//! # registry.register(def2, Box::new(TestCmd)).unwrap();
//! // Get all commands for help text
//! for cmd in registry.list_commands() {
//!     println!("{}: {}", cmd.name, cmd.description);
//! }
//! ```
//!
//! ## Error Handling
//!
//! ```
//! # use dynamic_cli::registry::CommandRegistry;
//! # use dynamic_cli::config::schema::CommandDefinition;
//! # use dynamic_cli::executor::CommandHandler;
//! # use dynamic_cli::error::{DynamicCliError, RegistryError};
//! # use std::collections::HashMap;
//! # let mut registry = CommandRegistry::new();
//! # let def1 = CommandDefinition {
//! #     name: "test".to_string(),
//! #     aliases: vec![],
//! #     description: "".to_string(),
//! #     required: false,
//! #     arguments: vec![],
//! #     options: vec![],
//! #     implementation: "".to_string(),
//! # };
//! # let def2 = CommandDefinition {
//! #     name: "test".to_string(),
//! #     aliases: vec![],
//! #     description: "".to_string(),
//! #     required: false,
//! #     arguments: vec![],
//! #     options: vec![],
//! #     implementation: "".to_string(),
//! # };
//! # struct TestCmd;
//! # impl CommandHandler for TestCmd {
//! #     fn execute(&self, _: &mut dyn dynamic_cli::context::ExecutionContext, _: &HashMap<String, String>) -> dynamic_cli::Result<()> { Ok(()) }
//! # }
//! # registry.register(def1, Box::new(TestCmd)).unwrap();
//! // Try to register duplicate
//! let result = registry.register(def2, Box::new(TestCmd));
//!
//! match result {
//!     Err(DynamicCliError::Registry(RegistryError::DuplicateRegistration { name })) => {
//!         eprintln!("Command '{}' already registered", name);
//!     }
//!     _ => {}
//! }
//! ```
//!
//! # Integration with Other Modules
//!
//! ## With Config Module
//!
//! The registry stores `CommandDefinition` from the config module:
//!
//! ```ignore
//! use dynamic_cli::config::loader::load_config;
//! use dynamic_cli::registry::CommandRegistry;
//!
//! let config = load_config("commands.yaml")?;
//! let mut registry = CommandRegistry::new();
//!
//! for cmd_def in config.commands {
//!     let handler = create_handler(&cmd_def.implementation);
//!     registry.register(cmd_def, handler)?;
//! }
//! ```
//!
//! ## With Executor Module
//!
//! The executor queries the registry to find handlers:
//!
//! ```ignore
//! use dynamic_cli::registry::CommandRegistry;
//!
//! fn execute_command(
//!     registry: &CommandRegistry,
//!     command_name: &str,
//!     context: &mut dyn ExecutionContext,
//!     args: &HashMap<String, String>,
//! ) -> Result<()> {
//!     let handler = registry.get_handler(command_name)
//!         .ok_or_else(|| anyhow::anyhow!("Unknown command"))?;
//!     
//!     handler.execute(context, args)
//! }
//! ```
//!
//! # Thread Safety
//!
//! The registry is designed for setup-once, use-many pattern:
//!
//! ```ignore
//! use std::sync::Arc;
//!
//! // Setup phase (single-threaded)
//! let mut registry = CommandRegistry::new();
//! // ... register commands ...
//!
//! // Usage phase (can be multi-threaded)
//! let registry = Arc::new(registry);
//! let registry_clone = registry.clone();
//!
//! std::thread::spawn(move || {
//!     // Safe to use in multiple threads
//!     if let Some(handler) = registry_clone.get_handler("test") {
//!         // ...
//!     }
//! });
//! ```

// Public submodule
pub mod command_registry;

// Public re-exports for convenience
pub use command_registry::CommandRegistry;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::CommandDefinition;
    use crate::context::ExecutionContext;
    use crate::executor::CommandHandler;
    use std::any::Any;
    use std::collections::HashMap;

    // Test fixtures
    #[derive(Default)]
    struct TestContext;

    impl ExecutionContext for TestContext {
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
            _context: &mut dyn ExecutionContext,
            _args: &HashMap<String, String>,
        ) -> crate::error::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_module_reexports() {
        // Verify that CommandRegistry is accessible from module root
        let _registry = CommandRegistry::new();
    }

    #[test]
    fn test_complete_workflow_integration() {
        // Test a complete workflow using the public API
        let mut registry = CommandRegistry::new();

        // Create multiple commands
        let simulate_def = CommandDefinition {
            name: "simulate".to_string(),
            aliases: vec!["sim".to_string(), "run".to_string()],
            description: "Run simulation".to_string(),
            required: true,
            arguments: vec![],
            options: vec![],
            implementation: "sim_handler".to_string(),
        };

        let validate_def = CommandDefinition {
            name: "validate".to_string(),
            aliases: vec!["val".to_string()],
            description: "Validate input".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "val_handler".to_string(),
        };

        // Register commands
        registry
            .register(simulate_def, Box::new(TestHandler))
            .unwrap();
        registry
            .register(validate_def, Box::new(TestHandler))
            .unwrap();

        // Verify complete workflow
        assert_eq!(registry.len(), 2);

        // Resolve by name
        assert_eq!(registry.resolve_name("simulate"), Some("simulate"));
        assert_eq!(registry.resolve_name("validate"), Some("validate"));

        // Resolve by alias
        assert_eq!(registry.resolve_name("sim"), Some("simulate"));
        assert_eq!(registry.resolve_name("val"), Some("validate"));

        // Get handlers
        assert!(registry.get_handler("simulate").is_some());
        assert!(registry.get_handler("sim").is_some());
        assert!(registry.get_handler("val").is_some());

        // Get definitions
        let sim_def = registry.get_definition("sim");
        assert!(sim_def.is_some());
        assert_eq!(sim_def.unwrap().name, "simulate");
        assert!(sim_def.unwrap().required);

        // List all commands
        let commands = registry.list_commands();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_use_case_command_executor_pattern() {
        // Simulate how an executor would use the registry
        let mut registry = CommandRegistry::new();

        let def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec!["t".to_string()],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "test_handler".to_string(),
        };

        registry.register(def, Box::new(TestHandler)).unwrap();

        // Executor pattern: resolve name, then get handler
        let user_input = "t"; // User types alias

        if let Some(canonical_name) = registry.resolve_name(user_input) {
            if let Some(handler) = registry.get_handler(canonical_name) {
                let mut context = TestContext;
                let args = HashMap::new();

                // Execute would happen here
                let result = handler.execute(&mut context, &args);
                assert!(result.is_ok());
            }
        }
    }

    #[test]
    fn test_use_case_help_text_generation() {
        // Simulate generating help text from registry
        let mut registry = CommandRegistry::new();

        let def1 = CommandDefinition {
            name: "help".to_string(),
            aliases: vec!["h".to_string(), "?".to_string()],
            description: "Show help information".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "help_handler".to_string(),
        };

        let def2 = CommandDefinition {
            name: "exit".to_string(),
            aliases: vec!["quit".to_string(), "q".to_string()],
            description: "Exit the application".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "exit_handler".to_string(),
        };

        registry.register(def1, Box::new(TestHandler)).unwrap();
        registry.register(def2, Box::new(TestHandler)).unwrap();

        // Generate help text
        let mut help_text = String::from("Available commands:\n");
        for cmd in registry.list_commands() {
            help_text.push_str(&format!("  {} - {}", cmd.name, cmd.description));
            if !cmd.aliases.is_empty() {
                help_text.push_str(&format!(" (aliases: {})", cmd.aliases.join(", ")));
            }
            help_text.push('\n');
        }

        // Verify help text contains expected information
        assert!(help_text.contains("help"));
        assert!(help_text.contains("exit"));
        assert!(help_text.contains("Show help information"));
        assert!(help_text.contains("Exit the application"));
    }

    #[test]
    fn test_use_case_command_autocomplete() {
        // Simulate command autocomplete functionality
        let mut registry = CommandRegistry::new();

        registry
            .register(
                CommandDefinition {
                    name: "simulate".to_string(),
                    aliases: vec![],
                    description: "".to_string(),
                    required: false,
                    arguments: vec![],
                    options: vec![],
                    implementation: "".to_string(),
                },
                Box::new(TestHandler),
            )
            .unwrap();

        registry
            .register(
                CommandDefinition {
                    name: "simulation".to_string(),
                    aliases: vec![],
                    description: "".to_string(),
                    required: false,
                    arguments: vec![],
                    options: vec![],
                    implementation: "".to_string(),
                },
                Box::new(TestHandler),
            )
            .unwrap();

        // User types "sim" - find all commands starting with "sim"
        let prefix = "sim";
        let matches: Vec<&str> = registry
            .list_commands()
            .iter()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .map(|cmd| cmd.name.as_str())
            .collect();

        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&"simulate"));
        assert!(matches.contains(&"simulation"));
    }

    #[test]
    fn test_error_handling_duplicate_detection() {
        let mut registry = CommandRegistry::new();

        let def = CommandDefinition {
            name: "test".to_string(),
            aliases: vec![],
            description: "".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "".to_string(),
        };

        // First registration succeeds
        assert!(registry
            .register(def.clone(), Box::new(TestHandler))
            .is_ok());

        // Second registration fails
        let result = registry.register(def, Box::new(TestHandler));
        assert!(result.is_err());

        // Error type is correct
        match result {
            Err(crate::error::DynamicCliError::Registry(
                crate::error::RegistryError::DuplicateRegistration { name },
            )) => {
                assert_eq!(name, "test");
            }
            _ => panic!("Wrong error type"),
        }
    }
}
