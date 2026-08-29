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
- **🔄 CLI, REPL & Batch Modes** : Command-line, interactive, and scripted batch execution
- **🔗 Command Chaining** : Chain multiple commands in one invocation, with per-command `continue_on_failure` / `requires_success` policy
- **✅ Automatic Validation** : Built-in type checking and constraint validation
- **🎨 Rich Error Messages** : Colorful and informative messages with suggestions
- **🔌 Plugin System** : Granular static plugins (help, version, exit, sysinfo, env, config —
  compose only what you need) and sandboxed WASM plugins (loaded at runtime)
- **📚 Well Documented** : Complete API documentation and examples
- **🧪 Thoroughly Tested** : Extensive test coverage
- **⚡ Performance** : Zero-cost abstractions with efficient parsing

---

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dynamic-cli = "0.8.0"

# Optional — sandboxed WASM plugins (see Plugin System below)
# dynamic-cli = { version = "0.8.0", features = ["wasm-plugins"] }
```

> **Upgrading from 0.7.x?** Cargo treats a `0.x` minor bump as a
> semver-incompatible change, so `cargo update` alone won't pull `0.8.0`
> in — bump the version requirement explicitly. This release is fully
> additive (see [`CHANGELOG.md`](CHANGELOG.md)); no code changes are
> required on your side.

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

**Command chaining** (v0.8.0+) — chain more than one command in a single
invocation, no separator needed: once a command's arguments are
exhausted, the next recognized command name starts the next one.

```bash
$ myapp configure model.yml configure scenario.yml solve
# "configure" runs twice, then "solve" — see CONFIG_SYNTAX_REFERENCE.md
# for the continue_on_failure / requires_success failure policy
```

**Batch mode** — run a whole file of commands, one per line (blank lines and
`#`-prefixed comments are skipped):

```rust, ignore
let outcome = app.run_script("commands.txt", ScriptErrorPolicy::Continue)?;
println!("{}/{} succeeded", outcome.lines_succeeded, outcome.lines_executed);
```

The same file can also be loaded from inside an already-running REPL
session with `:load commands.txt`. `run_script()` inherits command
chaining automatically (it shares its dispatch path with `run()`); `:load`
does **not** — each loaded line still runs as a single command, exactly
as before v0.8.0.

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

Each of `help`/`version`/`exit` is also available as its own independent
plugin (`HelpPlugin`, `VersionPlugin`, `ExitPlugin`) — register only the one
you need instead of the bundle. Three more, feature-gated, cover common
introspection needs: `SysInfoPlugin` (`sysinfo-plugin`, OS/architecture/
parallelism), `EnvPlugin` (`env-plugin`, filtered environment variables —
sensitive-looking ones hidden by default), and `ConfigPlugin`
(`config-plugin`, show/re-validate the loaded YAML config without
restarting):

```rust, ignore
use dynamic_cli::plugin::{ConfigPlugin, SysInfoPlugin};

CliBuilder::new()
    .config_file("commands.yaml")
    .context(Box::new(MyContext::default()))
    .register_plugin(Box::new(SysInfoPlugin::new()))
    .register_plugin(Box::new(ConfigPlugin::new().with_config(config)))
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
- **[rpn_calculator.rs](examples/rpn_calculator.rs)** - Reverse Polish Notation calculator
- **[advanced_rpn_calculator.rs](examples/advanced_rpn_calculator.rs)** - HP-41CX-flavored RPN calculator with scientific functions, memory registers, `SysInfoPlugin`/`ConfigPlugin`, and batch/`:load` execution
- **[file_manager.rs](examples/file_manager.rs)** - File operations with validation
- **[task_runner.rs](examples/task_runner.rs)** - Task management application
- **[async_token_demo.rs](examples/async_token_demo.rs)** - Async command handler demo

Run any example:
```bash
cargo run --example simple_calculator

# advanced_rpn_calculator needs its two feature flags:
cargo run --example advanced_rpn_calculator --features sysinfo-plugin,config-plugin
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
- **plugin** - Granular static plugins (`plugin::builtin`: help, version, exit, sysinfo, env, config) and sandboxed WASM (`wasm-plugins` feature) extension mechanisms

---

## 🧪 Tests

```bash
# Run all tests (default features)
cargo test

# Run all tests, across every feature flag
cargo test --all-features

# Run with coverage
cargo llvm-cov --all-targets --all-features --workspace

# Check code quality
cargo clippy --all-features -- -D warnings
```

**Current test statistics:**

- **500+ unit tests** ✅
- **230+ documentation tests**
- **12 integration tests** (static + WASM plugins, full public API chain)
- **80-90% code coverage** *(not re-measured for v0.8.0 — `cargo-llvm-cov` wasn't run this sprint either)*
- **Zero clippy warnings**, confirmed across `--all-features`
  (`wasm-plugins`, `sysinfo-plugin`, `env-plugin`, `config-plugin` combined)

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

**Last updated**: 2026-08-23
