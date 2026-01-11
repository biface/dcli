# Contributing to dynamic-cli

First off, thank you for considering contributing to dynamic-cli! 🎉

**English** | **[Français](CONTRIBUTING.fr.md)**

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

---

## 📜 Code of Conduct

This project and everyone participating in it is governed by our Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

### Our Standards

**Positive behaviors include:**
- Using courteous and welcoming language
- Being respectful of differing viewpoints and experiences
- Constructive criticism helps us move forward and improve—let's embrace it
- Focusing on what is best for the community
- Showing empathy towards other community members

**Unacceptable behaviors include:**
- Trolling, insulting/derogatory comments, and personal attacks
- Public or private harassment
- Publishing others' private information without permission
- Other conduct which could reasonably be considered inappropriate

---

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have installed:

```bash
# Rust (latest stable version)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Essential tools
rustup component add rustfmt clippy
```

**Recommended versions:**
- Rust: 1.75.0 or later
- Cargo: Latest stable version

### Quick Start

```bash
# 1. Fork the repository on GitHub
# 2. Clone your fork
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# 3. Add the upstream remote
git remote add upstream https://github.com/biface/dcli.git

# 4. Create a branch
git checkout -b feature/my-awesome-feature

# 5. Make your changes
# ...

# 6. Run the tests
cargo test --all-features

# 7. Commit and push
git commit -am "Add awesome feature"
git push origin feature/my-awesome-feature

# 8. Create a Pull Request on GitHub
```

---

## 🛠 Development Setup

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# Install dependencies and build
cargo build

# Run tests to verify setup
cargo test --all-features

# Run examples to see it in action
cargo run --example simple_calculator
```

### Development Tools

We use several tools to maintain code quality:

```bash
# Format code
cargo fmt

# Check for common errors
cargo clippy --all-features -- -D warnings

# Run all tests
cargo test --all-features

# Generate documentation
cargo doc --no-deps --open

# Run benchmarks
cargo bench
```

### Project Structure

```
dynamic-cli/
├── src/
│   ├── lib.rs              # Library entry point
│   ├── error/              # Error types and handling
│   ├── config/             # Configuration loading and validation
│   ├── context/            # Execution context traits
│   ├── executor/           # Command execution
│   ├── registry/           # Command registry
│   ├── parser/             # CLI and REPL parsing
│   ├── validator/          # Argument validation
│   ├── interface/          # CLI and REPL interfaces
│   ├── builder.rs          # Builder API
│   └── utils.rs            # Utility functions
├── examples/               # Example applications
├── tests/                  # Integration tests
├── benches/                # Benchmarks
└── docs/                   # Additional documentation
```

---

## 💡 How Can I Contribute?

### Reporting Bugs

**Before submitting a bug report:**
- Check the [issue tracker](https://github.com/biface/dcli/issues) to see if it's already reported
- Try to reproduce the issue with the latest version
- Collect relevant information (OS, Rust version, error messages)

**When submitting a bug report, include:**
- A clear and descriptive title
- Detailed steps to reproduce the problem
- Expected behavior vs. actual behavior
- Code samples or test cases (if applicable)
- Your environment details

**Bug report template:**
```markdown
**Description:**
A clear description of the bug.

**Steps to Reproduce:**
1. Step 1
2. Step 2
3. ...

**Expected Behavior:**
What you expected to happen.

**Actual Behavior:**
What actually happened.

**Environment:**
- OS: [e.g., Ubuntu 22.04]
- Rust version: [e.g., 1.75.0]
- dynamic-cli version: [e.g., 0.1.0]

**Additional Context:**
Any other relevant information.
```

### Suggesting Features

**Before suggesting a feature:**
- Check if it's not already suggested or in development
- Consider if it fits the project's scope and goals
- Think about the benefit it would bring to most users

**When suggesting a feature, include:**
- A clear and descriptive title
- The problem your feature solves
- Your proposed solution
- Alternative solutions you've considered
- Any relevant examples or use cases

**Feature request template:**
```markdown
**Problem:**
Describe the problem you're trying to solve.

**Proposed Solution:**
Describe your proposed solution.

**Alternatives:**
Other solutions you've considered.

**Use Cases:**
Real-world scenarios where this would be useful.
```

### Improving Documentation

Documentation improvements are always welcome! This includes:

- Fixing typos or grammatical errors
- Clarifying confusing explanations
- Adding missing documentation
- Improving code examples
- Translating documentation

**Documentation locations:**
- API documentation: Rustdoc comments in source files
- User guide: `docs/` directory
- Examples: `examples/` directory
- README: `README.md` and `README.fr.md`
- This file: `CONTRIBUTING.md` and `CONTRIBUTING.fr.md`

### Contributing Code

We welcome code contributions! Here are the types of contributions we're looking for:

**Bug fixes:**
- Fix reported issues
- Improve error handling
- Improve edge case handling

**Features:**
- Implement requested features
- Add new features (after discussion)
- Improve existing features

**Refactoring:**
- Improve code quality
- Optimize performance
- Improve maintainability

**Tests:**
- Add missing tests
- Improve test coverage
- Add integration tests

---

## 🔄 Development Workflow

### 1. Find or Create an Issue

- Check existing issues
- Create a new issue if necessary
- Discuss your approach before coding (for big changes)

### 2. Fork and Branch

```bash
# Fork on GitHub, then:
git clone https://github.com/biface/dynamic-cli.git
cd dynamic-cli

# Add upstream
git remote add upstream https://github.com/biface/dynamic-cli.git

# Create a feature branch
git checkout -b feature/descriptive-name
# or
git checkout -b fix/issue-number
```

**Branch naming conventions:**
- `feature/description` - New features
- `fix/issue-number` - Bug fixes
- `docs/description` - Documentation
- `refactor/description` - Code refactoring
- `test/description` - Test improvements

### 3. Make Your Changes

**Follow these practices:**
- Write clean and readable code
- Follow coding standards (see below)
- Add tests for new features
- Update documentation as needed
- Keep commits atomic and focused

### 4. Test Your Changes

```bash
# Run all tests
cargo test --all-features

# Run clippy
cargo clippy --all-features -- -D warnings

# Format code
cargo fmt

# Check documentation
cargo doc --no-deps

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### 5. Commit Your Changes

**Good commit messages:**
```bash
# Format: <type>: <subject>

# Examples:
git commit -m "feat: add support for custom validators"
git commit -m "fix: resolve parsing issue with escaped quotes"
git commit -m "docs: improve executor module documentation"
git commit -m "test: add integration tests for REPL mode"
git commit -m "refactor: simplify error handling in parser"
```

**Commit types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `test`: Tests
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `style`: Code style changes
- `chore`: Build/tooling changes

### 6. Push and Create a Pull Request

```bash
# Push to your fork
git push origin feature/my-feature

# Create a Pull Request on GitHub
# Fill out the PR template
```

---

## 📏 Coding Standards

### General Principles

- **Clarity over cleverness**: Write code that's easy to understand
- **Consistency**: Follow existing patterns in the codebase
- **Documentation**: Document public APIs and complex logic
- **Tests**: Aim for 80-90% test coverage
- **Performance**: Optimize when necessary, but prioritize correctness

### Rust-Specific Guidelines

**Code style:**
- Follow `rustfmt` defaults (run `cargo fmt`)
- Follow `clippy` suggestions (run `cargo clippy`)
- Use meaningful variable and function names
- Keep functions focused and small

**Error handling:**
- Use `Result<T>` for fallible operations
- Provide context in error messages
- Use `thiserror` for error types
- Use `anyhow` for application-level errors

**Documentation:**
- Document all public items with `///` comments
- Include examples in documentation
- Explain the "why", not just the "what"
- Use proper Markdown formatting

**Example:**
```rust
/// Parse a command-line argument into the specified type
///
/// This function attempts to parse a string value into the target type
/// specified by `arg_type`. It handles all supported argument types
/// and provides detailed error messages on failure.
///
/// # Arguments
///
/// * `value` - The string value to parse
/// * `arg_type` - The target type for parsing
///
/// # Returns
///
/// A `Result` containing the parsed value as a string, or an error
/// if parsing fails.
///
/// # Errors
///
/// Returns [`ParseError::TypeParseError`] if the value cannot be
/// parsed into the specified type.
///
/// # Examples
///
/// ```
/// use dynamic_cli::parser::parse_value;
/// use dynamic_cli::config::ArgumentType;
///
/// let result = parse_value("42", ArgumentType::Integer)?;
/// assert_eq!(result, "42");
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn parse_value(
    value: &str,
    arg_type: ArgumentType,
) -> Result<String> {
    // Implementation
}
```

### Code Organization

**Module structure:**
- One module per major responsibility
- Public API in `mod.rs`
- Private implementation in separate files
- Tests in `#[cfg(test)]` modules

**Naming conventions:**
- `snake_case` for functions and variables
- `PascalCase` for types and traits
- `SCREAMING_SNAKE_CASE` for constants
- Prefix unused private items with underscore

### Performance

**Optimization guidelines:**
- Profile before optimizing
- Document performance-critical sections
- Use appropriate data structures
- Avoid unnecessary allocations
- Clone only when necessary

---

## 🧪 Testing Guidelines

### Test Coverage Goals

- **Unit tests**: 80-90% coverage
- **Integration tests**: Cover main workflows
- **Documentation tests**: All public examples work
- **Edge cases**: Test error conditions

### Writing Tests

**Unit tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_integer() {
        let result = parse_integer("42").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_parse_invalid_integer() {
        let result = parse_integer("not a number");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_parse_integer_overflow() {
        parse_integer("999999999999999999999").unwrap();
    }
}
```

**Integration tests:**
```rust
// tests/cli_integration.rs
use dynamic_cli::prelude::*;

#[test]
fn test_complete_cli_workflow() {
    // Test complete CLI workflow
}
```

**Documentation tests:**
```rust
/// Parse an integer from a string
///
/// # Examples
///
/// ```
/// use dynamic_cli::parser::parse_integer;
///
/// let value = parse_integer("42")?;
/// assert_eq!(value, 42);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_integer(s: &str) -> Result<i64> {
    // Implementation
}
```

### Running Tests

```bash
# All tests
cargo test --all-features

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Documentation tests
cargo test --doc

# Integration tests only
cargo test --test '*'

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Test Organization

**File organization:**
- Unit tests: Same file as code in `#[cfg(test)]` module
- Integration tests: `tests/` directory
- Benchmarks: `benches/` directory

**Test naming:**
- Descriptive names: `test_parse_valid_integer`
- Group related tests in modules
- Use `#[ignore]` for slow tests

---

## 📚 Documentation

### Documentation Standards

**All public items must have:**
- Summary line
- Detailed description
- Arguments (for functions)
- Return value (for functions)
- Errors (for fallible functions)
- Examples
- Links to related items

**Example:**
```rust
/// Load configuration from a YAML or JSON file
///
/// Automatically detects the file format based on the extension
/// (`.yaml`, `.yml`, or `.json`) and parses the content accordingly.
///
/// # Arguments
///
/// * `path` - Path to the configuration file
///
/// # Returns
///
/// The parsed [`CommandsConfig`] on success.
///
/// # Errors
///
/// - [`ConfigError::FileNotFound`] if the file doesn't exist
/// - [`ConfigError::UnsupportedFormat`] if the extension is not supported
/// - [`ConfigError::YamlParse`] or [`ConfigError::JsonParse`] on parsing errors
///
/// # Examples
///
/// ```no_run
/// use dynamic_cli::config::load_config;
///
/// let config = load_config("commands.yaml")?;
/// println!("Loaded {} commands", config.commands.len());
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
///
/// # See Also
///
/// - [`load_yaml`] - Parse YAML content directly
/// - [`load_json`] - Parse JSON content directly
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<CommandsConfig> {
    // Implementation
}
```

### Documentation Best Practices

- Write in English (international audience) for code
- Use proper grammar and spelling
- Be concise but complete
- Include practical examples
- Link to related documentation
- Update docs when changing code

### Generating Documentation

```bash
# Generate and open documentation
cargo doc --no-deps --open

# Check for broken links
cargo doc --no-deps 2>&1 | grep warning

# Generate with all features
cargo doc --all-features --no-deps
```

---

## 🔀 Pull Request Process

### Before Submitting

**Checklist:**
- [ ] Code follows style guidelines (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy --all-features -- -D warnings`)
- [ ] All tests pass (`cargo test --all-features`)
- [ ] Documentation is updated
- [ ] New tests added for new features
- [ ] Commit messages are clear
- [ ] Branch is up to date with main

### PR Template

```markdown
## Description

Brief description of the changes.

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update

## Related Issues

Fixes #(issue number)

## Tests

Describe how you tested your changes:
- Test cases added
- Manual testing performed
- Edge cases considered

## Checklist

- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Code is commented where necessary
- [ ] Documentation updated
- [ ] No new warnings
- [ ] Tests added
- [ ] All tests pass
```

### Review Process

1. **Automated checks**: CI must pass
2. **Code review**: At least one approval required
3. **Discussion**: Address reviewer feedback
4. **Updates**: Make requested changes
5. **Approval**: Get final approval
6. **Merge**: Maintainer merges the PR

### After Merge

- Delete your feature branch
- Update your fork:
  ```bash
  git checkout main
  git pull upstream main
  git push origin main
  ```

---

## 🤝 Community

### Getting Help

**If you need help:**
- Check existing documentation
- Search existing issues
- Ask in discussions
- Create a new issue

**Be respectful and patient:**
- Maintainers are volunteers
- Provide complete information
- Be open to feedback
- Follow up on responses

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Pull Requests**: Code contributions

### Recognition

We value all contributions! Contributors are recognized in:
- Project README
- Release notes
- GitHub contributors page

---

## 📜 License

By contributing to dynamic-cli, you agree that your contributions will be licensed under the MIT/Apache-2.0 dual license.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

---

## 🙏 Thank You!

Your contributions aim to improve dynamic-cli. Whether you're fixing a typo, reporting a bug, or implementing a major feature, we appreciate your effort and time.

Happy coding! 🚀

---

## 📖 Additional Resources

**Learning Rust:**
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings)

**Rust Best Practices:**
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)

**Project Specific:**
- [API Documentation](https://docs.rs/dynamic-cli)
- [Examples](./examples)
- [Changelog](CHANGELOG.md)

---

**Last Updated**: 2026-01-11  
**Version**: 0.1.0
