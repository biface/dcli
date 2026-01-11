//! Utility functions for dynamic-cli
//!
//! This module provides common utility functions used across the framework,
//! including type conversion, string validation, path manipulation, and
//! formatting helpers.
//!
//! # Sections
//!
//! 1. **Formatting and Display** - Format lists, tables, sizes, durations
//! 2. **String Validation** - Check and normalize strings
//! 3. **Type Conversion** - Parse values with context
//! 4. **Path Manipulation** - Normalize and check paths
//! 5. **Test Helpers** - Common test utilities

use crate::config::schema::ArgumentType;
use crate::error::{DynamicCliError, ParseError, Result};
use std::time::Duration;

// ============================================================================
// SECTION 1: FORMATTING AND DISPLAY
// ============================================================================

/// Format a list with numbers
///
/// Creates a numbered list with each item on a new line.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::format_numbered_list;
/// let items = vec!["apple", "banana", "cherry"];
/// let formatted = format_numbered_list(&items);
/// assert_eq!(formatted, "  1. apple\n  2. banana\n  3. cherry");
/// ```
pub fn format_numbered_list<T: std::fmt::Display>(items: &[T]) -> String {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| format!("  {}. {}", i + 1, item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a simple table with headers and rows
///
/// Creates a text table with aligned columns.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::format_table;
/// let headers = vec!["Name", "Age"];
/// let rows = vec![
///     vec!["Alice", "30"],
///     vec!["Bob", "25"],
/// ];
/// let table = format_table(&headers, &rows);
/// assert!(table.contains("Name"));
/// assert!(table.contains("Alice"));
/// ```
pub fn format_table(headers: &[&str], rows: &[Vec<&str>]) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&headers.join(" | "));
    output.push('\n');
    output.push_str(&"-".repeat(headers.iter().map(|h| h.len() + 3).sum()));
    output.push('\n');

    // Rows
    for row in rows {
        output.push_str(&row.join(" | "));
        output.push('\n');
    }

    output
}

/// Format bytes as human-readable size
///
/// Converts byte count to KB, MB, GB, etc.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::format_bytes;
/// assert_eq!(format_bytes(0), "0 B");
/// assert_eq!(format_bytes(1024), "1.00 KB");
/// assert_eq!(format_bytes(1_048_576), "1.00 MB");
/// assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Format duration in human-readable form
///
/// Converts duration to readable format (e.g., "1m 30s").
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::format_duration;
/// # use std::time::Duration;
/// assert_eq!(format_duration(Duration::from_secs(0)), "0s");
/// assert_eq!(format_duration(Duration::from_secs(45)), "45s");
/// assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
/// assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
/// ```
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs == 0 {
        return "0s".to_string();
    }

    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 {
        parts.push(format!("{}m", minutes));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{}s", seconds));
    }

    parts.join(" ")
}

// ============================================================================
// SECTION 2: STRING VALIDATION
// ============================================================================

/// Check if string is empty or only whitespace
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::is_blank;
/// assert!(is_blank(""));
/// assert!(is_blank("   "));
/// assert!(is_blank("\t\n"));
/// assert!(!is_blank("hello"));
/// assert!(!is_blank("  hello  "));
/// ```
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Normalize a string (trim and lowercase)
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::normalize;
/// assert_eq!(normalize("  Hello World  "), "hello world");
/// assert_eq!(normalize("UPPERCASE"), "uppercase");
/// ```
pub fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Truncate string to max length with ellipsis
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::truncate;
/// assert_eq!(truncate("Hello World", 8), "Hello...");
/// assert_eq!(truncate("Hi", 10), "Hi");
/// assert_eq!(truncate("Exact", 5), "Exact");
/// ```
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Check if string looks like an email (basic validation)
///
/// This is a simple check, not RFC-compliant.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::is_valid_email;
/// assert!(is_valid_email("user@example.com"));
/// assert!(is_valid_email("name.surname@domain.co.uk"));
/// assert!(!is_valid_email("invalid"));
/// assert!(!is_valid_email("@example.com"));
/// assert!(!is_valid_email("user@"));
/// ```
pub fn is_valid_email(s: &str) -> bool {
    // Basic check: has @, has text before and after @, has . after @
    let parts: Vec<&str> = s.split('@').collect();

    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

// ============================================================================
// SECTION 3: TYPE CONVERSION
// ============================================================================

/// Parse string to integer with context
///
/// Returns a detailed error message on failure.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::parse_int;
/// assert_eq!(parse_int("42", "count").unwrap(), 42);
/// assert_eq!(parse_int("-10", "offset").unwrap(), -10);
/// assert!(parse_int("abc", "count").is_err());
/// ```
pub fn parse_int(value: &str, field_name: &str) -> Result<i64> {
    value.parse::<i64>().map_err(|_| {
        DynamicCliError::Parse(ParseError::TypeParseError {
            arg_name: field_name.to_string(),
            expected_type: "integer".to_string(),
            value: value.to_string(),
            details: Some("must be a valid integer".to_string()),
        })
    })
}

/// Parse string to float with context
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::parse_float;
/// assert_eq!(parse_float("3.14", "pi").unwrap(), 3.14);
/// assert_eq!(parse_float("42", "value").unwrap(), 42.0);
/// assert!(parse_float("abc", "value").is_err());
/// ```
pub fn parse_float(value: &str, field_name: &str) -> Result<f64> {
    value.parse::<f64>().map_err(|_| {
        DynamicCliError::Parse(ParseError::TypeParseError {
            arg_name: field_name.to_string(),
            expected_type: "float".to_string(),
            value: value.to_string(),
            details: Some("must be a valid floating-point number".to_string()),
        })
    })
}

/// Parse string to bool
///
/// Accepts: true/false, yes/no, 1/0, on/off (case-insensitive).
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::parse_bool;
/// assert_eq!(parse_bool("true").unwrap(), true);
/// assert_eq!(parse_bool("YES").unwrap(), true);
/// assert_eq!(parse_bool("1").unwrap(), true);
/// assert_eq!(parse_bool("on").unwrap(), true);
/// assert_eq!(parse_bool("false").unwrap(), false);
/// assert_eq!(parse_bool("no").unwrap(), false);
/// assert_eq!(parse_bool("0").unwrap(), false);
/// assert_eq!(parse_bool("off").unwrap(), false);
/// assert!(parse_bool("maybe").is_err());
/// ```
pub fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Ok(true),
        "false" | "no" | "0" | "off" => Ok(false),
        _ => Err(DynamicCliError::Parse(ParseError::TypeParseError {
            arg_name: "value".to_string(),
            expected_type: "bool".to_string(),
            value: value.to_string(),
            details: Some("must be one of: true, false, yes, no, 1, 0, on, off".to_string()),
        })),
    }
}

/// Detect argument type from string value
///
/// Tries to detect the most appropriate type for a string value.
///
/// # Detection Order
///
/// 1. Bool (true/false/yes/no/1/0/on/off)
/// 2. Integer (parseable as i64)
/// 3. Float (parseable as f64 and contains '.')
/// 4. Path (starts with /, ./, ../, or contains \)
/// 5. String (default)
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::detect_type;
/// # use dynamic_cli::config::schema::ArgumentType;
/// assert_eq!(detect_type("42"), ArgumentType::Integer);
/// assert_eq!(detect_type("3.14"), ArgumentType::Float);
/// assert_eq!(detect_type("true"), ArgumentType::Bool);
/// assert_eq!(detect_type("/path/to/file"), ArgumentType::Path);
/// assert_eq!(detect_type("hello"), ArgumentType::String);
/// ```
pub fn detect_type(value: &str) -> ArgumentType {
    // Try bool
    if parse_bool(value).is_ok() {
        return ArgumentType::Bool;
    }

    // Try integer
    if value.parse::<i64>().is_ok() {
        return ArgumentType::Integer;
    }

    // Try float (must contain '.')
    if value.contains('.') && value.parse::<f64>().is_ok() {
        return ArgumentType::Float;
    }

    // Check if looks like a path
    if value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || value.contains('\\')
    {
        return ArgumentType::Path;
    }

    // Default to string
    ArgumentType::String
}

// ============================================================================
// SECTION 4: PATH MANIPULATION
// ============================================================================

/// Normalize path separators (cross-platform)
///
/// Converts backslashes to forward slashes.
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::normalize_path;
/// assert_eq!(normalize_path("path\\to\\file"), "path/to/file");
/// assert_eq!(normalize_path("path/to/file"), "path/to/file");
/// ```
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Get file extension in lowercase
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::get_extension;
/// assert_eq!(get_extension("file.TXT"), Some("txt".to_string()));
/// assert_eq!(get_extension("data.csv"), Some("csv".to_string()));
/// assert_eq!(get_extension("no_extension"), None);
/// assert_eq!(get_extension(".hidden"), None);
/// ```
pub fn get_extension(path: &str) -> Option<String> {
    let path = std::path::Path::new(path);
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
}

/// Check if path has any of the given extensions
///
/// # Example
///
/// ```
/// # use dynamic_cli::utils::has_extension;
/// assert!(has_extension("data.csv", &["csv", "tsv"]));
/// assert!(has_extension("config.yaml", &["yaml", "yml"]));
/// assert!(!has_extension("data.txt", &["csv", "json"]));
/// ```
pub fn has_extension(path: &str, extensions: &[&str]) -> bool {
    if let Some(ext) = get_extension(path) {
        extensions.iter().any(|&e| e.to_lowercase() == ext)
    } else {
        false
    }
}

// ============================================================================
// SECTION 5: TEST HELPERS
// ============================================================================

#[cfg(test)]
pub mod test_helpers {
    use crate::config::schema::*;
    use crate::context::ExecutionContext;
    use std::any::Any;

    /// Create minimal valid configuration for tests
    pub fn create_test_config(prompt: &str, commands: Vec<&str>) -> CommandsConfig {
        CommandsConfig {
            metadata: Metadata {
                version: "1.0.0".to_string(),
                prompt: prompt.to_string(),
                prompt_suffix: " > ".to_string(),
            },
            commands: commands
                .into_iter()
                .map(|name| create_test_command(name, false))
                .collect(),
            global_options: vec![],
        }
    }

    /// Create simple command definition
    pub fn create_test_command(name: &str, required: bool) -> CommandDefinition {
        CommandDefinition {
            name: name.to_string(),
            aliases: vec![],
            description: format!("Test command: {}", name),
            required,
            arguments: vec![],
            options: vec![],
            implementation: format!("{}_handler", name),
        }
    }

    /// Default test context implementation
    #[derive(Default, Debug)]
    pub struct TestContext {
        pub executed: Vec<String>,
    }

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;

    // ========================================================================
    // SECTION 1: FORMATTING TESTS
    // ========================================================================

    #[test]
    fn test_format_numbered_list_empty() {
        let items: Vec<&str> = vec![];
        assert_eq!(format_numbered_list(&items), "");
    }

    #[test]
    fn test_format_numbered_list_single() {
        let items = vec!["apple"];
        assert_eq!(format_numbered_list(&items), "  1. apple");
    }

    #[test]
    fn test_format_numbered_list_multiple() {
        let items = vec!["apple", "banana", "cherry"];
        let result = format_numbered_list(&items);
        assert!(result.contains("1. apple"));
        assert!(result.contains("2. banana"));
        assert!(result.contains("3. cherry"));
    }

    #[test]
    fn test_format_table_simple() {
        let headers = vec!["Name", "Age"];
        let rows = vec![vec!["Alice", "30"], vec!["Bob", "25"]];
        let table = format_table(&headers, &rows);

        assert!(table.contains("Name"));
        assert!(table.contains("Alice"));
        assert!(table.contains("30"));
    }

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_various_sizes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
    }

    #[test]
    fn test_format_duration_various() {
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    // ========================================================================
    // SECTION 2: VALIDATION TESTS
    // ========================================================================

    #[test]
    fn test_is_blank_various() {
        assert!(is_blank(""));
        assert!(is_blank("   "));
        assert!(is_blank("\t\n"));
        assert!(!is_blank("hello"));
        assert!(!is_blank("  hello  "));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("  Hello World  "), "hello world");
        assert_eq!(normalize("UPPERCASE"), "uppercase");
        assert_eq!(normalize("MixedCase"), "mixedcase");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("Hello World", 8), "Hello...");
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("Hi", 10), "Hi");
        assert_eq!(truncate("Exact", 5), "Exact");
    }

    #[test]
    fn test_is_valid_email_valid() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("name.surname@domain.co.uk"));
    }

    #[test]
    fn test_is_valid_email_invalid() {
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("no-at-sign.com"));
    }

    // ========================================================================
    // SECTION 3: CONVERSION TESTS
    // ========================================================================

    #[test]
    fn test_parse_int_valid() {
        assert_eq!(parse_int("42", "count").unwrap(), 42);
        assert_eq!(parse_int("-10", "offset").unwrap(), -10);
        assert_eq!(parse_int("0", "zero").unwrap(), 0);
    }

    #[test]
    fn test_parse_int_invalid() {
        assert!(parse_int("abc", "count").is_err());
        assert!(parse_int("3.14", "count").is_err());
        assert!(parse_int("", "count").is_err());
    }

    #[test]
    fn test_parse_float_valid() {
        assert_eq!(parse_float("3.14", "pi").unwrap(), 3.14);
        assert_eq!(parse_float("42", "value").unwrap(), 42.0);
        assert_eq!(parse_float("-1.5", "neg").unwrap(), -1.5);
    }

    #[test]
    fn test_parse_bool_various() {
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("YES").unwrap(), true);
        assert_eq!(parse_bool("1").unwrap(), true);
        assert_eq!(parse_bool("on").unwrap(), true);

        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("no").unwrap(), false);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert_eq!(parse_bool("off").unwrap(), false);

        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn test_detect_type_integer() {
        assert_eq!(detect_type("42"), ArgumentType::Integer);
        assert_eq!(detect_type("-10"), ArgumentType::Integer);
    }

    #[test]
    fn test_detect_type_float() {
        assert_eq!(detect_type("3.14"), ArgumentType::Float);
        assert_eq!(detect_type("-1.5"), ArgumentType::Float);
    }

    #[test]
    fn test_detect_type_bool() {
        assert_eq!(detect_type("true"), ArgumentType::Bool);
        assert_eq!(detect_type("false"), ArgumentType::Bool);
        assert_eq!(detect_type("yes"), ArgumentType::Bool);
    }

    #[test]
    fn test_detect_type_path() {
        assert_eq!(detect_type("/usr/bin"), ArgumentType::Path);
        assert_eq!(detect_type("./file"), ArgumentType::Path);
        assert_eq!(detect_type("..\\path"), ArgumentType::Path);
    }

    // ========================================================================
    // SECTION 4: PATH TESTS
    // ========================================================================

    #[test]
    fn test_normalize_path_windows() {
        assert_eq!(normalize_path("path\\to\\file"), "path/to/file");
    }

    #[test]
    fn test_normalize_path_unix() {
        assert_eq!(normalize_path("path/to/file"), "path/to/file");
    }

    #[test]
    fn test_get_extension_valid() {
        assert_eq!(get_extension("file.TXT"), Some("txt".to_string()));
        assert_eq!(get_extension("data.csv"), Some("csv".to_string()));
    }

    #[test]
    fn test_get_extension_none() {
        assert_eq!(get_extension("no_extension"), None);
        assert_eq!(get_extension(".hidden"), None);
    }

    #[test]
    fn test_has_extension_match() {
        assert!(has_extension("data.csv", &["csv", "tsv"]));
        assert!(has_extension("config.YAML", &["yaml", "yml"]));
    }

    #[test]
    fn test_has_extension_no_match() {
        assert!(!has_extension("data.txt", &["csv", "json"]));
        assert!(!has_extension("no_ext", &["txt"]));
    }

    // ========================================================================
    // SECTION 5: TEST HELPERS TESTS
    // ========================================================================

    #[test]
    fn test_create_test_config() {
        let config = test_helpers::create_test_config("test", vec!["cmd1", "cmd2"]);
        assert_eq!(config.metadata.prompt, "test");
        assert_eq!(config.commands.len(), 2);
    }

    #[test]
    fn test_create_test_command() {
        let cmd = test_helpers::create_test_command("test", true);
        assert_eq!(cmd.name, "test");
        assert!(cmd.required);
    }

    #[test]
    fn test_test_context_downcast() {
        let mut ctx = test_helpers::TestContext::default();
        ctx.executed.push("test".to_string());

        let ctx_ref = &ctx as &dyn ExecutionContext;
        let downcast = crate::context::downcast_ref::<test_helpers::TestContext>(ctx_ref);
        assert!(downcast.is_some());
        assert_eq!(downcast.unwrap().executed.len(), 1);
    }
}
