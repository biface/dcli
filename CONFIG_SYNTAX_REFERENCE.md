# Configuration File Syntax Reference - dynamic-cli

**Version**: 1.0  
**Last Updated**: 2026-01-11  
**Format**: YAML or JSON

---

## Table of Contents

- [Overview](#overview)
- [File Format](#file-format)
- [Root Structure](#root-structure)
- [Metadata Section](#metadata-section)
- [Global Options](#global-options)
- [Commands Section](#commands-section)
- [Command Definition](#command-definition)
- [Arguments](#arguments)
- [Options](#options)
- [Argument Types](#argument-types)
- [Validation Rules](#validation-rules)
- [Complete Example](#complete-example)
- [Best Practices](#best-practices)

---

## Overview

The configuration file defines all CLI commands and REPL behavior for applications built with `dynamic-cli`.

**Key Features**:
- Define commands without writing code
- Support for positional arguments and options
- Automatic validation and type checking
- Extensible with custom handlers

**Supported Formats**:
- YAML (`.yaml`, `.yml`) - Recommended for readability
- JSON (`.json`) - Compatible with existing tools

---

## File Format

### YAML Example

```yaml
metadata:
  version: "1.0.0"
  prompt: "myapp"
  prompt_suffix: " > "

global_options:
  - name: "verbose"
    # ...

commands:
  - name: "init"
    # ...
```

### JSON Example

```json
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "myapp",
    "prompt_suffix": " > "
  },
  "global_options": [
    {
      "name": "verbose"
    }
  ],
  "commands": [
    {
      "name": "init"
    }
  ]
}
```

**Note**: YAML examples are used throughout this document for readability.

---

## Root Structure

The configuration file has **three main sections**:

```yaml
metadata:          # Application metadata
  # ...

global_options:    # Options available for ALL commands
  # ...

commands:          # List of available commands
  # ...
```

**All three sections are required**, even if `global_options` is empty.

---

## Metadata Section

Application-level information.

### Structure

```yaml
metadata:
  version: string        # Required - Configuration version
  prompt: string         # Required - REPL prompt text
  prompt_suffix: string  # Required - Suffix after prompt (e.g., " > ")
```

### Fields

| Field           | Type   | Required  | Description                                                  |
|-----------------|--------|-----------|--------------------------------------------------------------|
| `version`       | string | ✅ Yes     | Configuration file version (semantic versioning recommended) |
| `prompt`        | string | ✅ Yes     | Text displayed in REPL mode (e.g., "myapp", "rpn")           |
| `prompt_suffix` | string | ✅ Yes     | Text after prompt (typically `" > "` or `"$ "`)              |

### Example

```yaml
metadata:
  version: "1.0.0"
  prompt: "rpn"
  prompt_suffix: " > "
```

**REPL Display**:
```
rpn > _
```

---

## Global Options

Options that are **available for ALL commands** in the application.

### Structure

```yaml
global_options:
  - name: string              # Required - Unique identifier
    short: string             # Optional - Single character (e.g., "v")
    long: string              # Optional - Long form (e.g., "verbose")
    type: ArgumentType        # Required - Data type
    required: boolean         # Required - Must be provided?
    default: string           # Optional - Default value if not provided
    description: string       # Required - Help text
    choices: [string]         # Optional - List of valid values
```

### Fields

| Field         | Type    | Required  | Description                                            |
|---------------|---------|-----------|--------------------------------------------------------|
| `name`        | string  | ✅ Yes     | Internal identifier (used in handlers)                 |
| `short`       | string  | ⬜ No      | Short form flag (single character, e.g., "v" for `-v`) |
| `long`        | string  | ⬜ No      | Long form flag (e.g., "verbose" for `--verbose`)       |
| `type`        | string  | ✅ Yes     | One of: `string`, `integer`, `float`, `bool`, `path`   |
| `required`    | boolean | ✅ Yes     | If `true`, must be provided by user                    |
| `default`     | string  | ⬜ No      | Default value (as string, will be parsed to type)      |
| `description` | string  | ✅ Yes     | User-facing help text                                  |
| `choices`     | array   | ⬜ No      | Restrict values to specific list                       |

### Examples

**Verbose Flag**:
```yaml
- name: "verbose"
  short: "v"
  long: "verbose"
  type: "bool"
  required: false
  default: "false"
  description: "Enable verbose output"
```

**Usage**:
```bash
myapp -v command
myapp --verbose command
```

**Output Directory**:
```yaml
- name: "output"
  short: "o"
  long: "output"
  type: "path"
  required: false
  description: "Output directory for results"
```

**Usage**:
```bash
myapp -o /tmp/results command
myapp --output ./output command
```

**Format Selection**:
```yaml
- name: "format"
  short: "f"
  long: "format"
  type: "string"
  required: false
  default: "text"
  description: "Output format"
  choices: ["text", "json", "yaml"]
```

**Usage**:
```bash
myapp --format json command
myapp -f yaml command
```

### Notes

- At least one of `short` or `long` must be provided
- Both can be provided for maximum flexibility
- Global options are passed to **every** command handler

---

## Commands Section

List of available commands in the application.

### Structure

```yaml
commands:
  - name: string                    # Required - Command name
    aliases: [string]               # Required - Alternative names (can be empty)
    description: string             # Required - Help text
    required: boolean               # Required - Must be executed in workflow?
    arguments: [ArgumentDefinition] # Required - Positional arguments (can be empty)
    options: [OptionDefinition]     # Required - Command-specific options (can be empty)
    implementation: string          # Required - Handler function name
```

### Fields

| Field            | Type    | Required  | Description                                                 |
|------------------|---------|-----------|-------------------------------------------------------------|
| `name`           | string  | ✅ Yes     | Primary command name (no spaces, lowercase recommended)     |
| `aliases`        | array   | ✅ Yes     | Alternative names (e.g., `["quit", "q"]` for exit)          |
| `description`    | string  | ✅ Yes     | User-facing help text                                       |
| `required`       | boolean | ✅ Yes     | If `true`, command must be executed in application workflow |
| `arguments`      | array   | ✅ Yes     | List of positional arguments (use `[]` if none)             |
| `options`        | array   | ✅ Yes     | Command-specific options (use `[]` if none)                 |
| `implementation` | string  | ✅ Yes     | Identifier for command handler (referenced in code)         |

### Example

```yaml
commands:
  - name: "input"
    aliases: ["load", "open"]
    description: "Load a configuration file"
    required: true
    arguments:
      - name: "file"
        type: "path"
        required: true
        description: "Path to input file"
        validation:
          - must_exist: true
          - extensions: [".yaml", ".json"]
    options: []
    implementation: "load_config"
```

**Usage**:
```bash
myapp input config.yaml
myapp load config.yaml
myapp open config.json
```

---

## Command Definition

Detailed breakdown of a single command.

### Full Structure

```yaml
- name: "command-name"
  aliases: ["alt1", "alt2"]
  description: "What this command does"
  required: false
  
  arguments:
    - name: "arg1"
      type: "string"
      required: true
      description: "First argument"
      validation: []
    
    - name: "arg2"
      type: "integer"
      required: false
      description: "Optional second argument"
      validation:
        - min: 1
          max: 100
  
  options:
    - name: "option1"
      short: "o"
      long: "option"
      type: "bool"
      required: false
      default: "false"
      description: "Enable option"
      choices: []
  
  implementation: "handler_name"
```

### Field Details

**`name`** (string, required):
- Primary identifier for the command
- Used in CLI: `myapp <name>`
- Convention: lowercase, no spaces, use hyphens for multi-word (e.g., `run-simulation`)

**`aliases`** (array of strings, required):
- Alternative names for the same command
- Can be empty: `aliases: []`
- Example: `aliases: ["quit", "q", "exit"]`
- All aliases invoke the same handler

**`description`** (string, required):
- User-facing help text
- Displayed in `help` command output
- Should be concise but clear (one sentence recommended)

**`required`** (boolean, required):
- `true`: Command must be executed in application workflow
- `false`: Command is optional
- Application logic must enforce this (not automatic)

**`arguments`** (array, required):
- List of positional arguments
- Order matters (parsed left-to-right)
- Can be empty: `arguments: []`
- See [Arguments](#arguments) section for details

**`options`** (array, required):
- Command-specific flags/options
- Independent of `global_options`
- Can be empty: `options: []`
- See [Options](#options) section for details

**`implementation`** (string, required):
- Identifier used in Rust code to link to handler
- Convention: snake_case
- Example: `"load_config"` maps to `LoadConfigHandler`

---

## Arguments

Positional arguments for commands.

### Structure

```yaml
arguments:
  - name: string              # Required - Identifier
    type: ArgumentType        # Required - Data type
    required: boolean         # Required - Must be provided?
    description: string       # Required - Help text
    validation: [Rule]        # Required - Validation rules (can be empty)
```

### Fields

| Field         | Type    | Required  | Description                                          |
|---------------|---------|-----------|------------------------------------------------------|
| `name`        | string  | ✅ Yes     | Internal identifier for argument                     |
| `type`        | string  | ✅ Yes     | One of: `string`, `integer`, `float`, `bool`, `path` |
| `required`    | boolean | ✅ Yes     | If `true`, user must provide this argument           |
| `description` | string  | ✅ Yes     | Help text for user                                   |
| `validation`  | array   | ✅ Yes     | Validation rules (use `[]` if none)                  |

### Examples

**File Path Argument**:
```yaml
- name: "file"
  type: "path"
  required: true
  description: "Input file path"
  validation:
    - must_exist: true
    - extensions: [".yaml", ".json", ".yml"]
```

**Usage**: `myapp load config.yaml`

**Numeric Range**:
```yaml
- name: "iterations"
  type: "integer"
  required: true
  description: "Number of iterations"
  validation:
    - min: 1
      max: 10000
```

**Usage**: `myapp simulate 5000`

**Optional String**:
```yaml
- name: "output-name"
  type: "string"
  required: false
  description: "Output file name"
  validation: []
```

**Usage**: 
```bash
myapp export result.csv    # Provides name
myapp export               # Uses default
```

### Order Matters

Arguments are parsed **in order** from left to right:

```yaml
arguments:
  - name: "source"      # First positional
    type: "path"
    required: true
  
  - name: "destination" # Second positional
    type: "path"
    required: true
```

**Usage**: `myapp copy source.txt dest.txt`

---

## Options

Command-specific flags and options.

### Structure

```yaml
options:
  - name: string              # Required - Identifier
    short: string             # Optional - Single character
    long: string              # Optional - Long form
    type: ArgumentType        # Required - Data type
    required: boolean         # Required - Must be provided?
    default: string           # Optional - Default value
    description: string       # Required - Help text
    choices: [string]         # Optional - Valid values
```

### Fields

Same as [Global Options](#global-options) fields.

### Examples

**Boolean Flag**:
```yaml
- name: "recursive"
  short: "r"
  long: "recursive"
  type: "bool"
  required: false
  default: "false"
  description: "Process directories recursively"
```

**Usage**:
```bash
myapp process -r /path
myapp process --recursive /path
```

**Integer Option**:
```yaml
- name: "threads"
  short: "t"
  long: "threads"
  type: "integer"
  required: false
  default: "4"
  description: "Number of parallel threads"
```

**Usage**:
```bash
myapp simulate --threads 8
myapp simulate -t 8
myapp simulate              # Uses default: 4
```

**Choice Restriction**:
```yaml
- name: "level"
  short: "l"
  long: "level"
  type: "string"
  required: false
  default: "info"
  description: "Logging level"
  choices: ["debug", "info", "warn", "error"]
```

**Usage**:
```bash
myapp run --level debug
myapp run -l warn
myapp run                   # Uses default: info
myapp run -l invalid        # ERROR: invalid choice
```

---

## Argument Types

Supported data types for arguments and options.

### Available Types

| Type      | Description         | Example Values             | Rust Type  |
|-----------|---------------------|----------------------------|------------|
| `string`  | Text value          | `"hello"`, `"config.yaml"` | `String`   |
| `integer` | Whole number        | `42`, `-10`, `0`           | `i64`      |
| `float`   | Decimal number      | `3.14`, `-0.5`, `1.0`      | `f64`      |
| `bool`    | Boolean flag        | `true`, `false`            | `bool`     |
| `path`    | File/directory path | `"./file.txt"`, `"/tmp"`   | `PathBuf`  |

### Type Parsing

**`string`**:
- Accepts any text
- No parsing required
- Example: `name: "John Doe"`

**`integer`**:
- Must be valid integer
- Supports negative values
- Range: `-9,223,372,036,854,775,808` to `9,223,372,036,854,775,807`
- Example: `count: 42`

**`float`**:
- Must be valid decimal number
- Supports scientific notation
- Example: `value: 3.14` or `value: 1.5e-3`

**`bool`**:
- For flags: presence = `true`, absence = `false`
- For values: `"true"`, `"false"`, `"yes"`, `"no"`, `"1"`, `"0"`
- Case-insensitive
- Example: `--verbose` (true) vs no flag (false)

**`path`**:
- File or directory path
- Can be relative or absolute
- Validation can check existence
- Example: `file: "./data/input.csv"`

---

## Validation Rules

Constraints applied to arguments and options.

### Available Rules

#### 1. File/Directory Existence

```yaml
validation:
  - must_exist: true
```

**Applies to**: `path` type  
**Effect**: Checks if file/directory exists before execution  
**Error if**: Path does not exist

#### 2. File Extension

```yaml
validation:
  - extensions: [".txt", ".csv", ".json"]
```

**Applies to**: `path` type (files only)  
**Effect**: Validates file has one of the specified extensions  
**Error if**: File extension not in list

#### 3. Numeric Range

```yaml
validation:
  - min: 1
    max: 100
```

**Applies to**: `integer`, `float` types  
**Effect**: Value must be within range (inclusive)  
**Fields**:
- `min` (optional): Minimum value
- `max` (optional): Maximum value
- Both can be omitted for one-sided bounds

**Examples**:
```yaml
# Only minimum
- min: 0

# Only maximum
- max: 100

# Both
- min: 1
  max: 10
```

### Combining Rules

Multiple rules can be applied to a single argument:

```yaml
validation:
  - must_exist: true
  - extensions: [".yaml", ".json"]
```

**Effect**: File must exist AND have .yaml or .json extension

### No Validation

Use empty array if no validation needed:

```yaml
validation: []
```

---

## Complete Example

Full configuration file for a data processing application.

```yaml
# ═══════════════════════════════════════════════════════════
# Data Processor CLI Configuration
# ═══════════════════════════════════════════════════════════

metadata:
  version: "1.0.0"
  prompt: "dataproc"
  prompt_suffix: " > "

# ───────────────────────────────────────────────────────────
# GLOBAL OPTIONS (available for all commands)
# ───────────────────────────────────────────────────────────
global_options:
  - name: "verbose"
    short: "v"
    long: "verbose"
    type: "bool"
    required: false
    default: "false"
    description: "Enable verbose output"
  
  - name: "output-dir"
    short: "o"
    long: "output"
    type: "path"
    required: false
    description: "Output directory for results"
  
  - name: "format"
    short: "f"
    long: "format"
    type: "string"
    required: false
    default: "text"
    description: "Output format"
    choices: ["text", "json", "yaml"]

# ───────────────────────────────────────────────────────────
# COMMANDS
# ───────────────────────────────────────────────────────────
commands:
  
  # ═════════════════════════════════════════════════════════
  # Command: load
  # ═════════════════════════════════════════════════════════
  - name: "load"
    aliases: ["open", "input"]
    description: "Load a data file"
    required: true
    
    arguments:
      - name: "file"
        type: "path"
        required: true
        description: "Path to data file"
        validation:
          - must_exist: true
          - extensions: [".csv", ".json", ".yaml"]
    
    options: []
    
    implementation: "load_data"
  
  # ═════════════════════════════════════════════════════════
  # Command: process
  # ═════════════════════════════════════════════════════════
  - name: "process"
    aliases: ["run"]
    description: "Process loaded data"
    required: false
    
    arguments: []
    
    options:
      - name: "threads"
        short: "t"
        long: "threads"
        type: "integer"
        required: false
        default: "4"
        description: "Number of parallel threads"
        choices: []
      
      - name: "batch-size"
        short: "b"
        long: "batch"
        type: "integer"
        required: false
        default: "1000"
        description: "Batch size for processing"
        choices: []
    
    implementation: "process_data"
  
  # ═════════════════════════════════════════════════════════
  # Command: export
  # ═════════════════════════════════════════════════════════
  - name: "export"
    aliases: ["save"]
    description: "Export results to file"
    required: false
    
    arguments:
      - name: "output-file"
        type: "path"
        required: true
        description: "Output file path"
        validation: []
    
    options:
      - name: "compress"
        short: "c"
        long: "compress"
        type: "bool"
        required: false
        default: "false"
        description: "Compress output file"
        choices: []
    
    implementation: "export_results"
  
  # ═════════════════════════════════════════════════════════
  # Command: help
  # ═════════════════════════════════════════════════════════
  - name: "help"
    aliases: ["h", "?"]
    description: "Show help information"
    required: false
    
    arguments:
      - name: "command"
        type: "string"
        required: false
        description: "Command to get help for"
        validation: []
    
    options: []
    
    implementation: "show_help"
  
  # ═════════════════════════════════════════════════════════
  # Command: exit
  # ═════════════════════════════════════════════════════════
  - name: "exit"
    aliases: ["quit", "q"]
    description: "Exit the application"
    required: false
    
    arguments: []
    options: []
    
    implementation: "exit_app"
```

### Usage Examples

**CLI Mode**:
```bash
# With global options
dataproc --verbose load data.csv
dataproc -v -o ./results process --threads 8
dataproc --format json export output.json

# Without global options
dataproc load data.csv
dataproc process --threads 8 --batch 2000
dataproc export output.csv --compress
```

**REPL Mode**:
```
dataproc > load data.csv
Data loaded successfully.

dataproc > process --threads 8
Processing... Done.

dataproc > export results.csv
Results exported to results.csv

dataproc > help export
Command: export
Description: Export results to file
...

dataproc > exit
Goodbye!
```

---

## Best Practices

### 1. Naming Conventions

**Commands**:
- Use lowercase
- Use hyphens for multi-word: `run-simulation` (not `runSimulation` or `run_simulation`)
- Keep names concise but descriptive
- Provide meaningful aliases: `["quit", "q", "exit"]`

**Arguments/Options**:
- Use lowercase with hyphens: `output-dir` (not `outputDir`)
- Be consistent across the application
- Use short forms wisely: single character, commonly recognized (`-v` for verbose)

### 2. Required vs Optional

**Mark as `required: true` when**:
- Command MUST be executed for application workflow
- Argument MUST be provided for command to work
- Option is essential (rare - consider if it should be an argument instead)

**Mark as `required: false` when**:
- Command is optional convenience feature
- Argument has sensible default behavior
- Option enhances but doesn't block functionality

### 3. Validation

**Always validate when**:
- Accepting file paths (check existence, extensions)
- Accepting numeric values with constraints (use ranges)
- Accepting enums (use `choices`)

**Example**:
```yaml
# Good: Comprehensive validation
validation:
  - must_exist: true
  - extensions: [".csv", ".json"]
```
```yaml
# Bad: No validation for file input
validation: []
```

### 4. Descriptions

**Write clear, concise descriptions**:
- ✅ Good: "Load a configuration file"
- ❌ Bad: "Loads stuff"
- ✅ Good: "Number of parallel threads (1-16)"
- ❌ Bad: "threads"

### 5. Defaults

**Provide sensible defaults for optional fields**:
```yaml
# Good: Sensible default
- name: "threads"
  default: "4"  
```
```yaml
# Bad: Required but should have default
- name: "threads"
  required: true  # User must always specify
```

### 6. Global vs Command Options

**Use `global_options` for**:
- Verbose/debug flags
- Output format selection
- Configuration file paths
- Logging levels

**Use command `options` for**:
- Command-specific behavior
- Parameters that only make sense for one command

### 7. Aliases

**Provide helpful aliases**:
```yaml
# Good: Common shortcuts
aliases: ["quit", "q", "exit"]
```
```yaml

# Good: Alternative names
aliases: ["load", "open", "input"]
```
```yaml
# Avoid: Confusing or too many
aliases: ["a", "b", "c", "xyz123"]
```

### 8. Implementation Names

**Use clear, consistent naming**:
```yaml
# Good: Clear purpose
implementation: "load_config"
```
```yaml
implementation: "run_simulation"
```
```yaml
# Bad: Vague
implementation: "handler1"
```
```yaml
implementation: "do_stuff"
```

### 9. File Organization

For large applications, consider:
- One configuration file per mode (cli.yaml, repl.yaml)
- Shared commands in separate included file
- Environment-specific overrides (dev.yaml, prod.yaml)

### 10. Documentation

**Comment complex sections**:
```yaml
# ═══════════════════════════════════════════════════════════
# SIMULATION COMMANDS
# These commands control the numerical simulation engine
# ═══════════════════════════════════════════════════════════
commands:
  - name: "simulate"
    # Advanced users can override time step
    options:
      - name: "dt"
        description: "Time step in seconds (experts only)"
```

---

## JSON Equivalent

The complete example above in JSON format:

```json
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "dataproc",
    "prompt_suffix": " > "
  },
  "global_options": [
    {
      "name": "verbose",
      "short": "v",
      "long": "verbose",
      "type": "bool",
      "required": false,
      "default": "false",
      "description": "Enable verbose output"
    }
  ],
  "commands": [
    {
      "name": "load",
      "aliases": ["open", "input"],
      "description": "Load a data file",
      "required": true,
      "arguments": [
        {
          "name": "file",
          "type": "path",
          "required": true,
          "description": "Path to data file",
          "validation": [
            {"must_exist": true},
            {"extensions": [".csv", ".json"]}
          ]
        }
      ],
      "options": [],
      "implementation": "load_data"
    }
  ]
}
```

---

## Summary

This reference covers all syntax elements for `dynamic-cli` configuration files:

✅ **Metadata** - Application information  
✅ **Global Options** - Options for all commands  
✅ **Commands** - Individual command definitions  
✅ **Arguments** - Positional parameters  
✅ **Options** - Named flags and parameters  
✅ **Types** - Data type specifications  
✅ **Validation** - Constraint rules  
✅ **Examples** - Real-world usage  
✅ **Best Practices** - Recommendations  

For implementation details, see the `dynamic-cli` API documentation.

---

**Version**: 1.0  
**Framework**: dynamic-cli  
**Date**: 2026-01-11
