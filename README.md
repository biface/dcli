# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![codecov](https://codecov.io/gh/biface/dcli/graph/badge.svg?token=58T5WKC802)](https://codecov.io/gh/biface/dcli)
[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A powerful Rust framework for creating configurable CLI and REPL applications via YAML/JSON files.

**Define your command-line interface in a configuration file, not in code.** ✨

---

**English** | **[Français](README.fr.md)**

---

## 🎯 Features

- **📝 Configuration-Driven** : Define commands, arguments and options in YAML/JSON
- **🔄 CLI & REPL Modes** : Support for both command-line and interactive modes
- **✅ Automatic Validation** : Built-in type checking and constraint validation
- **🎨 Rich Error Messages** : Colorful and informative messages with suggestions
- **🔌 Plugin System** : Static plugins (compiled in) and sandboxed WASM plugins (loaded at runtime)
- **📚 Well Documented** : Complete API documentation and examples
- **🧪 Thoroughly Tested** : >80% test coverage with 345+ tests
- **⚡ Performance** : Zero-cost abstractions with efficient parsing

---

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dynamic-cli = "0.6.0"

# Optional — sandboxed WASM plugins (see Plugin System below)
# dynamic-cli = { version = "0.6.0", features = ["wasm-plugins"] }
```

### Basic Example

**1. Create a configuration file** (`commands.yaml`):

```yaml
metadata:
  version: "1.0.0"
  prompt: "myapp"
  prompt_suffix: " > "

commands:
  - name: greet
    aliases: [hello, hi]
    description: "Greet someone"
    required: false
    arguments:
      - name: name
        arg_type: string
        required: true
        description: "Name to greet"
        validation: []
    options:
      - name: loud
        short: l
        long: loud
        option_type: bool
        required: false
        description: "Use uppercase"
        choices: []
    implementation: "greet_handler"

global_options: []
```
> Note :
> 
> The proper syntax for the configuration file is available in [the project repository](CONFIG_SYNTAX_REFERENCE.md).  

**2. Implement your command handlers**:

```rust
use dynamic_cli::prelude::*;

// Define your application context
#[derive(Default)]
struct MyContext {
    // Your application state
}

impl ExecutionContext for MyContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// Implement the command handler
struct GreetCommand;

impl CommandHandler for GreetCommand {
    fn execute(
        &self,
        _context: &mut dyn ExecutionContext,
        args: &ParsedArgs,
    ) -> dynamic_cli::Result<()> {
        let name = args.get_scalar("name").unwrap();
        let loud = args.get_scalar("loud").map(|v| v == "true").unwrap_or(false);
        
        let greeting = format!("Hello, {}!", name);
        println!("{}", if loud { greeting.to_uppercase() } else { greeting });
        
        Ok(())
    }
}

fn main() -> dynamic_cli::Result<()> {
    CliBuilder::new()
        .config_file("commands.yaml")
        .context(Box::new(MyContext::default()))
        .register_sync_handler("greet_handler", Box::new(GreetCommand))
        .build()?
        .run()
}
```

**3. Run your application**:

```bash
# CLI mode
$ myapp greet Alice
Hello, Alice!

$ myapp greet Bob --loud
HELLO, BOB!

# REPL mode
$ myapp
myapp > greet Alice
Hello, Alice!
myapp > help
Available commands:
  greet [name] - Greet someone
myapp > exit
```

---

## 🔌 Plugin System

Extend an application with handlers that do not live in your own crate,
without modifying `dynamic-cli` itself. Two mechanisms are available:

| Mechanism                           | When to use it                                  | Cost                                                            |
|-------------------------------------|-------------------------------------------------|-----------------------------------------------------------------|
| **Static plugins** (`Plugin` trait) | Compiled into your binary                       | No `unsafe`, no extra dependency                                |
| **WASM plugins** (`WasmPlugin`)     | Distributed and loaded independently, sandboxed | `wasmtime` dependency, opt-in via `features = ["wasm-plugins"]` |

`dynamic-cli` ships `SystemPlugin` out of the box — `help`, `version`, and
`exit` in one call:

```rust, ignore
use dynamic_cli::plugin::SystemPlugin;

CliBuilder::new()
    .config_file("commands.yaml")
    .context(Box::new(MyContext::default()))
    .register_plugin(Box::new(SystemPlugin::new()))
    .register_sync_handler("greet_handler", Box::new(GreetCommand))
    .build()?
    .run()
```

WASM plugins run in a `wasmtime` sandbox, with no `unsafe` code on the host
side:

```rust, ignore
CliBuilder::new()
    .config_file("commands.yaml")
    .context(Box::new(MyContext::default()))
    .register_wasm_plugin(
        Path::new("plugins/greet.wasm"),
        &[("greet_hello", "say_hello")],
    )?
    .build()?
    .run()
```

Static and WASM plugins, and directly-registered handlers, all coexist in
the same application — the YAML configuration remains the single source of
truth for command definitions either way.

**[Full Plugin Guide →](PLUGIN_GUIDE.md)** ([Français](PLUGIN_GUIDE.fr.md)) —
the complete WASM ABI contract for third-party plugin authors, a worked
example, and the architecture decision behind it
([DD-021](https://github.com/biface/dcli/issues/10)).

---

## 📖 Documentation

- **[API Reference](https://docs.rs/dynamic-cli)** - Complete API documentation
- **[Examples](examples/README.md)** - Working examples and code samples
- **[Contributing Guide](CONTRIBUTING.md)** - How to contribute to the project

---

## 🎓 Examples

The [examples directory](examples) contains complete examples:

- **[simple_calculator.rs](examples/simple_calculator.rs)** - Basic arithmetic calculator
- **[file_manager.rs](examples/file_manager.rs)** - File operations with validation
- **[task_runner.rs](examples/task_runner.rs)** - Task management application

Run any example:
```bash
cargo run --example simple_calculator
```

---

## 🏗 Architecture

dynamic-cli is organized into focused modules:

- **config** - Configuration loading and validation
- **context** - Execution context trait
- **executor** - Command execution engine
- **registry** - Command and handler registry
- **parser** - CLI and REPL argument parsing
- **validator** - Argument validation
- **interface** - CLI and REPL interfaces
- **error** - Error types and display
- **builder** - Fluent API for building applications
- **help** - Dynamic `--help` generation
- **plugin** - Static (`Plugin` trait) and sandboxed WASM (`wasm-plugins` feature) extension mechanisms

---

## 🧪 Tests

```bash
# Run all tests (default features)
cargo test

# Run all tests, including WASM plugins
cargo test --features wasm-plugins

# Run with coverage
cargo llvm-cov --all-targets --workspace

# Check code quality
cargo clippy --all-features -- -D warnings
```

**Current test statistics:**
- **400+ unit tests** ✅
- **130+ documentation tests**
- **9 integration tests** (static + WASM plugins, full public API chain)
- **80-90% code coverage**, including the `wasm-plugins` feature
- **Zero clippy warnings** (default and `wasm-plugins`)

---

## 🤝 Contributing

We welcome contributions from everyone! Here's how you can help:

### Ways to Contribute

- 🐛 **Report bugs** - Found a bug? [Open an issue](https://github.com/biface/dcli/issues)
- 💡 **Suggest features** - Have an idea? [Start a discussion](https://github.com/biface/dcli/discussions)
- 📝 **Improve documentation** - Fix typos, clarify, add examples
- 🔧 **Submit code** - Fix bugs, implement features, improve performance
- 🧪 **Add tests** - Increase coverage, add edge cases

### Getting Started

```bash
# Fork and clone
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# Create a branch
git checkout -b feature/my-feature

# Make your changes and test
cargo test --all-features
cargo clippy --all-features

# Commit and push
git commit -am "Add awesome feature"
git push origin feature/my-feature
```

### Development Guidelines

**Before submitting a pull request:**

- [ ] Code follows Rust style guidelines (`cargo fmt`)
- [ ] All tests pass (`cargo test --all-features`)
- [ ] No clippy warnings (`cargo clippy --all-features -- -D warnings`)
- [ ] Documentation is updated
- [ ] New tests added for new features
- [ ] Commit messages are clear and descriptive

### Code of Conduct

This project follows a Code of Conduct to ensure a welcoming environment:

- ✅ Be respectful to others
- ✅ Welcome newcomers and help them learn
- ✅ Constructive criticism helps us move forward and improve—let's embrace it
- ✅ Focus on what's best for the community
- ❌ No harassment, trolling or personal attacks

**[Read the complete contributing guide →](CONTRIBUTING.md)**

---

## 📜 License

Licensed under your choice of:

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

## 🙏 Acknowledgments

- **Rust Community** - For the amazing tools and libraries developed
- **Contributors** - Everyone who has contributed to this project
- **[clap](https://github.com/clap-rs/clap)** - Inspiration for CLI design
- **[rustyline](https://github.com/kkawakam/rustyline)** - REPL functionality
- **[serde](https://github.com/serde-rs/serde)** - Serialization support

---

## 📞 Support

**Need help?**

- 📖 Check the [API documentation](https://docs.rs/dynamic-cli)
- 💬 Open a [discussion](https://github.com/biface/dcli/discussions)
- 🐛 Report an [issue](https://github.com/biface/dcli/issues)
- 📧 Contact the maintainers

**Found a security vulnerability?**  
Please report it privately to the maintainers.

---

## 🌟 Show Your Support

If you find dynamic-cli useful, please:

- ⭐ **Star the repository** on GitHub
- 📢 **Share** it with others who might find it useful
- 📝 **Write** a blog post or tutorial!

**Last updated**: 2026-07-24
