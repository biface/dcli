# Dynamic-CLI Examples

This directory contains complete, working examples demonstrating the capabilities of the dynamic-cli framework.

## 📚 Available Examples

### 1. Simple Calculator (`simple_calculator.rs`)

A basic arithmetic calculator with history tracking.

**Features:**
- Basic operations: add, subtract, multiply, divide
- Calculation history
- Last result recall
- Error handling (division by zero)
- REPL and CLI modes

**Usage:**
```bash
# REPL mode (interactive)
cargo run --example simple_calculator

# CLI mode (one-shot)
cargo run --example simple_calculator -- add 5 3
cargo run --example simple_calculator -- multiply 4 7
cargo run --example simple_calculator -- history
```

**Example session:**
```
calc > add 10 5
Result: 15

calc > multiply 3 4
Result: 12

calc > history
Calculation History:
  1. 10 + 5 = 15
  2. 3 × 4 = 12

calc > last
Last result: 12
```

---

### 2. File Manager (`file_manager.rs`)

File system navigation and information tool.

**Features:**
- List directory contents with sizes
- Display detailed file information
- Search files by pattern
- Path validation
- Formatted output (human-readable sizes)
- Statistics tracking

**Usage:**
```bash
# REPL mode
cargo run --example file_manager

# CLI mode
cargo run --example file_manager -- list .
cargo run --example file_manager -- info Cargo.toml
cargo run --example file_manager -- search . --pattern "*.rs"
cargo run --example file_manager -- stats
```

**Example session:**
```
filemgr > list src

Directory: src
─────────────────────────────────────
Directories:
  1. config/
  2. error/

Files:
  1. lib.rs (12.50 KB)
  2. utils.rs (28.30 KB)
─────────────────────────────────────
Total: 2 directories, 2 files (40.80 KB)

filemgr > info src/lib.rs
File Information
═════════════════════════════════════
Path:       src/lib.rs
Type:       File
Size:       12.50 KB
Extension:  .rs
```

---

### 3. Task Runner (`task_runner.rs`)

Complete task management system with priorities.

**Features:**
- Add tasks with priorities (low, medium, high)
- List pending or all tasks
- Mark tasks as completed
- Delete tasks
- Clear completed tasks
- Statistics and completion rate
- Validation (non-empty descriptions, valid priorities)

**Usage:**
```bash
# REPL mode
cargo run --example task_runner

# CLI mode
cargo run --example task_runner -- add "Write documentation" --priority high
cargo run --example task_runner -- add "Fix bug #123" --priority medium
cargo run --example task_runner -- list
cargo run --example task_runner -- complete 0
cargo run --example task_runner -- stats
```

**Example session:**
```
tasks > add "Implement feature X" --priority high
✓ Task #0 added: Implement feature X [High]

tasks > add "Review PR" --priority medium
✓ Task #1 added: Review PR [Medium]

tasks > list
Pending Tasks:
═════════════════════════════════════════════════════════
[ ] #00 !!! [High] Implement feature X
[ ] #01 !!  [Medium] Review PR
═════════════════════════════════════════════════════════
Total: 2 tasks

tasks > complete 0
✓ Task #0 marked as completed: Implement feature X

tasks > stats
Task Runner Statistics
═════════════════════════════════════
Total tasks:        2
Pending:            1
Completed:          1
High priority:      0
Total completed:    1
Completion rate:    50.0%
```

---

## 🎯 What These Examples Demonstrate

### Framework Features

| Feature | Calculator | File Manager | Task Runner |
|---------|-----------|--------------|-------------|
| **REPL Mode** | ✓ | ✓ | ✓ |
| **CLI Mode** | ✓ | ✓ | ✓ |
| **Context Management** | ✓ | ✓ | ✓ |
| **Argument Parsing** | ✓ | ✓ | ✓ |
| **Validation** | Basic | Path validation | Advanced |
| **Error Handling** | ✓ | ✓ | ✓ |
| **Options/Flags** | - | ✓ | ✓ |
| **Formatted Output** | ✓ | ✓ | ✓ |
| **State Persistence** | History | Statistics | Task list |

### Design Patterns

1. **Context Usage**
   - Each example defines a custom `ExecutionContext`
   - Context stores application state (history, tasks, etc.)
   - Safe downcasting with error handling

2. **Command Handlers**
   - Separate struct for each command
   - Implements `CommandHandler` trait
   - Clear separation of concerns

3. **Validation**
   - Built-in validation (types, ranges, paths)
   - Custom validation in handlers
   - Helpful error messages

4. **Utility Functions**
   - `parse_int`, `parse_float`, `parse_bool` for type conversion
   - `format_bytes`, `format_numbered_list` for output
   - `is_blank`, `normalize` for string handling
   - `get_extension`, `has_extension` for paths

---

## 🚀 Getting Started

### Prerequisites

```bash
# Ensure you're in the project root
cd dynamic-cli

# Create examples directory structure
mkdir -p examples/configs
```

### Copy Configuration Files

The examples require YAML configuration files:

```bash
# Copy the provided config files to examples/configs/
cp path/to/calculator.yaml examples/configs/
cp path/to/file_manager.yaml examples/configs/
cp path/to/task_runner.yaml examples/configs/
```

Or create them manually following the patterns shown in each example's documentation.

### Run Examples

```bash
# Build and run in REPL mode
cargo run --example simple_calculator
cargo run --example file_manager
cargo run --example task_runner

# Run in CLI mode with arguments
cargo run --example simple_calculator -- add 10 20
cargo run --example file_manager -- list .
cargo run --example task_runner -- add "My task" --priority high
```

---

## 📖 Learning Path

### Beginner: Simple Calculator

Start here to learn:
- Basic command structure
- Context management
- Simple argument parsing
- Error handling basics

**Key concepts:**
- How to define a context
- How to implement command handlers
- How to register handlers
- How to handle errors

### Intermediate: File Manager

Learn about:
- Path validation
- File system operations
- Formatted output
- Options and flags

**Key concepts:**
- Built-in validators
- Utility functions
- Option handling
- Error propagation

### Advanced: Task Runner

Master:
- Complex state management
- Custom validation
- Multiple data structures
- Statistics and reporting

**Key concepts:**
- Advanced context usage
- Custom validators
- Complex business logic
- User-friendly output

---

## 🔧 Customization

### Creating Your Own Example

1. **Define your context:**
```rust
#[derive(Default)]
struct MyContext {
    // Your application state
}

impl ExecutionContext for MyContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

2. **Implement command handlers:**
```rust
struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        // Your command logic
        Ok(())
    }
}
```

3. **Create configuration file:**
```yaml
metadata:
  version: "1.0.0"
  prompt: "myapp"

commands:
  - name: mycommand
    description: "My command"
    required: true
    arguments: []
    options: []
    implementation: "my_handler"

global_options: []
```

4. **Build and register:**
```rust
fn main() -> Result<()> {
    CliBuilder::new()
        .config_file("config.yaml")
        .context(Box::new(MyContext::default()))
        .register_handler("my_handler", Box::new(MyCommand))
        .build()?
        .run()
}
```

---

## 📊 Code Statistics

| Example | Lines | Commands | Handlers | Context Fields |
|---------|-------|----------|----------|----------------|
| **Calculator** | ~250 | 7 | 7 | 2 |
| **File Manager** | ~320 | 4 | 4 | 3 |
| **Task Runner** | ~420 | 6 | 6 | 3 |
| **Total** | ~990 | 17 | 17 | - |

---

## 🎓 Best Practices Demonstrated

1. **Error Handling**
   - Always use `Result<()>` return type
   - Provide context in error messages
   - Handle edge cases (division by zero, file not found)

2. **Validation**
   - Validate early in the handler
   - Use built-in validators when possible
   - Provide clear error messages

3. **User Experience**
   - Formatted output (tables, lists, colors via emojis)
   - Clear success/error messages
   - Helpful statistics and feedback

4. **Code Organization**
   - One struct per command handler
   - Clear separation of concerns
   - Well-documented code

5. **Context Management**
   - Store only necessary state
   - Use proper downcasting
   - Handle downcasting failures

---

## 🐛 Troubleshooting

### "Configuration file not found"

**Problem:** The example can't find its YAML configuration.

**Solution:**
```bash
# Ensure config files are in the right place
ls examples/configs/

# Should show:
# calculator.yaml
# file_manager.yaml
# task_runner.yaml
```

### "Handler not found"

**Problem:** Handler name in YAML doesn't match registered name.

**Solution:** Check that the `implementation` field in YAML matches the name in `.register_handler()`:
```rust
.register_handler("my_handler", ...)  // Must match
```
```yaml
implementation: "my_handler"  // Must match
```

### "Context downcast failed"

**Problem:** Wrong context type in handler.

**Solution:** Ensure you're downcasting to the correct context type:
```rust
let ctx = dynamic_cli::context::downcast_mut::<YourContext>(context)
    .ok_or_else(|| ...)?;
```

---

## 📚 Additional Resources

- **API Documentation:** `cargo doc --open`
- **Framework Guide:** See main README.md
- **Configuration Format:** See API_SPEC.md

---

## 🎉 Next Steps

After exploring these examples:

1. Modify them to add new features
2. Create your own application
3. Combine patterns from multiple examples
4. Share your creations!

**Happy coding with dynamic-cli!** 🚀
