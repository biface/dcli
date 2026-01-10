# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![License](https://img.shields.io/crates/l/dynamic-cli.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

[🇫🇷 Version française](README.fr.md)

**dynamic-cli** is a Rust framework for quickly building configurable CLI (Command Line Interface) and REPL (Read-Eval-Print Loop) applications through YAML or JSON configuration files.

Instead of manually coding each command with `clap` or other libraries, you define your commands in a configuration file, and **dynamic-cli** automatically generates:
- Argument parsing
- Input validation
- Contextual help
- Interactive mode (REPL)
- Error handling with smart suggestions

## 🎯 Use Cases

- **Scientific tools**: simulators, data analyzers, computational tools
- **File managers**: configurable batch operations, directory navigation
- **Task managers**: todo lists, project tracking, workflow automation
- **API clients**: interactive interfaces for web services
- **Build tools**: custom compilation systems, deployment scripts
- **Testing applications**: configurable test frameworks, test runners

## ✨ Features

- ✅ **Declarative configuration**: define commands in YAML/JSON
- ✅ **Dual mode**: classic CLI OR interactive REPL (auto-detected)
- ✅ **Automatic validation**: types, ranges, files, multiple choices
- ✅ **Smart suggestions**: typo correction with Levenshtein distance
- ✅ **Rich error handling**: clear messages with context and suggestions
- ✅ **REPL history**: automatic save between sessions (via rustyline)
- ✅ **Extensible**: custom context, custom validations
- ✅ **Type-safe**: Rust traits for implementations
- ✅ **Utility functions**: 18+ helper functions for common tasks
- ✅ **Colored output**: user-friendly error messages

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dynamic-cli = "0.1"
```

### Minimal Example

**1. Create a `commands.yaml` file:**

```yaml
metadata:
  version: "1.0.0"
  prompt: "myapp"
  prompt_suffix: " > "

commands:
  - name: greet
    aliases: [hello, hi]
    description: "Greet someone"
    required: true
    arguments:
      - name: name
        arg_type: string
        required: true
        description: "Name to greet"
        validation: []
    options: []
    implementation: "greet_handler"

global_options: []
```

**2. Implement the handler in Rust:**

```rust
use dynamic_cli::prelude::*;
use std::collections::HashMap;

// Define execution context (shared state)
#[derive(Default)]
struct MyContext {
    greeting_count: usize,
}

impl ExecutionContext for MyContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// Implement command handler
struct GreetCommand;

impl CommandHandler for GreetCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<MyContext>(context)
            .ok_or_else(|| /* error handling */)?;
        
        let name = args.get("name").unwrap();
        println!("Hello, {}!", name);
        ctx.greeting_count += 1;
        
        Ok(())
    }
}

fn main() -> Result<()> {
    CliBuilder::new()
        .config_file("commands.yaml")
        .context(Box::new(MyContext::default()))
        .register_handler("greet_handler", Box::new(GreetCommand))
        .build()?
        .run()
}
```

**3. Use the application:**

```bash
# CLI mode (one-shot command)
$ myapp greet Alice
Hello, Alice!

# REPL mode (interactive)
$ myapp
myapp > greet Bob
Hello, Bob!
myapp > hello Charlie
Hello, Charlie!
myapp > exit
```

## 📦 Complete Examples

The framework includes three complete, production-ready examples demonstrating different complexity levels:

### 1. Simple Calculator (Beginner)

A basic arithmetic calculator with history tracking.

```bash
# Run the calculator
cargo run --example simple_calculator

# Or use CLI mode
cargo run --example simple_calculator -- add 10 5
```

**Features:**
- Basic operations: add, subtract, multiply, divide
- Calculation history
- Last result recall
- Error handling (division by zero)

**Commands:** `add`, `subtract`, `multiply`, `divide`, `history`, `clear`, `last`

---

### 2. File Manager (Intermediate)

File system navigation and information tool with path validation.

```bash
# Run the file manager
cargo run --example file_manager

# Or use CLI mode
cargo run --example file_manager -- list .
cargo run --example file_manager -- info Cargo.toml
```

**Features:**
- List directory contents with sizes
- Display detailed file information
- Search files by pattern
- Path validation
- Human-readable sizes (KB, MB, GB)
- Statistics tracking

**Commands:** `list`, `info`, `search`, `stats`

---

### 3. Task Runner (Advanced)

Complete task management system with priorities and statistics.

```bash
# Run the task manager
cargo run --example task_runner

# Or use CLI mode
cargo run --example task_runner -- add "Write docs" --priority high
cargo run --example task_runner -- list
```

**Features:**
- Add tasks with priorities (low, medium, high)
- List pending or all tasks
- Mark tasks as completed
- Delete tasks
- Clear completed tasks
- Advanced statistics with completion rate
- Custom validation

**Commands:** `add`, `list`, `complete`, `delete`, `clear`, `stats`

**See [examples/README.md](examples/README.md) for detailed documentation.**

## 📖 Complete Documentation

### Command Configuration

The configuration file defines all available commands with their arguments, options, and validation rules:

```yaml
commands:
  - name: calculate
    aliases: [calc, compute]
    description: "Perform calculations"
    required: true
    
    arguments:
      - name: operation
        arg_type: string
        required: true
        description: "Operation: add, subtract, multiply, divide"
        validation: []
        
    options:
      - name: precision
        short: p
        long: precision
        option_type: integer
        required: false
        default: "2"
        description: "Number of decimal places"
        choices: []
        
      - name: verbose
        short: v
        long: verbose
        option_type: bool
        required: false
        description: "Enable verbose output"
        choices: []
    
    implementation: "calculate_handler"
```

### Supported Types

- **`string`**: text string (UTF-8)
- **`integer`**: signed integer (i64)
- **`float`**: floating-point number (f64)
- **`bool`**: boolean (accepts: true/false, yes/no, 1/0, on/off)
- **`path`**: file/directory path

### Validation Rules

Dynamic-cli provides built-in validators that can be applied to arguments:

```yaml
arguments:
  - name: config_file
    arg_type: path
    required: true
    validation:
      - must_exist: true
      - extensions: [yaml, yml, json]
      
  - name: percentage
    arg_type: float
    required: true
    validation:
      - min: 0.0
        max: 100.0
```

Available validators:
- **`must_exist`**: file/directory must exist
- **`extensions`**: file must have one of the specified extensions
- **`range`**: number must be within min/max bounds

### Execution Context

The context allows sharing state between commands:

```rust
use dynamic_cli::prelude::*;

#[derive(Default)]
struct AppContext {
    current_file: Option<PathBuf>,
    settings: HashMap<String, String>,
    verbose: bool,
}

impl ExecutionContext for AppContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

Use the provided helper functions for safe downcasting:

```rust
// In your handler
let ctx = downcast_mut::<AppContext>(context)
    .ok_or_else(|| /* error handling */)?;
```

### Command Handlers

Each command is implemented via the `CommandHandler` trait:

```rust
use dynamic_cli::prelude::*;

struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        // Get typed context
        let ctx = downcast_mut::<AppContext>(context)?;
        
        // Parse arguments with utility functions
        let count = parse_int(args.get("count").unwrap(), "count")?;
        let verbose = parse_bool(
            args.get("verbose").unwrap_or(&"false".to_string())
        )?;
        
        // Validate
        if is_blank(args.get("name").unwrap()) {
            return Err(/* validation error */);
        }
        
        // Execute logic
        println!("Processing {} items", count);
        
        Ok(())
    }
    
    // Optional: custom validation beyond config
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        // Additional validations
        Ok(())
    }
}
```

### Utility Functions

Dynamic-cli provides 18+ utility functions for common tasks:

**Type Conversion:**
```rust
parse_int(value, field_name) -> Result<i64>
parse_float(value, field_name) -> Result<f64>
parse_bool(value) -> Result<bool>  // Accepts: true/false, yes/no, 1/0, on/off
detect_type(value) -> ArgumentType  // Auto-detect type
```

**String Validation:**
```rust
is_blank(s) -> bool
normalize(s) -> String  // Trim + lowercase
truncate(s, max_len) -> String
is_valid_email(s) -> bool
```

**Path Manipulation:**
```rust
normalize_path(path) -> String  // Cross-platform
get_extension(path) -> Option<String>
has_extension(path, extensions) -> bool
```

**Formatting:**
```rust
format_bytes(bytes) -> String  // "2.50 MB"
format_duration(duration) -> String  // "1h 30m 5s"
format_numbered_list(items) -> String
format_table(headers, rows) -> String
```

**See full documentation at [docs.rs/dynamic-cli](https://docs.rs/dynamic-cli)**

## 🏗️ Architecture

```
dynamic-cli/
├── config/       Configuration loading and validation
├── context/      Execution context trait
├── executor/     Command execution logic
├── registry/     Command and handler registry
├── parser/       CLI and REPL argument parsing
├── validator/    Argument validation
├── interface/    CLI and REPL interfaces
├── builder/      Fluent builder API
├── utils/        Utility functions
└── error/        Error types with suggestions
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with coverage
cargo test --all-features

# Run specific example
cargo run --example simple_calculator

# Run benchmarks (if available)
cargo bench
```

## 🔧 Advanced Usage

### Custom Validators

Implement custom validation in your handlers:

```rust
impl CommandHandler for MyCommand {
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        let value = parse_int(args.get("count").unwrap(), "count")?;
        if value < 1 || value > 1000 {
            return Err(ValidationError::OutOfRange {
                arg_name: "count".to_string(),
                value: value as f64,
                min: 1.0,
                max: 1000.0,
            }.into());
        }
        Ok(())
    }
}
```

### Error Handling

Dynamic-cli provides rich error types with context:

```rust
use dynamic_cli::error::{DynamicCliError, ParseError};

// Errors include suggestions for typos
let error = ParseError::unknown_command_with_suggestions(
    "simulat",
    &["simulate", "validate"]
);
// Error: Unknown command: 'simulat'
// Did you mean: simulate?
```

### REPL History

REPL mode automatically saves command history between sessions using rustyline:

```bash
# History is saved in:
# - Linux/macOS: ~/.local/share/dynamic-cli/history
# - Windows: %APPDATA%\dynamic-cli\history
```

## 🎓 Learning Path

1. **Start with Simple Calculator** (30 min)
   - Learn basic command structure
   - Understand context management
   - Simple argument parsing

2. **Explore File Manager** (45 min)
   - Path validation
   - File operations
   - Options and flags
   - Formatted output

3. **Study Task Runner** (1 hour)
   - Complex state management
   - Custom validation
   - Business logic
   - Statistics and reporting

**See [examples/README.md](examples/README.md) for detailed guides.**

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Ensure all tests pass (`cargo test`)
5. Submit a pull request

For major changes, please open an issue first to discuss the proposed changes.

## 📄 License

This project is licensed MIT License ([LICENSE-MIT](LICENSE))

at your option.

## 🔗 Links

- **Documentation**: [docs.rs/dynamic-cli](https://docs.rs/dynamic-cli)
- **Crates.io**: [crates.io/crates/dynamic-cli](https://crates.io/crates/dynamic-cli)
- **Repository**: [github.com/biface/dynamic-cli](https://github.com/biface/dcli)
- **Examples**: [examples/](examples/)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

## 🙏 Acknowledgments

This framework was developed as part of the **chrom-rs** project (chromatography simulator) and generalized for broader use.

Special thanks to:
- The Rust community for excellent crates (serde, thiserror, rustyline)
- Early users and testers for valuable feedback
