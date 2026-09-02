# Configuration File Syntax Reference - dynamic-cli

**Version**: 1.0  
**Last Updated**: 2026-09-01  
**Format**: YAML or JSON

---

## Table of Contents

- [Overview](#overview)
- [File Format](#file-format)
- [Root Structure](#root-structure)
- [Metadata Section](#metadata-section)
  - [Multi-line Continuation (REPL)](#multi-line-continuation-repl)
- [Global Options](#global-options)
- [Commands Section](#commands-section)
- [Command Definition](#command-definition)
  - [Command Chaining](#command-chaining)
- [Arguments](#arguments)
- [Options](#options)
- [Repeatable Options](#repeatable-options)
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
  prompt_suffix: string  # Optional - Suffix after prompt (default: " > ")
```

### Fields

| Field           | Type   | Required  | Description                                                  |
|-----------------|--------|-----------|--------------------------------------------------------------|
| `version`       | string | ✅ Yes     | Configuration file version (semantic versioning recommended) |
| `prompt`        | string | ✅ Yes     | Text displayed in REPL mode (e.g., "myapp", "rpn")           |
| `prompt_suffix` | string | ❌ No      | Text after prompt (default: `" > "`; typically `" > "` or `"$ "`). Since v0.9.0 ([#67](https://github.com/biface/dcli/issues/67)), this value is what `ReplInterface` actually renders — an earlier version hardcoded `" > "` regardless of this field. |

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

### Multi-line Continuation (REPL)

*Since v0.9.0 ([DD-027](https://github.com/biface/dcli/issues/48)).*

In interactive REPL mode, a line ending in a trailing `\` is not
dispatched immediately: the marker is stripped, the line is buffered,
the prompt switches to a distinct continuation prompt derived from
`prompt`/`prompt_suffix` above, and another line is read. The first
line that does **not** end in `\` completes the command — every
buffered fragment plus this final line are joined with a single space
and dispatched exactly once, mirroring the familiar Unix shell
convention.

This is independent of [Command Chaining](#command-chaining) (DD-026):
chaining sequences several *complete* commands in one invocation; this
feature lets a single command's *options* span several REPL input
lines. The two compose without special-casing each other.

```
rpn > push \
...> 0
```

- Only a trailing `\` triggers accumulation — no alternative explicit
  terminator (e.g. `;`) is recognized.
- The reconstructed line (fragments joined by a single space, `\`
  markers removed) is what reaches REPL history — never a raw partial
  fragment.
- `Ctrl+C` while accumulating discards the buffer and returns to the
  normal prompt, the same as abandoning a continued line in a shell.

**Continuation prompt derivation**: the app-name segment of `prompt`
is replaced with `...`, and `prompt_suffix` is appended with its
leading whitespace stripped — so it reads as one continuous mark
against the separator rather than leaving a stray gap:

| `prompt` | `prompt_suffix` | Main prompt | Continuation prompt (default) |
|---|---|---|---|
| `rpn` | `" > "` (default) | `rpn > ` | `...> ` |
| `rpn` | `" $ "` | `rpn $ ` | `...$ ` |

The base segment (`...` above) can be overridden from Rust via
[`CliBuilder::prompt_multiline()`](https://docs.rs/dynamic-cli/latest/dynamic_cli/struct.CliBuilder.html#method.prompt_multiline)
— there is no YAML field for it, since it is a display detail of the
interactive REPL rather than part of the command schema. An override
replaces only the base; `prompt_suffix` is still appended, **without**
stripping its leading whitespace this time — the override is the
caller's own spacing choice:

```rust
CliBuilder::new()
    .prompt("rpn")
    .prompt_multiline("_ _ _")
    // prompt_suffix == " $ " → continuation prompt: "_ _ _ $ "
```

**Known constraint — no configurable abort/continue policy**: a
command signalled complete (no trailing `\`) but still missing a
required argument or option fails immediately with today's exact
`MissingArgument`/`MissingOption` error — there is no re-prompt loop.
Deliberate, not an oversight: CLI one-shot mode
(`CliInterface::run()`), `run_script()`, and `:load` have no live
operator to re-prompt, so they always abort on an incomplete command;
the REPL keeps the same abort behaviour for CLI/REPL parity. Not
scheduled to change; revisit only if a concrete need appears.

**Scope**: applies only to `ReplInterface::run()` (interactive REPL).
In CLI one-shot mode, the invoking shell has already collapsed any
`\`-continued line before `dynamic-cli` ever sees its `argv` — nothing
to implement there. `run_script()` and the REPL's `:load`
meta-command are **not** affected: each line is dispatched
independently, and a trailing `\` in a script line is read like any
other token.

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
  continue_on_failure: false
  requires_success: false
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

**`continue_on_failure`** (boolean, optional, default `false`):
- Governs command chaining (v0.8.0, [DD-026](https://github.com/biface/dcli/issues/52)) — has no effect when this command is not part of a chain.
- `false` (default): if this command fails, the chain stops
  immediately — identical to today's single-command behaviour, applied
  per segment.
- `true`: if this command fails, the chain-position error is reported
  and execution proceeds to the next segment. Intended for genuinely
  optional steps (e.g. a non-critical secondary export).

**`requires_success`** (boolean, optional, default `false`):
- Also part of command chaining ([DD-026](https://github.com/biface/dcli/issues/52)).
  **Not related to `required` above** — `required` is a startup-time
  check ("a handler must be registered"), `requires_success` is a
  per-invocation chain guard evaluated at dispatch time.
- `false` (default): this command runs regardless of any earlier
  failure in the chain (moot unless an earlier command's
  `continue_on_failure` is `true`).
- `true`: if any earlier command in the chain has already failed
  (regardless of *that* failure's own `continue_on_failure` value),
  this command is **skipped** — not executed, not counted as an
  additional failure.

> **Sync vs. async is not a config concern (ref. #8).** This field is
> agnostic to whether the handler behind it is sync or async — there is no
> `async: true` flag, and there won't be one. Whether a command executes
> synchronously or asynchronously is decided entirely on the Rust side, by
> which method the application calls when wiring up the handler:
> ```rust, ignore
> // Sync handler
> .register_sync_handler("load_config", Box::new(LoadConfigHandler))
> // Async handler — same implementation name convention, same YAML
> .register_async_handler("fetch_remote", Box::new(FetchRemoteHandler))
> ```
> `CliBuilder::build()` resolves each `implementation` name against
> whichever kind was registered. Adding a config-level flag would create a
> second source of truth that could disagree with the actual Rust
> registration — the binding alone is authoritative, deliberately.

### Command Chaining

*Since v0.8.0 ([DD-026](https://github.com/biface/dcli/issues/52)).*

A single CLI invocation (or a [`run_script()`](https://docs.rs/dynamic-cli/latest/dynamic_cli/interface/struct.CliInterface.html#method.run_script)
line) can chain more than one command: once a command's declared arity
is exhausted, the next bare token is checked against the command
registry — if it resolves to a command name (or alias), it starts the
next command in the chain; if it doesn't, the line is rejected exactly
as an over-long single command always was. There is no separator token
(`;`, `&&`, `|`) — chaining is a consequence of exhausted arity alone,
not a distinct syntax to opt into. `continue_on_failure` and
`requires_success`, described above, govern per-command behaviour
within a chain.

#### Segmentation Rule

For each command in the sequence:

1. Its options (including repeatable-option occurrences, DD-024) and
   positional arguments are consumed up to its declared arity.
2. The next token is looked up against the command registry.
   - Resolves to a command name or alias → starts the next segment.
   - Doesn't resolve → the same "too many arguments" error a single,
     non-chained command would raise — no behaviour change for a
     genuinely too-long single command.

#### Handler-Side Consistency Check

*Since v0.9.0 ([DD-028](https://github.com/biface/dcli/issues/64)).*

`continue_on_failure` is set in YAML, independently of the handler's
own code — nothing ties the two together by default, so a handler
written to assume its failures always stop the chain could silently
end up configured with `continue_on_failure: true` (or vice versa).
There is no new YAML field for this: it's a Rust-side consistency
check layered on the existing field, checked once, at registration
time, before the application ever runs.

A handler author can declare an expectation by overriding
`expected_fault_tolerance()` on `CommandHandler` or
`AsyncCommandHandler` (both default to `None` — no opinion, no check):

```rust
impl CommandHandler for SolveCommand {
    fn execute(&self, context: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
        // ...
    }

    fn expected_fault_tolerance(&self) -> Option<bool> {
        Some(false) // this handler was written assuming it always stops the chain
    }
}
```

`CommandRegistry::register_sync`/`register_async` compare this value
(when `Some`) against the command's configured `continue_on_failure`,
and reject registration with `RegistryError::FaultToleranceMismatch`
on a contradiction — catching a misconfigured YAML value at startup
instead of at first use in a chain. A handler that never overrides the
method (the default, `None`) is entirely unaffected, regardless of
what `continue_on_failure` is set to.

This does **not** verify the handler's internal logic — that its
`execute()` actually returns `Err(...)` when it's supposed to. That
remains outside any signature-level check, covered by the
application's own tests.

#### Error and Skip Reporting

- A failing command is reported as `Error in command {n}/{total}
  ('{name}'): <normal error output>` — the chain position is prefixed;
  the rest of the message is unchanged from a single-command failure.
- A skipped command (via `requires_success`) is reported as `Skipped:
  command {n}/{total} ('{name}') — a preceding command failed`.
- The process exit code always reflects the **first** failure
  encountered in the chain — the "triggering" failure — even if a
  later `continue_on_failure: true` command also fails, or a
  downstream command is skipped as a result.

#### Known Limitation

If a command line supplies one token more than that command's declared
arity, and that leftover token happens to also be a registered command
name, it is silently read as the start of the next segment instead of
raising an error — segmentation cannot distinguish "a stray extra
value" from "the next command" once arity is exhausted. There is no
`--`-style escape to force a value to be treated as positional rather
than a command boundary. Not scheduled for a fix; revisit only if a
concrete need appears.

#### Scope

- [`CliInterface::run()`](https://docs.rs/dynamic-cli/latest/dynamic_cli/interface/struct.CliInterface.html#method.run)
  (CLI one-shot) and `run_script()` inherit chaining automatically —
  both dispatch through the same internal resolution path.
- The REPL's `:load` meta-command does **not** chain: each loaded line
  still goes through the REPL's own single-command path, unchanged.

#### Example

```yaml
commands:
  - name: "configure"
    description: "Register an input source"
    aliases: []
    required: false
    arguments:
      - name: "path"
        type: "path"
        required: true
        description: "Source file"
        validation: []
    options: []
    implementation: "configure_handler"

  - name: "solve"
    description: "Run once all sources are configured"
    aliases: []
    required: false
    arguments: []
    options: []
    implementation: "solve_handler"
    requires_success: true
```

**Usage**:
```bash
myapp configure model.yml configure scenario.yml solve
# "configure" runs twice, then "solve" — only if both configure steps succeeded
```

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

## Repeatable Options

*Since v0.6.0 ([DD-024](https://github.com/biface/dcli/issues/21)).*

A repeatable option can appear more than once on the same command line.
Each occurrence starts with a **discriminant** — a bare token immediately
after the flag, drawn from `choices` — followed by zero or more `key=value`
sub-parameters. `dynamic-cli` accumulates every occurrence in command-line
order; the handler receives them all.

### Structure

```yaml
options:
  - name: string                          # Required
    long: string                          # Optional
    short: string                         # Optional
    option_type: ArgumentType             # Required
    required: boolean                     # Required
    description: string                   # Required
    choices: [string]                     # Required when repeatable — valid discriminants
    repeatable: true                      # Required to enable this behaviour
    option_parameters:                    # Required — one entry per discriminant
      <discriminant>:
        - name: string
          arg_type: ArgumentType
          required: boolean
          description: string
          validation: []                  # Optional
          secure: false                   # Optional
```

### Rules

- `choices` must be non-empty and lists the valid discriminants.
- `option_parameters` keys must match `choices` **exactly** — no orphan
  key, no discriminant left undeclared. Config loading fails otherwise.
- Each `option_parameters[discriminant]` entry is a regular
  [`ArgumentDefinition`](#arguments) (same fields as positional
  `arguments:`), but only `name` / `arg_type` / `required` / `description`
  / `validation` / `secure` are meaningful — ordering has no effect, since
  sub-parameters are matched by key, not position.
- `default` is **not allowed** when `repeatable: true`: the option's
  absence already means zero occurrences, so an implicit default would be
  ambiguous.
- `option_parameters` on an option with `repeatable: false` (or absent) is
  a config error.
- Two occurrences of the same discriminant with different `key=value`
  pairs are both kept. Two occurrences that are **strictly identical**
  (same discriminant, same key=value pairs) are rejected — this is the
  only overlap the framework can detect without domain knowledge;
  partially-overlapping occurrences (same `file`, different `resolution`,
  say) are accepted and left to the handler to reconcile.

### Example

```yaml
- name: "output"
  long: "output"
  option_type: "string"
  required: false
  description: "Write results in one or more output kinds"
  choices: ["csv", "plot"]
  repeatable: true
  option_parameters:
    csv:
      - name: "file"
        arg_type: "path"
        required: true
        description: "Destination CSV file"
      - name: "resolution"
        arg_type: "integer"
        required: false
        description: "Time-step resolution"
    plot:
      - name: "file"
        arg_type: "path"
        required: true
        description: "Destination image file"
```

**Usage**:
```bash
myapp export --output csv file=results.csv resolution=100
myapp export --output csv file=results.csv --output plot file=chart.png
myapp export --output csv file=a.csv --output csv file=b.csv   # both kept — different `file`
myapp export --output csv file=a.csv --output csv file=a.csv   # ERROR: DuplicateOptionOccurrence
myapp export --output json file=out.json                       # ERROR: UnknownDiscriminant
myapp export --output csv                                      # ERROR: MissingRequiredOptionParameter (file)
myapp export --output csv file=a.csv depth=3                   # ERROR: UnknownOptionParameter (depth)
```

### Handler side

`CommandHandler::execute` (and `validate`) receive a `ParsedArgs` value
(see the [`executor` module docs](https://docs.rs/dynamic-cli/latest/dynamic_cli/executor/))
rather than a flat `HashMap<String, String>`, precisely so a repeatable option's occurrences
can be represented alongside scalar options:

```rust
let occurrences = args.get_repeated("output").unwrap_or(&[]);
for occ in occurrences {
    match occ.discriminant.as_str() {
        "csv" => { /* occ.params.get("file"), occ.params.get("resolution") */ }
        "plot" => { /* occ.params.get("file") */ }
        _ => unreachable!(), // config validation already rejected other discriminants
    }
}
```

See `CHANGELOG.md` (v0.6.0) for the full `HashMap<String, String>` →
`ParsedArgs` migration path for existing handlers.

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
