# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![License](https://img.shields.io/crates/l/dynamic-cli.svg)](LICENSE-MIT)

[🇫🇷 Version française](README.fr.md)

**dynamic-cli** is a Rust framework for quickly building configurable CLI (Command Line Interface) and REPL (Read-Eval-Print Loop) applications through YAML or JSON configuration files.

Instead of manually coding each command with `clap` or other libraries, you define your commands in a configuration file, and **dynamic-cli** automatically generates:
- Argument parsing
- Input validation
- Contextual help
- Interactive mode (REPL)
- Error handling with suggestions

## 🎯 Use Cases

- **Scientific tools**: simulators, data analyzers
- **File managers**: configurable batch operations
- **API clients**: interactive interfaces for web services
- **Build tools**: custom compilation systems
- **Testing applications**: configurable test frameworks

## ✨ Features

- ✅ **Declarative configuration**: define your commands in YAML/JSON
- ✅ **Dual mode**: classic CLI OR interactive REPL (auto-detected)
- ✅ **Automatic validation**: types, ranges, files, multiple choices
- ✅ **Smart suggestions**: typo correction with Levenshtein distance
- ✅ **Rich error handling**: clear messages with context
- ✅ **REPL history**: automatic save between sessions
- ✅ **Extensible**: custom context, custom validations
- ✅ **Type-safe**: Rust traits for implementations

## 🚀 Quick Start

### Installation

```toml
[dependencies]
dynamic-cli = "0.1"
```

### Minimal Example

**1. Create a `commands.yaml` file:**

```yaml
metadata:
  version: "1.0"
  prompt: "my-app"
  prompt_suffix: " > "

commands:
  - name: "greet"
    description: "Greet someone"
    arguments:
      - name: "name"
        type: "string"
        required: true
        description: "Name to greet"
    options: []
    implementation: "greet_handler"

global_options: []
```

**2. Implement the handler in Rust:**

```rust
use dynamic_cli::{
    CliBuilder, CommandHandler, ExecutionContext,
    Result,
};
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
        let ctx = context.downcast_mut::<MyContext>().unwrap();
        let name = args.get("name").unwrap();
        
        println!("Hello, {}!", name);
        ctx.greeting_count += 1;
        
        Ok(())
    }
}

fn main() -> Result<()> {
    // Build and run the application
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
# CLI mode
$ my-app greet Alice
Hello, Alice!

# REPL mode (interactive)
$ my-app
my-app > greet Bob
Hello, Bob!
my-app > exit
```

## 📖 Complete Documentation

### Command Configuration

The configuration file defines all available commands:

```yaml
commands:
  - name: "calculate"
    aliases: ["calc", "compute"]
    description: "Perform calculations"
    
    arguments:
      - name: "operation"
        type: "string"
        required: true
        description: "Operation to perform (+, -, *, /)"
        
    options:
      - name: "precision"
        short: "p"
        long: "precision"
        type: "integer"
        required: false
        default: "2"
        description: "Number of decimal places"
        
      - name: "verbose"
        short: "v"
        long: "verbose"
        type: "bool"
        default: "false"
        description: "Enable verbose output"
    
    implementation: "calculate_handler"
```

### Supported Types

- **`string`**: text string
- **`integer`**: integer number (i64)
- **`float`**: decimal number (f64)
- **`bool`**: boolean (true/false)
- **`path`**: file/directory path

### Validation

```yaml
arguments:
  - name: "config_file"
    type: "path"
    required: true
    validation:
      - must_exist: true
      - extensions: [".yaml", ".yml", ".json"]
      
  - name: "percentage"
    type: "float"
    validation:
      - range:
          min: 0.0
          max: 100.0
```

### Execution Context

The context allows sharing state between commands:

```rust
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

### Command Handlers

Each command is implemented via the `CommandHandler` trait:

```rust
struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        // Get typed context
        let ctx = context.downcast_mut::<AppContext>().unwrap();
        
        // Get arguments
        let value = args.get("some_arg").unwrap();
        
        // Execute logic
        println!("Doing something with: {}", value);
        
        Ok(())
    }
    
    // Custom validation (optional)
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        // Additional validations beyond config
        Ok(())
    }
}
```

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test '*'

# Benchmarks
cargo bench

# Code coverage (with tarpaulin)
cargo tarpaulin --out Html
```

## 📦 Examples

The repository contains several complete examples:

```bash
# Simple calculator
cargo run --example simple_calculator

# File manager
cargo run --example file_manager

# Task runner
cargo run --example task_runner
```

## 🤝 Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License. See [LICENSE-MIT](LICENSE-MIT).

## 🔗 Links

- [API Documentation](https://docs.rs/dynamic-cli)
- [Crates.io](https://crates.io/crates/dynamic-cli)
- [GitHub Repository](https://github.com/your-org/dynamic-cli)
- [Examples](https://github.com/your-org/dynamic-cli/tree/main/examples)

## 🙏 Acknowledgments

Inspired by the needs of the **chrom-rs** project (chromatography simulator).
