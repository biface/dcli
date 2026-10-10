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
//! use dynamic_cli::config::schema::{CommandsConfig, Metadata};
//! use dynamic_cli::config::validator::validate_config;
//!
//! # let config = CommandsConfig {
//!       metadata: Metadata {
//!         version: "1.0.0".to_string(),
//!         prompt: "test".to_string(),
//!         prompt_suffix: " >".to_string()
//!         },
//!       commands: vec![],
//!       global_options: vec![],
//!       directives: vec![],
//! };
//! // After loading configuration
//! validate_config(&config)?;
//! # Ok::<(), dynamic_cli::error::DynamicCliError>(())
//! ```

use crate::config::directive::{
    effective_directives, DirectiveDefinition, ReplDirective, DIRECTIVE_PREFIX,
};
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
/// - Command names and aliases starting with `:`, which is reserved for
///   REPL directives
/// - REPL directive overrides (see [`crate::config::directive`]): each
///   directive overridden at most once; names and aliases non-empty,
///   without whitespace or leading `:`; names and aliases unique across
///   the directive table after merging with the defaults
///
/// # Arguments
///
/// * `config` - The configuration to validate
///
/// # Errors
///
/// - [`ConfigError::DuplicateCommand`] if command names/aliases conflict
/// - [`ConfigError::InvalidSchema`] if structural issues are found,
///   including every invalid directive override
/// - [`ConfigError::Inconsistency`] if logical inconsistencies are detected
///
/// # Example
///
/// ```
/// use dynamic_cli::config::schema::{CommandsConfig, Metadata};
/// use dynamic_cli::config::validator::validate_config;
///
/// # let config = CommandsConfig {
///       metadata: Metadata {
///         version: "1.0.0".to_string(),
///         prompt: "test".to_string(),
///         prompt_suffix: " >".to_string()
///         },
///       commands: vec![],
///       global_options: vec![],
///       directives: vec![],
/// };
/// // After loading configuration
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
                suggestion: None,
            }
            .into());
        }

        // Check for duplicate aliases
        for alias in &command.aliases {
            if !seen_names.insert(alias.clone()) {
                return Err(ConfigError::DuplicateCommand {
                    name: alias.clone(),
                    suggestion: None,
                }
                .into());
            }
        }

        // Validate that command has a non-empty name
        if command.name.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Command name cannot be empty".to_string(),
                path: Some(format!("commands[{}].name", idx)),
                suggestion: None,
            }
            .into());
        }

        // DA-030 (#92): the `:` prefix belongs to REPL directives
        if command.name.starts_with(DIRECTIVE_PREFIX) {
            return Err(reserved_prefix_error(
                &command.name,
                format!("commands[{}].name", idx),
            ));
        }
        for (alias_idx, alias) in command.aliases.iter().enumerate() {
            if alias.starts_with(DIRECTIVE_PREFIX) {
                return Err(reserved_prefix_error(
                    alias,
                    format!("commands[{}].aliases[{}]", idx, alias_idx),
                ));
            }
        }

        // Validate that implementation is specified
        if command.implementation.trim().is_empty() {
            return Err(ConfigError::InvalidSchema {
                reason: "Command implementation cannot be empty".to_string(),
                path: Some(format!("commands[{}].implementation", idx)),
                suggestion: None,
            }
            .into());
        }
    }

    // Validate global options
    validate_options(&config.global_options, "global_options")?;

    // Validate REPL directive overrides
    validate_directives(&config.directives)?;

    Ok(())
}

/// Error for a command name or alias using the directive prefix
fn reserved_prefix_error(name: &str, path: String) -> crate::error::DynamicCliError {
    ConfigError::InvalidSchema {
        reason: format!(
            "Command name or alias '{}' starts with '{}', which is reserved for REPL directives",
            name, DIRECTIVE_PREFIX
        ),
        path: Some(path),
        suggestion: Some(format!(
            "Remove the leading '{}' from '{}'.",
            DIRECTIVE_PREFIX, name
        )),
    }
    .into()
}

/// Validate the `directives:` overrides
///
/// Checks, in order:
/// 1. each directive is overridden at most once;
/// 2. every overriding name and alias is non-empty, contains no
///    whitespace and does not start with `:`;
/// 3. names and aliases are unique across the effective directive table
///    (defaults merged with overrides), so an override cannot reuse a name
///    or alias that another directive keeps by default.
///
/// Directive names are not compared with command names: a directive is
/// always typed with its `:` prefix, so the two cannot collide.
fn validate_directives(overrides: &[DirectiveDefinition]) -> Result<()> {
    // DA-030 (#92): overrides are keyed by `implementation`
    let mut overridden: HashMap<ReplDirective, usize> = HashMap::new();
    for (idx, entry) in overrides.iter().enumerate() {
        if let Some(first) = overridden.insert(entry.implementation, idx) {
            return Err(ConfigError::InvalidSchema {
                reason: format!(
                    "Directive '{}' is overridden more than once (also at directives[{}])",
                    entry.implementation.as_str(),
                    first
                ),
                path: Some(format!("directives[{}].implementation", idx)),
                suggestion: Some(format!(
                    "Merge the entries for '{}' into a single one.",
                    entry.implementation.as_str()
                )),
            }
            .into());
        }

        validate_directive_token(&entry.name, format!("directives[{}].name", idx))?;
        for (alias_idx, alias) in entry.aliases.iter().enumerate() {
            validate_directive_token(alias, format!("directives[{}].aliases[{}]", idx, alias_idx))?;
        }
    }

    // Defaults never collide among themselves, so any collision involves at
    // least one override; the reported path points at it.
    let mut owners: HashMap<&str, ReplDirective> = HashMap::new();
    let table = effective_directives(overrides);
    for def in &table {
        let tokens =
            std::iter::once(def.name.as_str()).chain(def.aliases.iter().map(String::as_str));
        for token in tokens {
            if let Some(owner) = owners.insert(token, def.implementation) {
                let culprit = overridden
                    .get(&def.implementation)
                    .or_else(|| overridden.get(&owner))
                    .copied();
                let detail = if owner == def.implementation {
                    format!(
                        "Directive name or alias '{}' is used twice by '{}'",
                        token,
                        owner.as_str()
                    )
                } else {
                    format!(
                        "Directive name or alias '{}' is used by both '{}' and '{}'",
                        token,
                        owner.as_str(),
                        def.implementation.as_str()
                    )
                };
                return Err(ConfigError::InvalidSchema {
                    reason: detail,
                    path: culprit.map(|idx| format!("directives[{}]", idx)),
                    suggestion: Some(
                        "Give each directive its own names; an override also replaces \
                         the default aliases of its directive."
                            .to_string(),
                    ),
                }
                .into());
            }
        }
    }

    Ok(())
}

/// Validate one directive name or alias
fn validate_directive_token(token: &str, path: String) -> Result<()> {
    let reason = if token.trim().is_empty() {
        "Directive name or alias cannot be empty".to_string()
    } else if token.chars().any(char::is_whitespace) {
        format!("Directive name or alias '{}' contains whitespace", token)
    } else if token.starts_with(DIRECTIVE_PREFIX) {
        format!(
            "Directive name or alias '{}' must not start with '{}'; the REPL adds it",
            token, DIRECTIVE_PREFIX
        )
    } else {
        return Ok(());
    };

    Err(ConfigError::InvalidSchema {
        reason,
        path: Some(path),
        suggestion: None,
    }
    .into())
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
///     continue_on_failure: false,
///     requires_success: false,
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
///         secure: false,
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
                suggestion: None,
            }
            .into());
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
                suggestion: None,
            }
            .into());
        }

        if !seen_names.insert(arg.name.clone()) {
            return Err(ConfigError::InvalidSchema {
                reason: format!("Duplicate argument name: '{}'", arg.name),
                path: Some(format!("{}.arguments", context)),
                suggestion: None,
            }
            .into());
        }
    }

    Ok(())
}

/// Validate that validation rules are consistent with argument types
fn validate_argument_validation_rules(args: &[ArgumentDefinition], _context: &str) -> Result<()> {
    for arg in args.iter() {
        for rule in arg.validation.iter() {
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
                            suggestion: None,
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
                            suggestion: None,
                        }
                        .into());
                    }

                    // Validate that min <= max if both are specified
                    if let (Some(min_val), Some(max_val)) = (min, max) {
                        if min_val > max_val {
                            return Err(ConfigError::Inconsistency {
                                details: format!(
                                    "Invalid range for argument '{}': min ({}) > max ({})",
                                    arg.name, min_val, max_val
                                ),
                                suggestion: None,
                            }
                            .into());
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
                suggestion: None,
            }
            .into());
        }

        // Check for duplicate names
        if !seen_names.insert(opt.name.clone()) {
            return Err(ConfigError::InvalidSchema {
                reason: format!("Duplicate option name: '{}'", opt.name),
                path: Some(format!("{}.options", context)),
                suggestion: None,
            }
            .into());
        }

        // Validate that at least one of short or long is specified
        if opt.short.is_none() && opt.long.is_none() {
            return Err(ConfigError::InvalidSchema {
                reason: format!(
                    "Option '{}' must have at least a short or long form",
                    opt.name
                ),
                path: Some(format!("{}.options[{}]", context, idx)),
                suggestion: None,
            }
            .into());
        }

        // --- DD-024 (#21): repeatable options and their option_parameters shapes ---
        if opt.repeatable {
            // Rule: a repeatable option's absence already means zero
            // occurrences, so a default value would be ambiguous — reject
            // it before the generic default/choices check below, so the
            // repeatable-specific message takes priority.
            if let Some(ref default) = opt.default {
                return Err(ConfigError::Inconsistency {
                    details: format!(
                        "Repeatable option '{}' cannot have a default value ('{}')",
                        opt.name, default
                    ),
                    suggestion: Some(
                        "Remove `default` — a repeatable option's absence already \
                         means zero occurrences, not an implicit one."
                            .to_string(),
                    ),
                }
                .into());
            }

            // Rule: choices doubles as the discriminant list, so it must
            // be non-empty for a repeatable option.
            if opt.choices.is_empty() {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Repeatable option '{}' must declare at least one discriminant in choices",
                        opt.name
                    ),
                    path: Some(format!("{}.options[{}].choices", context, idx)),
                    suggestion: Some(
                        "Add `choices: [...]` listing the valid discriminants for \
                         this repeatable option."
                            .to_string(),
                    ),
                }
                .into());
            }

            // Rule: option_parameters keys must equal choices exactly —
            // no discriminant left undeclared, no orphan key.
            for discriminant in &opt.choices {
                if !opt.option_parameters.contains_key(discriminant) {
                    return Err(ConfigError::InvalidSchema {
                        reason: format!(
                            "Discriminant '{}' is declared in choices for repeatable \
                             option '{}' but has no matching entry in option_parameters",
                            discriminant, opt.name
                        ),
                        path: Some(format!("{}.options[{}].option_parameters", context, idx)),
                        suggestion: Some(format!(
                            "Add an `option_parameters.{}` entry describing this \
                             discriminant's key=value parameters.",
                            discriminant
                        )),
                    }
                    .into());
                }
            }
            let choices_set: HashSet<&String> = opt.choices.iter().collect();
            for key in opt.option_parameters.keys() {
                if !choices_set.contains(key) {
                    return Err(ConfigError::InvalidSchema {
                        reason: format!(
                            "option_parameters key '{}' on option '{}' is not declared in choices",
                            key, opt.name
                        ),
                        path: Some(format!(
                            "{}.options[{}].option_parameters.{}",
                            context, idx, key
                        )),
                        suggestion: Some(format!(
                            "Add '{}' to choices, or remove this option_parameters entry.",
                            key
                        )),
                    }
                    .into());
                }
            }

            // Rule: each discriminant's key=value shape reuses the
            // existing argument validation — names and types, but
            // explicitly not ordering, which is meaningless for named
            // key=value pairs rather than positional arguments.
            for (discriminant, params) in &opt.option_parameters {
                let sub_context = format!(
                    "{}.options[{}].option_parameters.{}",
                    context, idx, discriminant
                );
                validate_argument_names(params, &sub_context)?;
                validate_argument_types(params)?;
            }
        } else if !opt.option_parameters.is_empty() {
            // Rule: option_parameters is meaningless without repeatable.
            return Err(ConfigError::Inconsistency {
                details: format!(
                    "Option '{}' has option_parameters but repeatable is false",
                    opt.name
                ),
                suggestion: Some(
                    "Set `repeatable: true`, or remove `option_parameters`.".to_string(),
                ),
            }
            .into());
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
                    suggestion: None,
                }
                .into());
            }
        }

        // Validate that boolean options don't have choices
        if opt.option_type == ArgumentType::Bool && !opt.choices.is_empty() {
            return Err(ConfigError::Inconsistency {
                details: format!("Boolean option '{}' cannot have choices", opt.name),
                suggestion: None,
            }
            .into());
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
                    suggestion: None,
                }
                .into());
            }

            if let Some(existing) = seen_short.insert(short.clone(), opt.name.clone()) {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Short option '-{}' is used by both '{}' and '{}'",
                        short, existing, opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                    suggestion: None,
                }
                .into());
            }
        }

        // Check long form
        if let Some(ref long) = opt.long {
            if long.is_empty() {
                return Err(ConfigError::InvalidSchema {
                    reason: format!("Long option for '{}' cannot be empty", opt.name),
                    path: Some(format!("{}.options", context)),
                    suggestion: None,
                }
                .into());
            }

            if let Some(existing) = seen_long.insert(long.clone(), opt.name.clone()) {
                return Err(ConfigError::InvalidSchema {
                    reason: format!(
                        "Long option '--{}' is used by both '{}' and '{}'",
                        long, existing, opt.name
                    ),
                    path: Some(format!("{}.options", context)),
                    suggestion: None,
                }
                .into());
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
                reason: format!("Option '{}' has the same name as an argument", opt.name),
                path: Some(format!("{}.options", context)),
                suggestion: None,
            }
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::CommandsConfig;
    use std::collections::HashMap;

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
                continue_on_failure: false,
                requires_success: false,
            },
            CommandDefinition {
                name: "test".to_string(), // Duplicate!
                aliases: vec![],
                description: "Test 2".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler2".to_string(),
                continue_on_failure: false,
                requires_success: false,
            },
        ];

        let result = validate_config(&config);
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Config(ConfigError::DuplicateCommand {
                name, ..
            }) => {
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
                continue_on_failure: false,
                requires_success: false,
            },
            CommandDefinition {
                name: "cmd2".to_string(),
                aliases: vec!["c".to_string()], // Duplicate alias!
                description: "Command 2".to_string(),
                required: false,
                arguments: vec![],
                options: vec![],
                implementation: "handler2".to_string(),
                continue_on_failure: false,
                requires_success: false,
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
            continue_on_failure: false,
            requires_success: false,
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
                secure: false,
            },
            ArgumentDefinition {
                name: "required".to_string(),
                arg_type: ArgumentType::String,
                required: true, // Required after optional!
                description: "Required".to_string(),
                validation: vec![],
                secure: false,
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
                secure: false,
            },
            ArgumentDefinition {
                name: "arg1".to_string(), // Duplicate!
                arg_type: ArgumentType::Integer,
                required: true,
                description: "Arg 1 again".to_string(),
                validation: vec![],
                secure: false,
            },
        ];

        let result = validate_argument_names(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_validation_rules_type_mismatch() {
        let args = vec![ArgumentDefinition {
            name: "count".to_string(),
            arg_type: ArgumentType::Integer,
            required: true,
            description: "Count".to_string(),
            validation: vec![
                ValidationRule::MustExist { must_exist: true }, // Wrong for integer!
            ],
            secure: false,
        }];

        let result = validate_argument_validation_rules(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_validation_rules_invalid_range() {
        let args = vec![ArgumentDefinition {
            name: "percentage".to_string(),
            arg_type: ArgumentType::Float,
            required: true,
            description: "Percentage".to_string(),
            validation: vec![ValidationRule::Range {
                min: Some(100.0),
                max: Some(0.0), // min > max!
            }],
            secure: false,
        }];

        let result = validate_argument_validation_rules(&args, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_options_no_flags() {
        let options = vec![OptionDefinition {
            name: "opt1".to_string(),
            short: None,
            long: None, // Neither short nor long!
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Option".to_string(),
            choices: vec![],
            repeatable: false,
            option_parameters: HashMap::new(),
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_options_default_not_in_choices() {
        let options = vec![OptionDefinition {
            name: "mode".to_string(),
            short: Some("m".to_string()),
            long: Some("mode".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: Some("invalid".to_string()), // Not in choices!
            description: "Mode".to_string(),
            choices: vec!["fast".to_string(), "slow".to_string()],
            repeatable: false,
            option_parameters: HashMap::new(),
        }];

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
                repeatable: false,
                option_parameters: HashMap::new(),
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
                repeatable: false,
                option_parameters: HashMap::new(),
            },
        ];

        let result = validate_option_flags(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_option_flags_invalid_short() {
        let options = vec![OptionDefinition {
            name: "opt1".to_string(),
            short: Some("opt".to_string()), // Too long!
            long: None,
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Option".to_string(),
            choices: vec![],
            repeatable: false,
            option_parameters: HashMap::new(),
        }];

        let result = validate_option_flags(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_name_conflicts() {
        let args = vec![ArgumentDefinition {
            name: "output".to_string(),
            arg_type: ArgumentType::Path,
            required: true,
            description: "Output".to_string(),
            validation: vec![],
            secure: false,
        }];

        let options = vec![OptionDefinition {
            name: "output".to_string(), // Same name as argument!
            short: Some("o".to_string()),
            long: Some("output".to_string()),
            option_type: ArgumentType::Path,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec![],
            repeatable: false,
            option_parameters: HashMap::new(),
        }];

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
            arguments: vec![ArgumentDefinition {
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
                secure: false,
            }],
            options: vec![OptionDefinition {
                name: "output".to_string(),
                short: Some("o".to_string()),
                long: Some("output".to_string()),
                option_type: ArgumentType::Path,
                required: false,
                default: Some("out.csv".to_string()),
                description: "Output file".to_string(),
                choices: vec![],
                repeatable: false,
                option_parameters: HashMap::new(),
            }],
            implementation: "process_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        };

        assert!(validate_command(&cmd).is_ok());
    }

    #[test]
    fn test_validate_boolean_with_choices() {
        let options = vec![OptionDefinition {
            name: "flag".to_string(),
            short: Some("f".to_string()),
            long: Some("flag".to_string()),
            option_type: ArgumentType::Bool,
            required: false,
            default: None,
            description: "A flag".to_string(),
            choices: vec!["true".to_string(), "false".to_string()], // Boolean can't have choices!
            repeatable: false,
            option_parameters: HashMap::new(),
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    // ── DD-024 (#21): repeatable options / option_parameters ────────────────

    #[test]
    fn test_validate_repeatable_requires_non_empty_choices() {
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec![], // Repeatable but no discriminants!
            repeatable: true,
            option_parameters: HashMap::new(),
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_repeatable_missing_option_parameters_entry() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        // "plot" is in choices but has no option_parameters entry.
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec!["csv".to_string(), "plot".to_string()],
            repeatable: true,
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_repeatable_orphan_option_parameters_key() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        // "json" has an option_parameters entry but is not in choices.
        option_parameters.insert(
            "json".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec!["csv".to_string()],
            repeatable: true,
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_repeatable_option_parameters_reuses_argument_validation() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "".to_string(), // Empty name — invalid per validate_argument_names.
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec!["csv".to_string()],
            repeatable: true,
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_non_repeatable_with_option_parameters_is_error() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Output".to_string(),
            choices: vec!["csv".to_string()],
            repeatable: false, // option_parameters set despite repeatable: false!
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_repeatable_with_default_is_error() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: Some("csv".to_string()), // Forbidden when repeatable: true!
            description: "Output".to_string(),
            choices: vec!["csv".to_string()],
            repeatable: true,
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_repeatable_valid_config_passes() {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![
                ArgumentDefinition {
                    name: "file".to_string(),
                    arg_type: ArgumentType::Path,
                    required: true,
                    description: "Destination CSV file".to_string(),
                    validation: vec![],
                    secure: false,
                },
                ArgumentDefinition {
                    name: "resolution".to_string(),
                    arg_type: ArgumentType::Integer,
                    required: false,
                    description: "Time-step resolution".to_string(),
                    validation: vec![],
                    secure: false,
                },
            ],
        );
        option_parameters.insert(
            "plot".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination image file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );
        let options = vec![OptionDefinition {
            name: "output".to_string(),
            short: None,
            long: Some("output".to_string()),
            option_type: ArgumentType::String,
            required: false,
            default: None,
            description: "Write simulation results in one or more output kinds".to_string(),
            choices: vec!["csv".to_string(), "plot".to_string()],
            repeatable: true,
            option_parameters,
        }];

        let result = validate_options(&options, "test");
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------
    // REPL directives and the reserved `:` prefix
    // ------------------------------------------------------------------

    fn command(name: &str, aliases: &[&str]) -> CommandDefinition {
        CommandDefinition {
            name: name.to_string(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            description: "Test".to_string(),
            required: false,
            arguments: vec![],
            options: vec![],
            implementation: format!("{}_handler", name.trim_start_matches(':')),
            continue_on_failure: false,
            requires_success: false,
        }
    }

    fn directive(
        implementation: ReplDirective,
        name: &str,
        aliases: &[&str],
    ) -> DirectiveDefinition {
        DirectiveDefinition {
            implementation,
            name: name.to_string(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            description: "Custom".to_string(),
        }
    }

    fn config_with_directives(directives: Vec<DirectiveDefinition>) -> CommandsConfig {
        let mut config = CommandsConfig::minimal();
        config.directives = directives;
        config
    }

    /// Unwrap an `InvalidSchema` error and return `(reason, path)`
    fn invalid_schema(result: Result<()>) -> (String, Option<String>) {
        match result {
            Err(crate::error::DynamicCliError::Config(ConfigError::InvalidSchema {
                reason,
                path,
                ..
            })) => (reason, path),
            other => panic!("Expected InvalidSchema error, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_config_from_yaml_without_directives() {
        let yaml = r#"
metadata:
  version: "1.0.0"
  prompt: "test"
commands:
  - name: hello
    description: "Say hello"
    implementation: "hello_handler"
"#;
        let config: CommandsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.directives.is_empty());
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_command_names_may_match_directive_names() {
        // Directives are typed with `:`, so `help` and `quit` stay free for
        // application commands.
        let mut config = CommandsConfig::minimal();
        config.commands = vec![command("help", &["h"]), command("quit", &["q"])];
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_command_name_with_colon_prefix() {
        let mut config = CommandsConfig::minimal();
        config.commands = vec![command(":run", &[])];
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("':run'"), "{}", reason);
        assert!(
            reason.contains("reserved for REPL directives"),
            "{}",
            reason
        );
        assert_eq!(path.as_deref(), Some("commands[0].name"));
    }

    #[test]
    fn test_validate_config_command_alias_with_colon_prefix() {
        let mut config = CommandsConfig::minimal();
        config.commands = vec![command("list", &[]), command("run", &["r", ":r"])];
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("':r'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("commands[1].aliases[1]"));
    }

    #[test]
    fn test_validate_config_command_colon_inside_name_allowed() {
        let mut config = CommandsConfig::minimal();
        config.commands = vec![command("db:migrate", &[])];
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_directives_valid_overrides() {
        let config = config_with_directives(vec![
            directive(ReplDirective::Help, "aide", &["a", "?"]),
            directive(ReplDirective::Quit, "quitter", &["q"]),
        ]);
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_directives_override_may_reuse_its_own_default_alias() {
        // `h` belongs to `repl_help` by default; overriding `repl_help`
        // replaces its aliases, so the override may keep `h`.
        let config = config_with_directives(vec![directive(ReplDirective::Help, "aide", &["h"])]);
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_directives_implementation_overridden_twice() {
        let config = config_with_directives(vec![
            directive(ReplDirective::Quit, "quitter", &[]),
            directive(ReplDirective::Help, "aide", &[]),
            directive(ReplDirective::Quit, "partir", &[]),
        ]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("'repl_quit'"), "{}", reason);
        assert!(reason.contains("directives[0]"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[2].implementation"));
    }

    #[test]
    fn test_validate_directives_empty_name() {
        let config = config_with_directives(vec![directive(ReplDirective::Help, "  ", &[])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("cannot be empty"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].name"));
    }

    #[test]
    fn test_validate_directives_empty_alias() {
        let config = config_with_directives(vec![directive(ReplDirective::Load, "load", &[""])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("cannot be empty"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].aliases[0]"));
    }

    #[test]
    fn test_validate_directives_name_with_whitespace() {
        let config =
            config_with_directives(vec![directive(ReplDirective::Exit, "sortir vite", &[])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("whitespace"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].name"));
    }

    #[test]
    fn test_validate_directives_alias_with_whitespace() {
        let config = config_with_directives(vec![directive(ReplDirective::Exit, "exit", &["e\t"])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("whitespace"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].aliases[0]"));
    }

    #[test]
    fn test_validate_directives_name_with_colon_prefix() {
        let config = config_with_directives(vec![directive(ReplDirective::Quit, ":quit", &[])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("must not start with ':'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].name"));
    }

    #[test]
    fn test_validate_directives_alias_with_colon_prefix() {
        let config = config_with_directives(vec![directive(ReplDirective::Quit, "quit", &[":q"])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("must not start with ':'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0].aliases[0]"));
    }

    #[test]
    fn test_validate_directives_override_name_collides_with_default_alias() {
        // `q` is a default alias of `repl_quit`, which is not overridden.
        let config = config_with_directives(vec![directive(ReplDirective::Exit, "q", &[])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("'q'"), "{}", reason);
        assert!(reason.contains("'repl_quit'"), "{}", reason);
        assert!(reason.contains("'repl_exit'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0]"));
    }

    #[test]
    fn test_validate_directives_override_alias_collides_with_default_name() {
        // The override of `repl_help` (first in `ALL`) takes `load`, the
        // default name of a later directive: the path still points at it.
        let config =
            config_with_directives(vec![directive(ReplDirective::Help, "help", &["load"])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("'load'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0]"));
    }

    #[test]
    fn test_validate_directives_two_overrides_collide() {
        let config = config_with_directives(vec![
            directive(ReplDirective::Quit, "partir", &[]),
            directive(ReplDirective::Exit, "sortir", &["partir"]),
        ]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("'partir'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[1]"));
    }

    #[test]
    fn test_validate_directives_name_repeated_as_own_alias() {
        let config =
            config_with_directives(vec![directive(ReplDirective::Load, "load", &["load"])]);
        let (reason, path) = invalid_schema(validate_config(&config));
        assert!(reason.contains("used twice by 'repl_load'"), "{}", reason);
        assert_eq!(path.as_deref(), Some("directives[0]"));
    }
}
