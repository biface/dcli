# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Ideas for Future Releases
- Configuration versioning and migration tools
- Subcommand support (git-style: `myapp config set key value`)
- Advanced REPL features (multi-line editing, syntax highlighting)
- Integration with system package managers
- Command pipelines and composition
- Remote command execution
- Configuration profiles (dev, prod, test)
- Telemetry and metrics collection

---

## [0.6.0] - Planned (Q1 2027)

**Theme**: Polish & Advanced Features  
**Estimated Effort**: 4-6 weeks  
**Dependencies**: None

### Planned

#### Configuration Management
- **Configuration hot-reload**: Watch config file and reload without restart
  - File system watching with `notify` crate
  - Graceful handler replacement
  - Validation before applying changes
  - Rollback on errors

#### Customization
- **Color scheme customization**: User-defined color themes
  - Theme definition in config file
  - Pre-defined themes (dark, light, high-contrast)
  - Per-element color control (errors, warnings, prompts, etc.)
  - Support for RGB and named colors

#### REPL Enhancements
- **Command history search**: Fuzzy search in history (Ctrl+R)
- **Multi-line command support**: Continue commands across lines with `\`
- **Command macros**: Define custom shortcuts in config

#### Quality of Life
- **Verbose mode**: `-v`/`--verbose` for detailed output
- **Quiet mode**: `-q`/`--quiet` for minimal output
- **Debug mode**: `--debug` for troubleshooting

---

## [0.5.0] - Planned (Q4 2026)

**Theme**: Async Support  
**Estimated Effort**: 4-6 weeks  
**Dependencies**: None (optional feature)

### Planned

#### Async Command Handlers (Optional Feature)
- **`AsyncCommandHandler` trait**: Async version of `CommandHandler`
  ```rust
  #[async_trait]
  pub trait AsyncCommandHandler: Send + Sync {
      async fn execute(
          &self,
          context: &mut dyn ExecutionContext,
          args: &HashMap<String, String>,
      ) -> Result<()>;
  }
  ```
- **Tokio runtime integration**: Optional tokio runtime
- **Non-blocking I/O**: Async file operations, network requests
- **Concurrent command execution**: Run multiple commands in parallel
- **Progress indicators**: Real-time progress for long operations

#### Feature Flag
```toml
[features]
default = ["sync"]
async = ["tokio", "async-trait"]
```

#### Examples
- **async_http_client**: Fetch data from APIs
- **async_file_processor**: Process large files asynchronously
- **concurrent_tasks**: Run multiple operations in parallel

#### Documentation
- Async/await guide for command handlers
- Migration guide from sync to async
- Performance comparison benchmarks

**Breaking Changes**: None (feature is optional)

---

## [0.4.0] - Planned (Q3 2026)

**Theme**: Extensibility  
**Estimated Effort**: 4-6 weeks  
**Dependencies**: None

### Planned

#### Plugin System
- **Dynamic plugin loading**: Load plugins from shared libraries (.so, .dylib, .dll)
  ```yaml
  plugins:
    - path: ./plugins/custom_validator.so
      config:
        enabled: true
    - path: ./plugins/extra_commands.so
  ```
- **Plugin API**: Stable ABI for external plugins
  - Add custom command handlers
  - Add custom validators
  - Register hooks (pre-execution, post-execution)
  - Access to context and registry

#### Custom Validators API
- **`ValidatorPlugin` trait**: Define custom validation logic
- **Registration system**: Register validators at runtime
- **Composition**: Chain multiple validators

#### Plugin Discovery
- **Plugin directory**: Auto-discover plugins in `~/.config/myapp/plugins/`
- **Plugin metadata**: Version, author, description
- **Dependency management**: Plugin dependencies and compatibility

#### Safety
- **Sandboxing**: Isolate plugin execution
- **Signature verification**: Verify plugin authenticity (optional)
- **Error isolation**: Plugin errors don't crash main application

#### Documentation
- Plugin development guide
- Plugin API reference
- Security considerations
- Example plugins repository

**Breaking Changes**: None (plugins are opt-in)

---

## [0.3.0] - 2026-05-30

**Theme**: Shell Completions & Advanced History  
**Dependencies**: v0.2.0

### Added

#### REPL Tab Completion (issue #18)
- New `DcliCompleter` and `DcliHelper` types (private) implementing the
  `rustyline::completion::Completer` trait. Completion operates at three depth levels
  driven by the YAML configuration:
  - Level 1: command names and aliases (`p<Tab>` → `peek`, `pop`, `push`)
  - Level 2: long and short option flags after a command (`push --<Tab>` → `--count`, `-c`)
  - Positional argument values are not completed (open-ended strings)
- `ReplInterface` now uses `Editor<DcliHelper, DefaultHistory>` instead of `DefaultEditor`,
  activating tab completion as soon as the REPL starts.
- `ReplInterface::new()` signature updated to accept all configuration upfront:
  `registry`, `context`, `prompt`, `config: Option<CommandsConfig>`,
  `help_formatter: Option<Box<dyn HelpFormatter>>`. Eliminates the two-phase
  construction pattern.
- `registry` and `config` are shared via `Arc<T>` between `ReplInterface` and
  `DcliCompleter` — single source of truth, no data duplication.
- `ReplInterface::with_help()` removed; `CliBuilder::run_repl()` adapted to pass
  the full configuration to `new()` in a single call.

#### Per-Application History & Secure Argument Filtering (issue #19)
- History is now stored per application under the XDG data directory:
  `~/.local/share/<app_name>/history` (Linux/macOS) via `dirs::data_local_dir()`.
  Each application built on `dynamic-cli` gets an isolated history file.
- New `secure: bool` field on `ArgumentDefinition` (YAML schema, `serde` default: `false`).
  Fully backward-compatible — existing configs without this field are unaffected.
- History write moved from `run()` to `execute_line()`: only successfully parsed
  commands are persisted; parse failures are silently discarded.
- When a parsed command contains at least one argument marked `secure: true`,
  the entire line is silently omitted from history. The command name itself is
  not filtered.

#### Example YAML for secure arguments
```yaml
arguments:
  - name: password
    arg_type: string
    required: true
    description: "User password"
    secure: true
```

### Changed

- `ReplInterface::new()` now takes 5 arguments (was 3). All call sites in
  `CliBuilder`, examples, and tests updated. `chrom-rs` is unaffected (uses
  `CliBuilder` exclusively).
- History path migrated from `~/.config/<app_name>/history.txt`
  (v0.2.0, `dirs::config_dir()`) to `~/.local/share/<app_name>/history`
  (v0.3.0, `dirs::data_local_dir()`). Existing history files are not migrated
  automatically.

### Fixed

- `test_validate_file_exists_relative_path`: removed `std::env::set_current_dir()`
  which mutated the process-wide working directory and caused data races under
  parallel test execution. Now uses `Cargo.toml` as a stable relative path.
- All `TempDir` + `File::create` patterns in validator tests replaced with
  `NamedTempFile` to eliminate a `Permission denied` race condition under
  parallel test execution.

**Breaking Changes**: None for `CliBuilder` users. `ReplInterface::new()` signature
changed — direct callers must update to the 5-argument form.

---

## [0.2.0] - 2026-04-05

**Theme**: Built-in Help & Error Improvements  
**Dependencies**: v0.1.1

### Added (issue #14)

#### REPL Help Support
- `ReplInterface` now intercepts `--help`, `-h`, `--help <command>`, `-h <command>`,
  `<command> --help`, and `<command> -h` in `execute_line()` before dispatch.
  Formatted help is printed via the configured `HelpFormatter`; normal command
  execution is unaffected.
- New `ReplInterface::with_help(config, formatter)` builder method — attaches
  a `CommandsConfig` and a `Box<dyn HelpFormatter>` to the REPL. Called
  automatically by `CliBuilder::run_repl()` when a formatter is registered.
- `CliBuilder::run_repl()` now wires `with_help()` automatically when a
  formatter has been supplied via `CliBuilder::help_formatter()`.

#### Coverage
- Overall line coverage: **95.76%** (target ≥ 85 %)
- All v0.2.0 modules exceed target:
  `help/mod.rs` 98.92%, `error/types.rs` 97.38%,
  `error/display.rs` 92.98%, `interface/repl.rs` 91.46%

### Added (issue #12)

#### Built-in Help System
- New `help` module with a `HelpFormatter` trait and a `DefaultHelpFormatter`
  implementation. Both are re-exported from the crate root and from `prelude`.
- `CliBuilder::help_formatter(Box<dyn HelpFormatter>) -> Self` — optional
  method to supply a custom formatter. Fully backward-compatible (additive).
- `CliApp::run_cli()` intercepts `--help` and `--help <command>` before
  command dispatch and prints formatted help to the terminal.
  The formatter is instantiated lazily, only when `--help` is detected.
- `DefaultHelpFormatter` produces aligned, colored output (via `colored`)
  listing all commands, their arguments, options, and aliases.
  Output is English-only; other languages are supported via custom
  `HelpFormatter` implementations.
- `CliApp` retains the `CommandsConfig` after `build()` to make it available
  to the formatter at runtime (additive private field — no downstream breakage).

### Fixed (issue #12)

- Pre-existing clippy warning `borrowed_box` on `CommandRegistry::get_handler()`
  suppressed with a justified `#[allow(clippy::borrowed_box)]` attribute.
  Changing the return type would be a breaking API change.

**Breaking Changes**: None

---

## [0.1.1] - 2026-01-11

### Fixed
- Silenced 11 clippy warnings while preserving necessary imports
    - Added `#[allow(unused_imports)]` for `Result` in `parser/mod.rs` (import is necessary)
    - Added `#[allow(unused_imports)]` for `ArgumentDefinition` in `parser/cli_parser.rs` (import is necessary)
    - Removed unnecessary `.enumerate()` calls in `config/validator.rs`
    - Added `#[allow(clippy::needless_range_loop)]` in `config/validator.rs` (readability)

---

## [0.1.0] - 2026-01-10

**Theme**: Foundation  
**Initial Release**

### Added

#### Core Framework
- Complete CLI/REPL framework driven by YAML/JSON configuration files
- 11 production-ready modules with >85% test coverage

#### Configuration System (`config` module)
- YAML primary format, JSON alternative via single `serde` pipeline
- `CommandsConfig` root structure with metadata, commands, and global options
- `CommandDefinition` with arguments, options, aliases, and validation rules
- `ArgumentDefinition` with type system (String, Integer, Float, Bool, Path)
- `OptionDefinition` with short/long forms, defaults, and restricted choices
- `ValidationRule` enum: `MustExist`, `Extensions`, `Range`
- Internal schema validator at startup

#### Error System (`error` module)
- Typed error hierarchy via `thiserror`: `DynamicCliError` with variants
  `Config`, `Parse`, `Validation`, `Execution`, `Registry`
- `suggestion: Option<String>` on key variants for actionable error messages
- Position-aware errors for configuration files
- Colored error output for better readability
- Context-preserving error propagation

#### Execution Context (`context` module)
- `ExecutionContext` trait for shared application state
- Type-safe downcasting with helper functions
- Thread-safe design (Send + Sync requirements)
- Support for custom context implementations

#### User Interface (`interface` module)
- CLI interface for one-shot command execution
- REPL interface with `rustyline` integration
- Persistent command history across sessions
- Colored prompts and output
- Automatic history directory creation

#### Utility Functions (`utils` module)
18+ utility functions organized in categories:

**Type Conversion:**
- `parse_int()` - Parse integers with contextual errors
- `parse_float()` - Parse floating-point numbers
- `parse_bool()` - Parse booleans (supports true/false, yes/no, 1/0, on/off)
- `detect_type()` - Automatic type detection

**String Validation:**
- `is_blank()` - Check for empty or whitespace-only strings
- `normalize()` - Trim and lowercase strings
- `truncate()` - Limit string length with ellipsis
- `is_valid_email()` - Basic email validation

**Path Manipulation:**
- `normalize_path()` - Cross-platform path normalization
- `get_extension()` - Extract file extension
- `has_extension()` - Check file extension against list

**Formatting:**
- `format_bytes()` - Human-readable byte sizes (B, KB, MB, GB, TB)
- `format_duration()` - Human-readable durations (1h 30m 5s)
- `format_numbered_list()` - Create numbered lists
- `format_table()` - Create text tables

**Test Helpers:**
- `create_test_config()` - Generate minimal test configurations
- `create_test_command()` - Generate test command definitions
- `TestContext` - Mock execution context for testing

#### Examples
Three complete, production-ready example applications:

**Simple Calculator** (beginner level):
- Basic arithmetic operations (add, subtract, multiply, divide)
- Calculation history tracking
- Last result recall
- Error handling (division by zero)
- 250 lines, 7 commands

**File Manager** (intermediate level):
- Directory listing with human-readable sizes
- Detailed file information display
- Pattern-based file search
- Path validation
- Statistics tracking
- 320 lines, 4 commands

**Task Runner** (advanced level):
- Task management with priorities (low, medium, high)
- Task completion tracking
- Advanced statistics with completion rate
- Custom validation
- State persistence
- 420 lines, 6 commands

#### Documentation
- Complete rustdoc documentation for all public APIs
- README.md with comprehensive usage guide
- README.fr.md (French translation)
- CONTRIBUTING.md (English and French)
- examples/README.md with detailed example documentation
- Learning path from beginner to advanced
- Troubleshooting guides

#### Testing
- 365+ unit and integration tests
- >85% code coverage
- Comprehensive test suite covering:
  - Configuration loading and validation
  - Command parsing and execution
  - Error handling and suggestions
  - Type conversion and validation
  - Context management
  - REPL functionality

#### Developer Experience
- `prelude` module for convenient imports
- Fluent builder API for application construction
- Clear error messages with suggestions
- Type-safe downcasting helpers
- Extensive inline documentation

### Technical Details

#### Dependencies
- `serde` 1.0 - Serialization/deserialization
- `serde_json` 1.0 - JSON support
- `serde_yaml` 0.9 - YAML support
- `thiserror` 2.0 - Error handling
- `anyhow` 1.0 - Error context
- `rustyline` 14.0 - REPL with history
- `dirs` 5.0 - Directory paths
- `colored` 3.0 - Terminal colors

#### Minimum Rust Version
- Rust 1.70.0 or higher

#### Platform Support
- Linux ✅
- macOS ✅
- Windows ✅

### Architecture

#### Module Structure
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

#### Design Patterns
- Builder pattern for application construction
- Trait objects for command handlers
- Type-safe downcasting with free functions
- Dual HashMap pattern for O(1) alias resolution
- Layered parser architecture

#### Key Design Decisions
- Object-safe traits for trait object usage
- Free functions for generic operations on trait objects
- Thread-safe design throughout (Send + Sync)
- Comprehensive error context preservation
- Separation of concerns between modules

### Quality Metrics
- **Lines of Code**: ~14,070
- **Number of Files**: 34
- **Test Count**: 365+
- **Code Coverage**: >85%
- **Clippy Warnings**: 0
- **Modules**: 11 (all complete)
- **Examples**: 3 (complete)

### Development Process
- 11 development sessions from conception to completion
- Iterative development with continuous validation
- Comprehensive testing from the start
- Zero-warning policy with clippy
- Production-ready code quality

---

## Version Roadmap Summary

| Version   | Theme                    | Key Features                        | Effort    | Status               |
|-----------|--------------------------|-------------------------------------|-----------|----------------------|
| **0.1.0** | Initial Release          | Complete framework                  | -         | ✅ Released           |
| **0.2.0** | Help & Errors            | Built-in help, better errors        | 3-4 weeks | ✅ Released           |
| **0.3.0** | Shell Completions        | REPL completion, secure history     | 3-4 weeks | ✅ Released           |
| **0.4.0** | Plugin System            | Extensible handlers                 | 4-6 weeks | 🔵 Planned Q3 2026   |
| **0.5.0** | Async Support            | Async handlers (optional)           | 4-6 weeks | 🔵 Planned Q4 2026   |
| **0.6.0** | Advanced Options         | Repeatable options, typed sub-params| 4-6 weeks | 🔵 Planned Q1 2027   |
| **1.0.0** | Stable                   | Production-ready, locked API        | -         | 🔵 Planned Q1 2028   |

---

## Development Guidelines

### For Each Release

1. **Planning** (1-2 days)
   - Review planned features
   - Adjust scope if needed
   - Create development checklist

2. **Implementation** (70% of time)
   - Follow TDD approach
   - Maintain >85% coverage
   - Zero clippy warnings

3. **Documentation** (20% of time)
   - Update rustdoc
   - Add examples
   - Update guides

4. **Testing & Polish** (10% of time)
   - Integration tests
   - Manual testing
   - Performance checks

### Release Criteria

- ✅ All tests pass
- ✅ Zero clippy warnings
- ✅ >85% code coverage
- ✅ Documentation complete
- ✅ CHANGELOG updated
- ✅ Examples work
- ✅ Migration guide (if breaking changes)

---

## Links

- **Documentation**: https://docs.rs/dynamic-cli
- **Crates.io**: https://crates.io/crates/dynamic-cli
- **Repository**: https://github.com/biface/dcli
- **Issues**: https://github.com/biface/dcli/issues
- **Discussions**: https://github.com/biface/dcli/discussions

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

---

## License

Licensed under either of:

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

---

**Last Updated**: 2026-05-30  
**Current Version**: 0.3.0  
**Next Release**: 0.4.0 (planned Q3 2026)
