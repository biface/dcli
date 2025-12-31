//! Configuration validation
//!
//! This module validates the consistency and correctness of
//! configuration after it has been loaded and parsed.
//!
//! # Validation Levels
//!
//! 1. **Structural validation** - Ensures required fields are present
//! 2. **Semantic validation** - Checks for logical inconsistencies
//! 3. **Uniqueness validation** - Prevents duplicate names/aliases
//!
//! # Example
//!
//! ```
//! use dynamic_cli::config::{schema::CommandsConfig, validator::validate_config};
//!
//! # let config = CommandsConfig::minimal();
//! // After loading configuration
//! validate_config(&config)?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::config::schema::{
    ArgumentDefinition, ArgumentType, CommandDefinition, CommandsConfig, OptionDefinition,
    ValidationRule,
};
use crate::error::{ConfigError, Result};
use std::collections::{HashMap, HashSet};

/// Validate the entire configuration
///
/// Performs comprehensive validation of the configuration structure,
/// checking for:
/// - Duplicate command names and aliases
/// - Valid argument types
/// - Consistent validation rules
/// - Option/argument naming conflicts
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Errors
///
/// - [`ConfigError::DuplicateCommand`] if command names/aliases conflict
/// - [`ConfigError::InvalidSchema`] if structural issues are found
/// - [`ConfigError::Inconsistency`] if logical inconsistencies are detected
///
/// # Example
///
/// ```
/// use dynamic_cli::config::{schema::CommandsConfig, validator::validate_config};
///
/// # let config = CommandsConfig::minimal();
/// validate_config(&config)?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn validate_config(config: &CommandsConfig) -> Result<()> {
    // Track all command names and aliases to detect duplicates
    let mut seen_names: HashSet<String> = HashSet::new();
    
    for (idx, command) in config.commands.iter().enumerate() {
        // Validate the command itself
        validate_command(command)?;
        
        // Check for duplicate command name
        if !seen_names.insert(command.name.clone()) {
            return Err(ConfigError::DuplicateCommand {
                name: command.name.clone(),
            }.into());
        }
        
        // Check for duplicate aliases
        for alias in &command.aliases {
            if !seen_names.insert(alias.clone()) {
                return Err(ConfigError::DuplicateCommand {
                    name: alias.clone(),
                }.into());
            }
        }
        
        // Validate that command has a non-empty name
        if command.name.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Command name cannot be empty".to_string(),
                path: Some(format!("commands[{}].name", idx)),
            }.into());
        }
        
        // Validate that implementation is specified
        if command.implementation.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Command implementation cannot be empty".to_string(),
                path: Some(format!("commands[{}].implementation", idx)),
            }.into());
        }
    }
    
    // Validate global options
    validate_options(&config.global_options, "global_options")?;
    
    Ok(())
}

/// Validate a single command definition
///
/// Checks:
/// - Argument types are valid
/// - No duplicate argument/option names
/// - Validation rules are consistent with types
/// - Required arguments come before optional ones
///
/// # Arguments
///
/// * `cmd` - The command definition to validate
///
/// # Errors
///
/// - [`ConfigError::InvalidSchema`] for structural issues
/// - [`ConfigError::Inconsistency`] for logical problems
///
/// # Example
///
/// ```
/// use dynamic_cli::config::{
///     schema::{CommandDefinition, ArgumentType},
///     validator::validate_command,
/// };
///
/// let cmd = CommandDefinition {
///     name: "test".to_string(),
///     aliases: vec![],
///     description: "Test command".to_string(),
///     required: false,
///     arguments: vec![],
///     options: vec![],
///     implementation: "test_handler".to_string(),
/// };
///
/// validate_command(&cmd)?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn validate_command(cmd: &CommandDefinition) -> Result<()> {
    // Validate arguments
    validate_argument_types(&cmd.arguments)?;
    validate_argument_ordering(&cmd.arguments, &cmd.name)?;
    validate_argument_names(&cmd.arguments, &cmd.name)?;
    validate_argument_validation_rules(&cmd.arguments, &cmd.name)?;
    
    // Validate options
    validate_options(&cmd.options, &cmd.name)?;
    validate_option_flags(&cmd.options, &cmd.name)?;
    
    // Check for name conflicts between arguments and options
    check_name_conflicts(&cmd.arguments, &cmd.options, &cmd.name)?;
    
    Ok(())
}

/// Validate argument types
///
/// Currently, all [`ArgumentType`] variants are valid, but this function
/// exists for future extensibility and to ensure types are properly defined.
///
/// # Arguments
///
/// * `args` - List of argument definitions to validate
///
/// # Example
///
/// ```
/// use dynamic_cli::config::{
///     schema::{ArgumentDefinition, ArgumentType},
///     validator::validate_argument_types,
/// };
///
/// let args = vec![
///     ArgumentDefinition {
///         name: "count".to_string(),
///         arg_type: ArgumentType::Integer,
///         required: true,
///         description: "Count".to_string(),
///         validation: vec![],
///     }
/// ];
///
/// validate_argument_types(&args)?;
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn validate_argument_types(args: &[ArgumentDefinition]) -> Result<()> {
    // Currently all ArgumentType variants are valid
    // This function exists for future extensibility
    
    for arg in args {
        // Validate that the type is properly defined
        // (In the current implementation, all enum variants are valid)
        let _ = arg.arg_type;
    }
    
    Ok(())
}

/// Validate that required arguments come before optional ones
///
/// This prevents confusing situations where an optional argument
/// appears before a required one in the command line.
///
/// # Arguments
///
/// * `args` - List of argument definitions
/// * `context` - Context string for error messages (command name)
fn validate_argument_ordering(args: &[ArgumentDefinition], context: &str) -> Result<()> {
    let mut seen_optional = false;
    
    for (idx, arg) in args.iter().enumerate() {
        if !arg.required {
            seen_optional = true;
        } else if seen_optional {
            return Err(ConfigError::InvalidSchema {
                reason: format!(
                    "Required argument '{}' cannot come after optional arguments",
                    arg.name
                ),
                path: Some(format!("{}.arguments[{}]", context, idx)),
            }.into());
        }
    }
    
    Ok(())
}

/// Validate that argument names are unique
fn validate_argument_names(args: &[ArgumentDefinition], context: &str) -> Result<()> {
    let mut seen_names: HashSet<String> = HashSet::new();
    
    for (idx, arg) in args.iter().enumerate() {
        if arg.name.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Argument name cannot be empty".to_string(),
                path: Some(format!("{}.arguments[{}]", context, idx)),
            }.into());
        }
        
        if !seen_names.insert(arg.name.clone()) {
            return Err(ConfigError::InvalidSchema {
                reason: format!("Duplicate argument name: '{}'", arg.name),
                path: Some(format!("{}.arguments", context)),
            }.into());
        }
    }
    
    Ok(())
}

/// Validate that validation rules are consistent with argument types
fn validate_argument_validation_rules(
    args: &[ArgumentDefinition],
    _context: &str,
) -> Result<()> {
    for (_idx, arg) in args.iter().enumerate() {
        for (_rule_idx, rule) in arg.validation.iter().enumerate() {
            match rule {
                ValidationRule::MustExist { .. } | ValidationRule::Extensions { .. } => {
                    // These rules only make sense for Path arguments
                    if arg.arg_type != ArgumentType::Path {
                        return Err(ConfigError::Inconsistency {
                            details: format!(
                                "Validation rule 'must_exist' or 'extensions' can only be used with 'path' type, \
                                but argument '{}' has type '{}'",
                                arg.name,
                                arg.arg_type.as_str()
                            ),
                        }.into());
                    }
                }
                ValidationRule::Range { min, max } => {
                    // Range rules only make sense for numeric types
                    if !matches!(arg.arg_type, ArgumentType::Integer | ArgumentType::Float) {
                        return Err(ConfigError::Inconsistency {
                            details: format!(
                                "Validation rule 'range' can only be used with numeric types, \
                                but argument '{}' has type '{}'",
                                arg.name,
                                arg.arg_type.as_str()
                            ),
                        }.into());
                    }
                    
                    // Validate that min <= max if both are specified
                    if let (Some(min_val), Some(max_val)) = (min, max) {
                        if min_val > max_val {
                            return Err(ConfigError::Inconsistency {
                                details: format!(
                                    "Invalid range for argument '{}': min ({}) > max ({})",
                                    arg.name, min_val, max_val
                                ),
                            }.into());
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// Validate option definitions
fn validate_options(options: &[OptionDefinition], context: &str) -> Result<()> {
    let mut seen_names: HashSet<String> = HashSet::new();
    
    for (idx, opt) in options.iter().enumerate() {
        // Validate name is not empty
        if opt.name.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Option name cannot be empty".to_string(),
                path: Some(format!("{}.options[{}]", context, idx)),
            }.into());
        }
        
        // Check for duplicate names
        if !seen_names.insert(opt.name.clone()) {
            return Err(ConfigError::InvalidSchema {
                reason: format!("Duplicate option name: '{}'", opt.name),
                path: Some(format!("{}.options", context)),
            }.into());
        }
        
        // Validate that at least one of short or long is specified
        if opt.short.is_none() && opt.long.is_none() {
            return Err(ConfigError::InvalidSchema {
                reason: format!(
                    "Option '{}' must have at least a short or long form",
                    opt.name
                ),
                path: Some(format!("{}.options[{}]", context, idx)),
            }.into());
        }
        
        // Validate choices are consistent with default
        if let Some(ref default) = opt.default {
            if !opt.choices.is_empty() && !opt.choices.contains(default) {
                return Err(ConfigError::Inconsistency {
                    details: format!(
                        "Default value '{}' for option '{}' is not in choices: [{}]",
                        default,
                        opt.name,
                        opt.choices.join(", ")
                    ),
                }.into());
            }
        }
        
        // Validate that boolean options don't have choices
        if opt.option_type == ArgumentType::Bool && !opt.choices.is_empty() {
            return Err(ConfigError::Inconsistency {
                details: format!(
                    "Boolean option '{}' cannot have choices",
                    opt.name
                ),
            }.into());
        }
    }
    
    Ok(())
}

/// Validate option flags (short and long forms)
fn validate_option_flags(options: &[OptionDefinition], context: &str) -> Result<()> {
    let mut seen_short: HashMap<String, String> = HashMap::new();
    let mut seen_long: HashMap<String, String> = HashMap::new();
    
    for opt in options {
        // Check short form
        if let Some(ref short) = opt.short {
            if short.len() != 1 {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Short option '{}' for '{}' must be a single character",
                        short, opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                }.into());
            }
            
            if let Some(existing) = seen_short.insert(short.clone(), opt.name.clone()) {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Short option '-{}' is used by both '{}' and '{}'",
                        short, existing, opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                }.into());
            }
        }
        
        // Check long form
        if let Some(ref long) = opt.long {
            if long.is_empty() {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Long option for '{}' cannot be empty",
                        opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                }.into());
            }
            
            if let Some(existing) = seen_long.insert(long.clone(), opt.name.clone()) {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Long option '--{}' is used by both '{}' and '{}'",
                        long, existing, opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                }.into());
            }
        }
    }
    
    Ok(())
}

/// Check for name conflicts between arguments and options
fn check_name_conflicts(
    args: &[ArgumentDefinition],
    options: &[OptionDefinition],
    context: &str,
) -> Result<()> {
    let arg_names: HashSet<String> = args.iter().map(|a| a.name.clone()).collect();
    
    for opt in options {
        if arg_names.contains(&opt.name) {
            return Err(ConfigError::InvalidSchema {
                reason: format!(
                    "Option '{}' has the same name as an argument",
                    opt.name
                ),
                path: Some(format!("{}.options", context)),
            }.into());
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::CommandsConfig;

    #[test]
    fn test_validate_config_empty() {
        let config = CommandsConfig::minimal();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_duplicate_command_name() {
        let mut config = CommandsConfig::minimal();
        config.commands = vec![
            CommandDefinition {
                name: "test".to_string(),
                aliases: vec![],
                description: "Test 1".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler1".to_string(),
            },
            CommandDefinition {
                name: "test".to_string(), // Duplicate!
                aliases: vec![],
                description: "Test 2".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler2".to_string(),
            },
        ];
        
        let result = validate_config(&config);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Config(ConfigError::DuplicateCommand { name }) => {
                assert_eq!(name, "test");
            }
            other => panic!("Expected DuplicateCommand error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_config_duplicate_alias() {
        let mut config = CommandsConfig::minimal();
        config.commands = vec![
            CommandDefinition {
                name: "cmd1".to_string(),
                aliases: vec!["c".to_string()],
                description: "Command 1".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler1".to_string(),
            },
            CommandDefinition {
                name: "cmd2".to_string(),
                aliases: vec!["c".to_string()], // Duplicate alias!
                description: "Command 2".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler2".to_string(),
            },
        ];
        
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_empty_name() {
        let cmd = CommandDefinition {
            name: "".to_string(), // Empty name!
            aliases: vec![],
            description: "Test".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: "handler".to_string(),
        };
        
        let mut config = CommandsConfig::minimal();
        config.commands = vec![cmd];
        
        let result = validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_argument_ordering() {
        let args = vec![
            ArgumentDefinition {
                name: "optional".to_string(),
                arg_type: ArgumentType::String,
                required: false,
                description: "Optional".to_string(),
                validation: vec![],
            },
            ArgumentDefinition {
                name: "required".to_string(),
                arg_type: ArgumentType::String,
                required: true, // Required after optional!
                description: "Required".to_string(),
                validation: vec![],
            },
        ];
        
        let result = validate_argument_ordering(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_argument_names_duplicate() {
        let args = vec![
            ArgumentDefinition {
                name: "arg1".to_string(),
                arg_type: ArgumentType::String,
                required: true,
                description: "Arg 1".to_string(),
                validation: vec![],
            },
            ArgumentDefinition {
                name: "arg1".to_string(), // Duplicate!
                arg_type: ArgumentType::Integer,
                required: true,
                description: "Arg 1 again".to_string(),
                validation: vec![],
            },
        ];
        
        let result = validate_argument_names(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_validation_rules_type_mismatch() {
        let args = vec![
            ArgumentDefinition {
                name: "count".to_string(),
                arg_type: ArgumentType::Integer,
                required: true,
                description: "Count".to_string(),
                validation: vec![
                    ValidationRule::MustExist { must_exist: true }, // Wrong for integer!
                ],
            },
        ];
        
        let result = validate_argument_validation_rules(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_validation_rules_invalid_range() {
        let args = vec![
            ArgumentDefinition {
                name: "percentage".to_string(),
                arg_type: ArgumentType::Float,
                required: true,
                description: "Percentage".to_string(),
                validation: vec![
                    ValidationRule::Range {
                        min: Some(100.0),
                        max: Some(0.0), // min > max!
                    },
                ],
            },
        ];
        
        let result = validate_argument_validation_rules(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_options_no_flags() {
        let options = vec![
            OptionDefinition {
                name: "opt1".to_string(),
                short: None,
                long: None, // Neither short nor long!
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Option".to_string(),
                choices: vec![],
            },
        ];
        
        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_options_default_not_in_choices() {
        let options = vec![
            OptionDefinition {
                name: "mode".to_string(),
                short: Some("m".to_string()),
                long: Some("mode".to_string()),
                option_type: ArgumentType::String,
                required: false,
                default: Some("invalid".to_string()), // Not in choices!
                description: "Mode".to_string(),
                choices: vec!["fast".to_string(), "slow".to_string()],
            },
        ];
        
        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_option_flags_duplicate_short() {
        let options = vec![
            OptionDefinition {
                name: "opt1".to_string(),
                short: Some("o".to_string()),
                long: None,
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Option 1".to_string(),
                choices: vec![],
            },
            OptionDefinition {
                name: "opt2".to_string(),
                short: Some("o".to_string()), // Duplicate!
                long: None,
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Option 2".to_string(),
                choices: vec![],
            },
        ];
        
        let result = validate_option_flags(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_option_flags_invalid_short() {
        let options = vec![
            OptionDefinition {
                name: "opt1".to_string(),
                short: Some("opt".to_string()), // Too long!
                long: None,
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Option".to_string(),
                choices: vec![],
            },
        ];
        
        let result = validate_option_flags(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_name_conflicts() {
        let args = vec![
            ArgumentDefinition {
                name: "output".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Output".to_string(),
                validation: vec![],
            },
        ];
        
        let options = vec![
            OptionDefinition {
                name: "output".to_string(), // Same name as argument!
                short: Some("o".to_string()),
                long: Some("output".to_string()),
                option_type: ArgumentType::Path,
                required: false,
                default: None,
                description: "Output".to_string(),
                choices: vec![],
            },
        ];
        
        let result = check_name_conflicts(&args, &options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_valid() {
        let cmd = CommandDefinition {
            name: "process".to_string(),
            aliases: vec!["proc".to_string()],
            description: "Process data".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "input".to_string(),
                    arg_type: ArgumentType::Path,
                    required: true,
                    description: "Input file".to_string(),
                    validation: vec![
                        ValidationRule::MustExist { must_exist: true },
                        ValidationRule::Extensions {
                            extensions: vec!["csv".to_string()],
                        },
                    ],
                },
            ],
            options: vec![
                OptionDefinition {
                    name: "output".to_string(),
                    short: Some("o".to_string()),
                    long: Some("output".to_string()),
                    option_type: ArgumentType::Path,
                    required: false,
                    default: Some("out.csv".to_string()),
                    description: "Output file".to_string(),
                    choices: vec![],
                },
            ],
            implementation: "process_handler".to_string(),
        };
        
        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_validate_boolean_with_choices() {
        let options = vec![
            OptionDefinition {
                name: "flag".to_string(),
                short: Some("f".to_string()),
                long: Some("flag".to_string()),
                option_type: ArgumentType::Bool,
                required: false,
                default: None,
                description: "A flag".to_string(),
                choices: vec!["true".to_string(), "false".to_string()], // Boolean can't have choices!
            },
        ];
        
        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }
}
