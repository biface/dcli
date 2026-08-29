//! CLI argument parser
//!
//! This module provides the [`CliParser`] which parses Unix-style command-line
//! arguments into a structured HashMap. It handles:
//! - Positional arguments
//! - Short options (`-v`)
//! - Long options (`--verbose`)
//! - Options with values (`-o file.txt`, `--output=file.txt`)
//! - Type conversion and validation
//!
//! # Example
//!
//! ```
//! use dynamic_cli::parser::cli_parser::CliParser;
//! use dynamic_cli::config::schema::{CommandDefinition, ArgumentDefinition, ArgumentType};
//!
//! let definition = CommandDefinition {
//!     name: "process".to_string(),
//!     aliases: vec![],
//!     description: "Process files".to_string(),
//!     required: false,
//!     arguments: vec![
//!         ArgumentDefinition {
//!             name: "input".to_string(),
//!             arg_type: ArgumentType::Path,
//!             required: true,
//!             description: "Input file".to_string(),
//!             validation: vec![],
//!             secure: false,
//!         }
//!     ],
//!     options: vec![],
//!     implementation: "handler".to_string(),
//!     continue_on_failure: false,
//!     requires_success: false,
//! };
//!
//! let parser = CliParser::new(&definition);
//! let args = vec!["file.txt".to_string()];
//! let parsed = parser.parse(&args).unwrap();
//!
//! assert_eq!(parsed.get("input"), Some(&"file.txt".to_string()));
//! ```

#[allow(unused_imports)]
use crate::config::schema::{ArgumentDefinition, CommandDefinition, OptionDefinition};
use crate::error::{ParseError, Result};
use crate::parser::type_parser;
use std::collections::HashMap;

/// CLI argument parser
///
/// Parses command-line arguments according to a [`CommandDefinition`].
/// The parser handles both positional arguments and named options
/// with type conversion and validation.
///
/// # Lifetime
///
/// The parser holds a reference to a [`CommandDefinition`] and therefore
/// has a lifetime parameter `'a` that must outlive the parser.
///
/// # Example
///
/// ```
/// use dynamic_cli::parser::cli_parser::CliParser;
/// use dynamic_cli::config::schema::{
///     CommandDefinition, OptionDefinition, ArgumentType
/// };
/// use std::collections::HashMap;
///
/// let definition = CommandDefinition {
///     name: "test".to_string(),
///     aliases: vec![],
///     description: "Test command".to_string(),
///     required: false,
///     arguments: vec![],
///     options: vec![
///         OptionDefinition {
///             name: "verbose".to_string(),
///             short: Some("v".to_string()),
///             long: Some("verbose".to_string()),
///             option_type: ArgumentType::Bool,
///             required: false,
///             default: Some("false".to_string()),
///             description: "Verbose output".to_string(),
///             choices: vec![],
///             repeatable: false,
///             option_parameters: HashMap::new(),
///         }
///     ],
///     implementation: "handler".to_string(),
///     continue_on_failure: false,
///     requires_success: false,
/// };
///
/// let parser = CliParser::new(&definition);
/// let args = vec!["-v".to_string()];
/// let parsed = parser.parse(&args).unwrap();
///
/// assert_eq!(parsed.get("verbose"), Some(&"true".to_string()));
/// ```
pub struct CliParser<'a> {
    /// The command definition that specifies expected arguments and options
    definition: &'a CommandDefinition,
}

/// A single occurrence of a repeatable option
///
/// Produced when a `repeatable: true` option is encountered on the
/// command line: `--output csv file=results.csv resolution=100` becomes
/// `OptionOccurrence { discriminant: "csv", params: {"file": "results.csv",
/// "resolution": "100"} }`.
///
/// `params` values are stored as strings after type validation against
/// `option_parameters[discriminant]`, consistent with how scalar option
/// and argument values are stored (see [`ParsedValue::Scalar`]).
#[derive(Debug, Clone, PartialEq)]
pub struct OptionOccurrence {
    /// The token immediately following the flag, validated against the
    /// option's `choices`.
    pub discriminant: String,
    /// The `key=value` pairs supplied for this occurrence.
    pub params: HashMap<String, String>,
}

/// The value parsed for a single positional argument or option
///
/// [`CliParser::parse_typed`] returns `HashMap<String, ParsedValue>` so
/// that repeatable options (which may occur zero or more times, each
/// with their own sub-parameters) and plain scalar values can coexist in
/// a single result map. [`CliParser::parse`] remains additive and
/// unaffected — see its docs for how the two relate.
#[derive(Debug, Clone, PartialEq)]
pub enum ParsedValue {
    /// A plain positional argument or non-repeatable option value.
    Scalar(String),
    /// Every occurrence of a repeatable option, in command-line order.
    Repeated(Vec<OptionOccurrence>),
}

impl<'a> CliParser<'a> {
    /// Create a new CLI parser for the given command definition
    ///
    /// # Arguments
    ///
    /// * `definition` - The command definition specifying expected arguments
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::cli_parser::CliParser;
    /// use dynamic_cli::config::schema::CommandDefinition;
    ///
    /// # let definition = CommandDefinition {
    /// #     name: "test".to_string(),
    /// #     aliases: vec![],
    /// #     description: "".to_string(),
    /// #     required: false,
    /// #     arguments: vec![],
    /// #     options: vec![],
    /// #     implementation: "".to_string(),
    /// #     continue_on_failure: false,
    /// #     requires_success: false,
    /// # };
    /// let parser = CliParser::new(&definition);
    /// ```
    pub fn new(definition: &'a CommandDefinition) -> Self {
        Self { definition }
    }

    /// Parse command-line arguments into a HashMap of strings
    ///
    /// Thin, non-breaking wrapper around [`Self::parse_typed`] for callers
    /// that only deal in scalar values. Any [`ParsedValue::Repeated`] entry
    /// (i.e. any `repeatable: true` option) is silently dropped from the
    /// result — no command definition predating DD-024 can have one, so
    /// existing callers see no behaviour change. Once the dispatch layer
    /// is migrated to consume [`crate::parser::ParsedArgs`] directly
    /// (#39, in progress — the type exists but `interface/cli.rs` and
    /// `interface/repl.rs` still call this method, not `parse_typed`),
    /// this method can be removed.
    ///
    /// # Arguments
    ///
    /// * `args` - Slice of argument strings (excluding the command name)
    ///
    /// # Returns
    ///
    /// A HashMap mapping argument/option names to their string values.
    /// All values are stored as strings after type validation.
    ///
    /// # Errors
    ///
    /// - [`ParseError::MissingArgument`] if required arguments are missing
    /// - [`ParseError::MissingOption`] if required options are missing
    /// - [`ParseError::UnknownOption`] if an unrecognized option is provided
    /// - [`ParseError::TypeParseError`] if a value cannot be converted to its expected type
    /// - [`ParseError::TooManyArguments`] if more positional arguments than expected
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::cli_parser::CliParser;
    /// use dynamic_cli::config::schema::{
    ///     CommandDefinition, ArgumentDefinition, ArgumentType
    /// };
    ///
    /// let definition = CommandDefinition {
    ///     name: "greet".to_string(),
    ///     aliases: vec![],
    ///     description: "Greet someone".to_string(),
    ///     required: false,
    ///     arguments: vec![
    ///         ArgumentDefinition {
    ///             name: "name".to_string(),
    ///             arg_type: ArgumentType::String,
    ///             required: true,
    ///             description: "Name".to_string(),
    ///             validation: vec![],
    ///             secure: false,
    ///         }
    ///     ],
    ///     options: vec![],
    ///     implementation: "handler".to_string(),
    ///     continue_on_failure: false,
    ///     requires_success: false,
    /// };
    ///
    /// let parser = CliParser::new(&definition);
    /// let result = parser.parse(&["Alice".to_string()]).unwrap();
    /// assert_eq!(result.get("name"), Some(&"Alice".to_string()));
    /// ```
    pub fn parse(&self, args: &[String]) -> Result<HashMap<String, String>> {
        let typed = self.parse_typed(args)?;

        Ok(typed
            .into_iter()
            .filter_map(|(name, value)| match value {
                ParsedValue::Scalar(s) => Some((name, s)),
                ParsedValue::Repeated(_) => None,
            })
            .collect())
    }

    /// Parse command-line arguments into a HashMap of [`ParsedValue`]
    ///
    /// Like [`Self::parse`], but preserves repeatable options as
    /// [`ParsedValue::Repeated`] instead of dropping them. This is the
    /// method that actually implements DD-024 parsing; `parse()` is a
    /// filtering wrapper around it.
    ///
    /// # Arguments
    ///
    /// * `args` - Slice of argument strings (excluding the command name)
    ///
    /// # Errors
    ///
    /// In addition to the errors documented on [`Self::parse`]:
    /// - [`ParseError::UnknownDiscriminant`] if the token following a
    ///   repeatable option's flag is not in that option's `choices`
    /// - [`ParseError::UnknownOptionParameter`] if a `key=value` pair uses
    ///   a key not declared in `option_parameters[discriminant]`
    /// - [`ParseError::MissingRequiredOptionParameter`] if a required key
    ///   is absent from an occurrence
    /// - [`ParseError::DuplicateOptionOccurrence`] if the same
    ///   discriminant is supplied twice with identical `key=value` pairs
    pub fn parse_typed(&self, args: &[String]) -> Result<HashMap<String, ParsedValue>> {
        let mut result = HashMap::new();
        let mut positional_index = 0;
        let mut i = 0;

        // Parse arguments
        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with("--") {
                // Long option
                self.parse_long_option(arg, args, &mut i, &mut result)?;
            } else if arg.starts_with('-') && arg.len() > 1 {
                // Short option (ensure it's not just a negative number)
                if arg
                    .chars()
                    .nth(1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    // This is a negative number, treat as positional
                    self.parse_positional_argument(arg, positional_index, &mut result)?;
                    positional_index += 1;
                } else {
                    self.parse_short_option(arg, args, &mut i, &mut result)?;
                }
            } else {
                // Positional argument
                self.parse_positional_argument(arg, positional_index, &mut result)?;
                positional_index += 1;
            }

            i += 1;
        }

        // Apply defaults for missing optional options
        self.apply_defaults(&mut result)?;

        // Validate all required arguments are present
        self.validate_required_arguments(&result)?;
        self.validate_required_options(&result)?;

        Ok(result)
    }

    /// Parse command-line arguments, stopping cleanly at a segment boundary
    /// instead of erroring on positional-arity overflow (DD-026, #52).
    ///
    /// Shares [`Self::parse_typed`]'s token loop and every option-parsing
    /// helper it calls ([`Self::parse_long_option`], [`Self::parse_short_option`],
    /// [`Self::parse_repeatable_occurrence`]) unchanged. The only
    /// difference is what happens when a bare (non-flag) token is reached
    /// once `positional_index` has already reached
    /// `self.definition.arguments.len()`: where [`Self::parse_typed`]
    /// calls [`Self::parse_positional_argument`] and gets back
    /// [`crate::error::ParseError::too_many_arguments`], this method stops
    /// the loop immediately instead — without consuming that token,
    /// without erroring — then runs the same finishing steps
    /// (`apply_defaults` / `validate_required_arguments` /
    /// `validate_required_options`) on whatever was accumulated so far.
    ///
    /// [`Self::parse_typed`] itself is not modified by this method's
    /// existence: it keeps calling [`Self::parse_positional_argument`]
    /// directly and erroring immediately on overflow, so [`Self::parse`]
    /// and any existing caller keep today's exact behaviour.
    ///
    /// # Returns
    ///
    /// `(parsed, consumed)`, where `consumed` is the number of tokens of
    /// `args` that belong to this command. When the loop reaches the end
    /// of `args` with no leftover boundary token (the single-command,
    /// non-chained case), `consumed == args.len()` and `parsed` is
    /// identical to what [`Self::parse_typed`] would return for the same
    /// input.
    ///
    /// # Errors
    ///
    /// Same as [`Self::parse_typed`] for every case *other* than
    /// positional-arity overflow, which this method never raises — an
    /// overflowing bare token is reported to the caller via `consumed`
    /// instead, for [`crate::registry::CommandRegistry::resolve_name`] to
    /// resolve as the next chain segment.
    ///
    /// # Example
    ///
    /// ```
    /// use dynamic_cli::parser::cli_parser::{CliParser, ParsedValue};
    /// use dynamic_cli::config::schema::{
    ///     CommandDefinition, ArgumentDefinition, ArgumentType
    /// };
    ///
    /// let definition = CommandDefinition {
    ///     name: "config".to_string(),
    ///     aliases: vec![],
    ///     description: "Configure a source".to_string(),
    ///     required: false,
    ///     arguments: vec![
    ///         ArgumentDefinition {
    ///             name: "source".to_string(),
    ///             arg_type: ArgumentType::Path,
    ///             required: true,
    ///             description: "Source file".to_string(),
    ///             validation: vec![],
    ///             secure: false,
    ///         }
    ///     ],
    ///     options: vec![],
    ///     implementation: "config_handler".to_string(),
    ///     continue_on_failure: false,
    ///     requires_success: false,
    /// };
    ///
    /// let parser = CliParser::new(&definition);
    /// // "solve" is the next chained command's name — arity for "config"
    /// // (one positional) is already satisfied by "model.yml".
    /// let args = vec!["model.yml".to_string(), "solve".to_string()];
    /// let (parsed, consumed) = parser.parse_typed_segment(&args).unwrap();
    ///
    /// assert_eq!(consumed, 1);
    /// assert_eq!(
    ///     parsed.get("source"),
    ///     Some(&ParsedValue::Scalar("model.yml".to_string()))
    /// );
    /// ```
    pub fn parse_typed_segment(
        &self,
        args: &[String],
    ) -> Result<(HashMap<String, ParsedValue>, usize)> {
        let mut result = HashMap::new();
        let mut positional_index = 0;
        let mut i = 0;

        while i < args.len() {
            let arg = &args[i];

            if arg.starts_with("--") {
                // Long option
                self.parse_long_option(arg, args, &mut i, &mut result)?;
            } else if arg.starts_with('-') && arg.len() > 1 {
                // Short option (ensure it's not just a negative number)
                if arg
                    .chars()
                    .nth(1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    // This is a negative number, treat as positional —
                    // subject to the same arity-boundary check below.
                    if positional_index >= self.definition.arguments.len() {
                        break;
                    }
                    self.parse_positional_argument(arg, positional_index, &mut result)?;
                    positional_index += 1;
                } else {
                    self.parse_short_option(arg, args, &mut i, &mut result)?;
                }
            } else {
                // Positional argument, or (arity already exhausted) the
                // segment boundary: stop here, without consuming or
                // erroring, and let the caller resolve it as the next
                // chain segment.
                if positional_index >= self.definition.arguments.len() {
                    break;
                }
                self.parse_positional_argument(arg, positional_index, &mut result)?;
                positional_index += 1;
            }

            i += 1;
        }

        // Apply defaults for missing optional options
        self.apply_defaults(&mut result)?;

        // Validate all required arguments are present
        self.validate_required_arguments(&result)?;
        self.validate_required_options(&result)?;

        Ok((result, i))
    }

    /// Parse a long option (--option or --option=value)
    fn parse_long_option(
        &self,
        arg: &str,
        args: &[String],
        index: &mut usize,
        result: &mut HashMap<String, ParsedValue>,
    ) -> Result<()> {
        let arg_without_dashes = &arg[2..];

        // Check for --option=value format
        if let Some(eq_pos) = arg_without_dashes.find('=') {
            let option_name = &arg_without_dashes[..eq_pos];
            let value = &arg_without_dashes[eq_pos + 1..];

            let option = self.find_option_by_long(option_name)?;
            if option.repeatable {
                // A repeatable option's discriminant/params are never
                // attached via `=` — only the space-separated form is
                // supported (see parse_repeatable_occurrence).
                return Err(ParseError::InvalidSyntax {
                    details: format!(
                        "Option --{} is repeatable and does not support --{}=<value>",
                        option.name, option.name
                    ),
                    hint: Some(format!(
                        "Usage: --{} <{}> [key=value ...]",
                        option.name,
                        option.choices.join("|")
                    )),
                }
                .into());
            }
            let parsed_value = type_parser::parse_value(value, option.option_type)?;
            result.insert(option.name.clone(), ParsedValue::Scalar(parsed_value));
        } else {
            // --option format (value might be next arg)
            let option = self.find_option_by_long(arg_without_dashes)?;

            if option.repeatable {
                self.parse_repeatable_occurrence(option, args, index, result)?;
            } else if matches!(
                option.option_type,
                crate::config::schema::ArgumentType::Bool
            ) {
                result.insert(option.name.clone(), ParsedValue::Scalar("true".to_string()));
            } else {
                // Non-boolean: expect value in next argument
                *index += 1;
                if *index >= args.len() {
                    return Err(ParseError::InvalidSyntax {
                        details: format!(
                            "Option --{} requires a value",
                            option.long.as_ref().unwrap()
                        ),
                        hint: Some(format!(
                            "Usage: --{}=<value> or --{} <value>",
                            option.long.as_ref().unwrap(),
                            option.long.as_ref().unwrap()
                        )),
                    }
                    .into());
                }

                let value = &args[*index];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), ParsedValue::Scalar(parsed_value));
            }
        }

        Ok(())
    }

    /// Parse a short option (-o or -o value)
    fn parse_short_option(
        &self,
        arg: &str,
        args: &[String],
        index: &mut usize,
        result: &mut HashMap<String, ParsedValue>,
    ) -> Result<()> {
        let short_flag = &arg[1..2];
        let option = self.find_option_by_short(short_flag)?;

        if option.repeatable {
            if arg.len() > 2 {
                return Err(ParseError::InvalidSyntax {
                    details: format!(
                        "Option -{} is repeatable and does not support an attached value",
                        short_flag
                    ),
                    hint: Some(format!(
                        "Usage: -{} <{}> [key=value ...]",
                        short_flag,
                        option.choices.join("|")
                    )),
                }
                .into());
            }
            self.parse_repeatable_occurrence(option, args, index, result)?;
        } else if matches!(
            option.option_type,
            crate::config::schema::ArgumentType::Bool
        ) {
            result.insert(option.name.clone(), ParsedValue::Scalar("true".to_string()));
        } else {
            // Check if value is attached (e.g., -ovalue)
            if arg.len() > 2 {
                let value = &arg[2..];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), ParsedValue::Scalar(parsed_value));
            } else {
                // Value is next argument
                *index += 1;
                if *index >= args.len() {
                    return Err(ParseError::InvalidSyntax {
                        details: format!("Option -{} requires a value", short_flag),
                        hint: Some(format!(
                            "Usage: -{}<value> or -{} <value>",
                            short_flag, short_flag
                        )),
                    }
                    .into());
                }

                let value = &args[*index];
                let parsed_value = type_parser::parse_value(value, option.option_type)?;
                result.insert(option.name.clone(), ParsedValue::Scalar(parsed_value));
            }
        }

        Ok(())
    }

    /// Parse a positional argument
    fn parse_positional_argument(
        &self,
        value: &str,
        index: usize,
        result: &mut HashMap<String, ParsedValue>,
    ) -> Result<()> {
        if index >= self.definition.arguments.len() {
            return Err(ParseError::too_many_arguments(
                &self.definition.name,
                self.definition.arguments.len(),
                index + 1,
            )
            .into());
        }

        let arg_def = &self.definition.arguments[index];
        let parsed_value = type_parser::parse_value(value, arg_def.arg_type)?;
        result.insert(arg_def.name.clone(), ParsedValue::Scalar(parsed_value));

        Ok(())
    }

    /// Parse one occurrence of a repeatable option
    ///
    /// On entry, `index` points at the option's flag token. Reads the
    /// discriminant token immediately following it, validates it against
    /// `option.choices`, then consumes `key=value` tokens until the next
    /// flag (any token starting with `-`), a bare token that isn't a
    /// `key=value` pair, or the end of input. A `key=value` pair can never
    /// start with `-` itself, so the flag check is unambiguous; a bare
    /// non-`key=value` token ends the occurrence's span without error,
    /// leaving it for the caller (`parse_typed` / `parse_typed_segment`,
    /// #54) to treat as whatever it actually is — this command's next
    /// positional argument, or, in a chained invocation (DD-026, #52),
    /// the next segment's boundary token.
    ///
    /// On return, `index` points at the last token consumed (the
    /// discriminant if no parameters followed, or the last `key=value`
    /// token), matching the convention already used by
    /// [`Self::parse_long_option`] / [`Self::parse_short_option`] — the
    /// caller's own `i += 1` advances past it.
    fn parse_repeatable_occurrence(
        &self,
        option: &OptionDefinition,
        args: &[String],
        index: &mut usize,
        result: &mut HashMap<String, ParsedValue>,
    ) -> Result<()> {
        // Read the discriminant token.
        *index += 1;
        if *index >= args.len() {
            return Err(ParseError::InvalidSyntax {
                details: format!("Option --{} requires a discriminant", option.name),
                hint: Some(format!(
                    "Usage: --{} <{}> [key=value ...]",
                    option.name,
                    option.choices.join("|")
                )),
            }
            .into());
        }
        let discriminant = args[*index].clone();
        if !option.choices.contains(&discriminant) {
            return Err(ParseError::UnknownDiscriminant {
                option: option.name.clone(),
                value: discriminant,
                valid_choices: option.choices.clone(),
                suggestion: Some(format!(
                    "Run --help {} to see valid --{} kinds.",
                    self.definition.name, option.name
                )),
            }
            .into());
        }

        // Guaranteed present by validate_options() (#36) once
        // repeatable/choices/option_parameters consistency has been
        // validated at config-load time.
        let empty: Vec<ArgumentDefinition> = Vec::new();
        let param_defs = option
            .option_parameters
            .get(&discriminant)
            .unwrap_or(&empty);

        // Consume key=value tokens until the next flag, a non-key=value
        // bare token, or end of input.
        let mut params: HashMap<String, String> = HashMap::new();
        while *index + 1 < args.len() {
            let next = &args[*index + 1];
            if next.starts_with('-') {
                break;
            }

            let eq_pos = match next.find('=') {
                Some(pos) => pos,
                // Not a key=value pair: the occurrence's span ends here
                // (see doc comment above) rather than erroring.
                None => break,
            };
            let key = &next[..eq_pos];
            let value = &next[eq_pos + 1..];

            let arg_def = param_defs.iter().find(|a| a.name == key).ok_or_else(|| {
                ParseError::UnknownOptionParameter {
                    option: option.name.clone(),
                    discriminant: discriminant.clone(),
                    key: key.to_string(),
                    valid_keys: param_defs.iter().map(|a| a.name.clone()).collect(),
                    suggestion: Some(format!(
                        "Run --help {} to see valid keys for --{} {}.",
                        self.definition.name, option.name, discriminant
                    )),
                }
            })?;

            let typed_value = type_parser::parse_value(value, arg_def.arg_type)?;
            params.insert(key.to_string(), typed_value);

            *index += 1;
        }

        // Validate required keys are present.
        for arg_def in param_defs {
            if arg_def.required && !params.contains_key(&arg_def.name) {
                return Err(ParseError::MissingRequiredOptionParameter {
                    option: option.name.clone(),
                    discriminant: discriminant.clone(),
                    key: arg_def.name.clone(),
                    suggestion: Some(format!(
                        "Run --help {} to see required keys for --{} {}.",
                        self.definition.name, option.name, discriminant
                    )),
                }
                .into());
            }
        }

        let occurrence = OptionOccurrence {
            discriminant: discriminant.clone(),
            params,
        };

        match result
            .entry(option.name.clone())
            .or_insert_with(|| ParsedValue::Repeated(Vec::new()))
        {
            ParsedValue::Repeated(occurrences) => {
                if occurrences.contains(&occurrence) {
                    return Err(ParseError::DuplicateOptionOccurrence {
                        option: option.name.clone(),
                        discriminant: occurrence.discriminant.clone(),
                        params: occurrence.params.clone().into_iter().collect(),
                        suggestion: Some(format!(
                            "Remove one of the two identical --{} {} occurrences.",
                            option.name, discriminant
                        )),
                    }
                    .into());
                }
                occurrences.push(occurrence);
            }
            ParsedValue::Scalar(_) => unreachable!(
                "option '{}' marked repeatable cannot already hold a Scalar value",
                option.name
            ),
        }

        Ok(())
    }

    /// Apply default values for options not provided
    fn apply_defaults(&self, result: &mut HashMap<String, ParsedValue>) -> Result<()> {
        for option in &self.definition.options {
            if !result.contains_key(&option.name) {
                if let Some(ref default) = option.default {
                    // Validate the default value
                    let parsed_default = type_parser::parse_value(default, option.option_type)?;
                    result.insert(option.name.clone(), ParsedValue::Scalar(parsed_default));
                }
            }
        }
        Ok(())
    }

    /// Validate that all required arguments are present
    fn validate_required_arguments(&self, result: &HashMap<String, ParsedValue>) -> Result<()> {
        for arg in &self.definition.arguments {
            if arg.required && !result.contains_key(&arg.name) {
                return Err(ParseError::missing_argument(&arg.name, &self.definition.name).into());
            }
        }
        Ok(())
    }

    /// Validate that all required options are present
    fn validate_required_options(&self, result: &HashMap<String, ParsedValue>) -> Result<()> {
        for option in &self.definition.options {
            if option.required && !result.contains_key(&option.name) {
                return Err(ParseError::missing_option(
                    &option
                        .long
                        .clone()
                        .or(option.short.clone())
                        .unwrap_or_default(),
                    &self.definition.name,
                )
                .into());
            }
        }
        Ok(())
    }

    /// Find an option by its long form
    fn find_option_by_long(&self, long: &str) -> Result<&OptionDefinition> {
        self.definition
            .options
            .iter()
            .find(|opt| opt.long.as_deref() == Some(long))
            .ok_or_else(|| {
                let available: Vec<String> = self
                    .definition
                    .options
                    .iter()
                    .filter_map(|o| o.long.clone())
                    .collect();
                ParseError::unknown_option_with_suggestions(
                    &format!("--{}", long),
                    &self.definition.name,
                    &available,
                )
                .into()
            })
    }

    /// Find an option by its short form
    fn find_option_by_short(&self, short: &str) -> Result<&OptionDefinition> {
        self.definition
            .options
            .iter()
            .find(|opt| opt.short.as_deref() == Some(short))
            .ok_or_else(|| {
                let available: Vec<String> = self
                    .definition
                    .options
                    .iter()
                    .filter_map(|o| o.short.clone())
                    .collect();
                ParseError::unknown_option_with_suggestions(
                    &format!("-{}", short),
                    &self.definition.name,
                    &available,
                )
                .into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ArgumentType, OptionDefinition};

    /// Helper to create a test command definition
    fn create_test_definition() -> CommandDefinition {
        CommandDefinition {
            name: "test".to_string(),
            aliases: vec![],
            description: "Test command".to_string(),
            required: false,
            arguments: vec![
                ArgumentDefinition {
                    name: "input".to_string(),
                    arg_type: ArgumentType::Path,
                    required: true,
                    description: "Input file".to_string(),
                    validation: vec![],
                    secure: false,
                },
                ArgumentDefinition {
                    name: "output".to_string(),
                    arg_type: ArgumentType::Path,
                    required: false,
                    description: "Output file".to_string(),
                    validation: vec![],
                    secure: false,
                },
            ],
            options: vec![
                OptionDefinition {
                    name: "verbose".to_string(),
                    short: Some("v".to_string()),
                    long: Some("verbose".to_string()),
                    option_type: ArgumentType::Bool,
                    required: false,
                    default: Some("false".to_string()),
                    description: "Verbose output".to_string(),
                    choices: vec![],
                    repeatable: false,
                    option_parameters: HashMap::new(),
                },
                OptionDefinition {
                    name: "count".to_string(),
                    short: Some("c".to_string()),
                    long: Some("count".to_string()),
                    option_type: ArgumentType::Integer,
                    required: false,
                    default: Some("10".to_string()),
                    description: "Count".to_string(),
                    choices: vec![],
                    repeatable: false,
                    option_parameters: HashMap::new(),
                },
            ],
            implementation: "handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        }
    }

    // ========================================================================
    // Positional arguments tests
    // ========================================================================

    #[test]
    fn test_parse_single_positional_argument() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
    }

    #[test]
    fn test_parse_multiple_positional_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "output.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
    }

    #[test]
    fn test_parse_missing_required_argument() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args: Vec<String> = vec![];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::MissingArgument {
                argument, ..
            }) => {
                assert_eq!(argument, "input");
            }
            other => panic!("Expected MissingArgument error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_too_many_positional_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "extra.txt".to_string(),
        ];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::TooManyArguments { .. }) => {}
            other => panic!("Expected TooManyArguments error, got {:?}", other),
        }
    }

    // ========================================================================
    // Long options tests
    // ========================================================================

    #[test]
    fn test_parse_long_boolean_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--verbose".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_long_option_with_equals() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--count=42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_long_option_with_space() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "--count".to_string(),
            "42".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_unknown_long_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "--unknown".to_string()];
        let result = parser.parse(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::UnknownOption { .. }) => {}
            other => panic!("Expected UnknownOption error, got {:?}", other),
        }
    }

    // ========================================================================
    // Short options tests
    // ========================================================================

    #[test]
    fn test_parse_short_boolean_option() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-v".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_short_option_with_space() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-c".to_string(), "42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_short_option_attached_value() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string(), "-c42".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("count"), Some(&"42".to_string()));
    }

    #[test]
    fn test_parse_negative_number_as_positional() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // -123 should be treated as a positional argument, not an option
        let args = vec!["-123".to_string()];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"-123".to_string()));
    }

    // ========================================================================
    // Default values tests
    // ========================================================================

    #[test]
    fn test_apply_default_values() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["input.txt".to_string()];
        let result = parser.parse(&args).unwrap();

        // Default values should be applied
        assert_eq!(result.get("verbose"), Some(&"false".to_string()));
        assert_eq!(result.get("count"), Some(&"10".to_string()));
    }

    #[test]
    fn test_override_default_values() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "-v".to_string(),
            "-c".to_string(),
            "5".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        // Provided values should override defaults
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"5".to_string()));
    }

    // ========================================================================
    // Type conversion tests
    // ========================================================================

    #[test]
    fn test_type_conversion_error() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // "abc" cannot be parsed as integer
        let args = vec![
            "input.txt".to_string(),
            "--count".to_string(),
            "abc".to_string(),
        ];
        let result = parser.parse(&args);

        assert!(result.is_err());
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_parse_complex_command_line() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "--verbose".to_string(),
            "--count=100".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"100".to_string()));
    }

    #[test]
    fn test_parse_mixed_options_and_arguments() {
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        // Options can be interspersed with positional arguments
        let args = vec![
            "--verbose".to_string(),
            "input.txt".to_string(),
            "-c".to_string(),
            "50".to_string(),
            "output.txt".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("input"), Some(&"input.txt".to_string()));
        assert_eq!(result.get("output"), Some(&"output.txt".to_string()));
        assert_eq!(result.get("verbose"), Some(&"true".to_string()));
        assert_eq!(result.get("count"), Some(&"50".to_string()));
    }

    // ========================================================================
    // DD-024: repeatable options with option_parameters (#38)
    // ========================================================================

    /// Helper: a command with a repeatable `--output` option, mirroring
    /// the chrom-rs motivating example (csv with an optional resolution,
    /// plot with just a file).
    fn create_repeatable_test_definition() -> CommandDefinition {
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

        CommandDefinition {
            name: "export".to_string(),
            aliases: vec![],
            description: "Export simulation results".to_string(),
            required: false,
            arguments: vec![],
            options: vec![OptionDefinition {
                name: "output".to_string(),
                short: None,
                long: Some("output".to_string()),
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Write results in one or more output kinds".to_string(),
                choices: vec!["csv".to_string(), "plot".to_string()],
                repeatable: true,
                option_parameters,
            }],
            implementation: "export_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        }
    }

    #[test]
    fn test_parse_repeatable_option_single_occurrence() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=results.csv".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(occurrences.len(), 1);
                assert_eq!(occurrences[0].discriminant, "csv");
                assert_eq!(
                    occurrences[0].params.get("file"),
                    Some(&"results.csv".to_string())
                );
            }
            other => panic!("Expected Repeated([csv]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_optional_param_can_be_omitted() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=results.csv".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(occurrences[0].params.get("resolution"), None);
            }
            other => panic!("Expected Repeated([csv]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_with_optional_param_provided() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=results.csv".to_string(),
            "resolution=100".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(
                    occurrences[0].params.get("resolution"),
                    Some(&"100".to_string())
                );
            }
            other => panic!("Expected Repeated([csv]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_multiple_discriminants_both_parse() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=results.csv".to_string(),
            "--output".to_string(),
            "plot".to_string(),
            "file=chart.png".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(occurrences.len(), 2);
                assert_eq!(occurrences[0].discriminant, "csv");
                assert_eq!(occurrences[1].discriminant, "plot");
            }
            other => panic!("Expected Repeated([csv, plot]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_same_discriminant_different_params_both_kept() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=a.csv".to_string(),
            "--output".to_string(),
            "csv".to_string(),
            "file=b.csv".to_string(),
            "resolution=50".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(occurrences.len(), 2);
            }
            other => panic!("Expected Repeated([csv, csv]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_duplicate_occurrence_errors() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=a.csv".to_string(),
            "--output".to_string(),
            "csv".to_string(),
            "file=a.csv".to_string(),
        ];
        let result = parser.parse_typed(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::DuplicateOptionOccurrence {
                ..
            }) => {}
            other => panic!("Expected DuplicateOptionOccurrence error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_missing_required_param_errors() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        // "file" is required for the csv discriminant and is not supplied.
        let args = vec!["--output".to_string(), "csv".to_string()];
        let result = parser.parse_typed(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::MissingRequiredOptionParameter {
                key,
                ..
            }) => {
                assert_eq!(key, "file");
            }
            other => panic!(
                "Expected MissingRequiredOptionParameter error, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_parse_repeatable_option_unknown_param_key_errors() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=a.csv".to_string(),
            "compression=gzip".to_string(),
        ];
        let result = parser.parse_typed(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::UnknownOptionParameter {
                key,
                ..
            }) => {
                assert_eq!(key, "compression");
            }
            other => panic!("Expected UnknownOptionParameter error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_repeatable_option_unknown_discriminant_errors() {
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "xml".to_string(),
            "file=a.xml".to_string(),
        ];
        let result = parser.parse_typed(&args);

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::DynamicCliError::Parse(ParseError::UnknownDiscriminant {
                value, ..
            }) => {
                assert_eq!(value, "xml");
            }
            other => panic!("Expected UnknownDiscriminant error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_legacy_drops_repeated_values() {
        // parse() (Option A design: non-breaking wrapper) must keep
        // working for definitions with no repeatable options — and
        // silently drop Repeated entries rather than erroring, since no
        // pre-DD-024 caller can represent them anyway.
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=a.csv".to_string(),
        ];
        let result = parser.parse(&args).unwrap();

        assert_eq!(result.get("output"), None);
    }

    // ========================================================================
    // parse_typed_segment() tests (DD-026, #52 / #54)
    // ========================================================================

    /// Helper: one positional argument (still open arity) *and* a
    /// repeatable option, so a repeatable occurrence's key=value span can
    /// be followed by this same command's own remaining positional value —
    /// the scenario #54's boundary-case acceptance criterion targets.
    fn create_repeatable_and_positional_test_definition() -> CommandDefinition {
        let mut option_parameters = HashMap::new();
        option_parameters.insert(
            "csv".to_string(),
            vec![ArgumentDefinition {
                name: "file".to_string(),
                arg_type: ArgumentType::Path,
                required: true,
                description: "Destination CSV file".to_string(),
                validation: vec![],
                secure: false,
            }],
        );

        CommandDefinition {
            name: "output".to_string(),
            aliases: vec![],
            description: "Configure output and a target label".to_string(),
            required: false,
            arguments: vec![ArgumentDefinition {
                name: "target".to_string(),
                arg_type: ArgumentType::String,
                required: false,
                description: "Target label".to_string(),
                validation: vec![],
                secure: false,
            }],
            options: vec![OptionDefinition {
                name: "format".to_string(),
                short: None,
                long: Some("format".to_string()),
                option_type: ArgumentType::String,
                required: false,
                default: None,
                description: "Output format".to_string(),
                choices: vec!["csv".to_string()],
                repeatable: true,
                option_parameters,
            }],
            implementation: "output_handler".to_string(),
            continue_on_failure: false,
            requires_success: false,
        }
    }

    #[test]
    fn test_parse_typed_segment_stops_at_boundary_with_zero_arity() {
        // create_repeatable_test_definition()'s "export" command has zero
        // positional arguments: the very first bare token is already past
        // arity and must stop the segment without being consumed or
        // erroring.
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["solve".to_string()];
        let (result, consumed) = parser.parse_typed_segment(&args).unwrap();

        assert_eq!(consumed, 0);
        assert!(!result.contains_key("output"));
    }

    #[test]
    fn test_parse_typed_segment_stops_after_repeatable_occurrence_with_zero_arity() {
        // Combines the zero-arity boundary with a completed repeatable
        // occurrence beforehand — mirrors DD-026's own chrom-rs-motivated
        // example (an "output"-shaped command whose last occurrence is
        // immediately followed by the next chained command's name).
        let definition = create_repeatable_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--output".to_string(),
            "csv".to_string(),
            "file=result.csv".to_string(),
            "solve".to_string(),
        ];
        let (result, consumed) = parser.parse_typed_segment(&args).unwrap();

        assert_eq!(consumed, 3, "'solve' at index 3 must not be consumed");
        match result.get("output") {
            Some(ParsedValue::Repeated(occurrences)) => {
                assert_eq!(occurrences.len(), 1);
                assert_eq!(occurrences[0].discriminant, "csv");
            }
            other => panic!("Expected Repeated([csv]), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_typed_segment_resumes_positional_counting_after_repeatable_occurrence() {
        // "target" is this command's own positional argument (arity still
        // open) — it must be read as such, not mistaken for a continuing
        // param of the "csv" occurrence that precedes it.
        let definition = create_repeatable_and_positional_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--format".to_string(),
            "csv".to_string(),
            "file=out.csv".to_string(),
            "primary".to_string(),
        ];
        let (result, consumed) = parser.parse_typed_segment(&args).unwrap();

        assert_eq!(consumed, args.len());
        assert_eq!(
            result.get("target"),
            Some(&ParsedValue::Scalar("primary".to_string()))
        );
    }

    #[test]
    fn test_parse_repeatable_occurrence_bare_token_no_longer_errors() {
        // Companion to the segment test above, exercised directly through
        // parse_typed(): the fix to parse_repeatable_occurrence (needed for
        // parse_typed_segment's boundary detection to work at all) is
        // shared code, so parse_typed benefits from it too. No existing
        // test asserted the old error behaviour for this input, so this is
        // additive, not a break of "byte-for-byte unchanged".
        let definition = create_repeatable_and_positional_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "--format".to_string(),
            "csv".to_string(),
            "file=out.csv".to_string(),
            "primary".to_string(),
        ];
        let result = parser.parse_typed(&args).unwrap();

        assert_eq!(
            result.get("target"),
            Some(&ParsedValue::Scalar("primary".to_string()))
        );
    }

    #[test]
    fn test_parse_typed_segment_negative_number_heuristic_preserved_at_boundary() {
        // create_test_definition() has exactly two positional arguments
        // (input, output). Two negative-number tokens fill both; a third
        // bare token is past arity and must stop the segment, exactly as
        // for any other kind of token.
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec!["-3".to_string(), "-5".to_string(), "solve".to_string()];
        let (result, consumed) = parser.parse_typed_segment(&args).unwrap();

        assert_eq!(consumed, 2, "'solve' at index 2 must not be consumed");
        assert_eq!(
            result.get("input"),
            Some(&ParsedValue::Scalar("-3".to_string()))
        );
        assert_eq!(
            result.get("output"),
            Some(&ParsedValue::Scalar("-5".to_string()))
        );
    }

    #[test]
    fn test_parse_typed_segment_end_of_input_matches_parse_typed() {
        // Single-command, non-chained case: no leftover boundary token.
        // consumed must equal args.len(), and the result must be
        // identical to what parse_typed() returns for the same input.
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "--verbose".to_string(),
        ];

        let (segment_result, consumed) = parser.parse_typed_segment(&args).unwrap();
        let typed_result = parser.parse_typed(&args).unwrap();

        assert_eq!(consumed, args.len());
        assert_eq!(segment_result, typed_result);
    }

    #[test]
    fn test_parse_typed_segment_too_many_arguments_still_reported_by_dispatch_path() {
        // parse_typed_segment() itself never raises too_many_arguments —
        // it stops cleanly instead (DD-026's segmentation phase is
        // responsible for turning a non-resolving leftover token back into
        // that same error). This test only pins down the "never errors on
        // overflow" half: the segment boundary is reported via `consumed`,
        // not via Err(..).
        let definition = create_test_definition();
        let parser = CliParser::new(&definition);

        let args = vec![
            "input.txt".to_string(),
            "output.txt".to_string(),
            "unexpected_extra".to_string(),
        ];
        let (result, consumed) = parser.parse_typed_segment(&args).unwrap();

        assert_eq!(consumed, 2, "'unexpected_extra' must not be consumed");
        assert_eq!(
            result.get("input"),
            Some(&ParsedValue::Scalar("input.txt".to_string()))
        );
        assert_eq!(
            result.get("output"),
            Some(&ParsedValue::Scalar("output.txt".to_string()))
        );
    }
}
