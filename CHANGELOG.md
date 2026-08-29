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

## [0.8.0] "Chain Reaction" - 2026-08-29

**Theme**: CLI: repeatable commands — Multi-Command Chaining
**Decision**: [DD-026](https://github.com/biface/dcli/issues/52)

### Added

#### CLI-mode multi-command chaining
- **Multi-command chaining** (#52–#56): a single CLI invocation (or
  `run_script()` line) can now chain more than one command with no
  separator token (`;`, `&&`, `|`) — the boundary between commands is
  detected purely from arity exhaustion: once a command's declared
  positional arguments and options are consumed, the next bare token is
  looked up against the command registry; a match starts the next
  command, no match raises the same "too many arguments" error a single,
  over-long command always raised.
- **`CommandDefinition::continue_on_failure` / `requires_success`** (#53,
  both additive `bool`, `#[serde(default)]`, default `false`): per-command
  chain failure policy. `continue_on_failure` governs whether the chain
  stops when *this* command fails; `requires_success` governs whether
  *this* command should even run given an earlier failure elsewhere in
  the chain. Unrelated to the pre-existing `required` field on the same
  struct (startup-time handler-registration check).
- **`CliParser::parse_typed_segment()`** (#54, additive): stops cleanly at
  a segment boundary instead of erroring on positional-arity overflow,
  returning `(parsed, consumed)`. `parse_typed()` itself is untouched —
  every existing caller keeps today's exact behaviour.
- **`CliInterface` segmentation and chain execution** (#55, #56,
  internal — `dispatch()` is private, no public API impact): the full
  argument list is resolved and parsed into segments up front, then
  executed in order applying `continue_on_failure`/`requires_success`. A
  failing command is reported as `Error in command {n}/{total}
  ('{name}'): <normal error output>`; a skipped one as `Skipped: command
  {n}/{total} ('{name}') — a preceding command failed`. The process exit
  code always reflects the first ("triggering") failure in the chain,
  even when later commands also fail or are skipped.
- `CliInterface::run_script()` inherits chaining automatically (shared
  `dispatch()`, no code change of its own). The REPL's `:load`
  meta-command does **not** chain — it stays on its own pre-existing
  scalar-only path, unchanged and regression-tested.
- **`CONFIG_SYNTAX_REFERENCE.md` / `.fr.md`** (#57): new "Command
  Chaining" section — field reference, the segmentation rule, exact
  error/skip message formats, and the documented known limitation below.

### Fixed

- **`CliParser::parse_repeatable_occurrence()` — bare token after a
  repeatable option's `key=value` span** (#54): a plain token with
  neither a leading `-` nor a `=` immediately following a repeatable
  option's occurrence used to raise `InvalidSyntax` ("Expected
  key=value..."), instead of cleanly ending that occurrence's span. Left
  uncaught, this would have broken chaining's own motivating case (a
  repeatable-option command immediately followed by the next chained
  command's name). No prior test exercised this path, so
  `parse_typed()`/`parse()` behaviour for every previously-valid input is
  unaffected.

### Maintenance

- **`SysInfoPlugin`/`EnvPlugin` — `clippy::default_constructed_unit_structs`**:
  both are unit structs whose `Default` impl exists solely to satisfy
  `clippy::new_without_default`; a newer clippy version's own suggestion
  (drop `::default()` for the bare unit literal) would have stopped their
  dedicated tests from exercising that impl at all. Targeted
  `#[allow(clippy::default_constructed_unit_structs)]` added on those two
  tests instead, with justification — unrelated to this release's actual
  scope, surfaced only because `--all-features --all-targets` clippy was
  run in full for the first time during this sprint's validation.

**Breaking Changes**: None — every schema addition is additive
(`#[serde(default)]`), `parse_typed()`/`parse()` and `dispatch()`'s public
callers (`run()`, `run_script()`) are all unaffected in shape and
single-command behaviour.

**Known limitation (documented, not scheduled for a fix)**: if a command
line supplies one token more than that command's declared arity, and
that leftover token happens to also be a registered command name, it is
silently read as the start of the next segment instead of raising an
error — segmentation cannot distinguish "a stray extra value" from "the
next command" once arity is exhausted. No `--`-style escape is
implemented; revisit only if a concrete need appears.

---

## [0.7.0] "Divide and Conquer" - 2026-08-23

**Theme**: Static Plugin Library — Granular Plugins & Batch Execution
**Decision**: [DD-025](https://github.com/biface/dcli/issues/25)

### Added

#### Granular static plugins, split out of `SystemPlugin`
- **`HelpPlugin`, `VersionPlugin`, `ExitPlugin`** (`src/plugin/builtin/`):
  `SystemPlugin`'s three bundled commands (`help`/`version`/`exit`) are now
  also available as independent, composable plugins — register only the
  one(s) an application needs. Each reuses `SystemPlugin`'s exact handler
  logic internally (the handlers moved to `pub(crate)` with a `::new()`
  constructor) — zero duplicated logic, and `SystemPlugin`'s own public API,
  `handlers()` output, and test suite are unchanged. Implementation names
  (`system_help`/`system_version`/`system_exit`) are unchanged too, so
  existing YAML configs work unmodified whichever way the commands are
  wired in.
- **`SysInfoPlugin`** (`src/plugin/builtin/sysinfo.rs`, feature
  `sysinfo-plugin`): `sysinfo_show` reports OS, architecture, and available
  parallelism — a deliberate `std`-only baseline, zero new dependency. A
  richer `sysinfo`-crate-backed variant, if pursued later, stays under this
  same feature flag rather than a second one.
- **`EnvPlugin`** (`src/plugin/builtin/env.rs`, feature `env-plugin`):
  `env_show` prints environment variables, hiding any whose name looks
  sensitive under a case-insensitive deny-list (`SECRET`, `TOKEN`, `KEY`,
  `PASS`, `CREDENTIAL`, `AUTH`, `PRIVATE`). A hidden variable's *name* is
  still shown — only its value is withheld. An allow-list mechanism
  (`ArgumentDefinition::secure`, DD-023) isn't viable here: unlike a
  config-declared argument, environment variable names are arbitrary and
  unknown ahead of time, so there is nothing to declare against.
- **`ConfigPlugin`** (`src/plugin/builtin/config.rs`, feature
  `config-plugin`): `config_show` (renders the loaded config as YAML via
  `serde_yaml`, already a non-feature-gated dependency) and
  `config_validate` (calls `crate::config::validate_config` directly — the
  same function `CliBuilder::build()` already runs internally, no
  duplicated validation logic).
- All granular plugins re-exported from the crate root and `prelude`,
  cfg-gated where feature-flagged.

#### Batch command execution and `:load`
- **`CliInterface::run_script()` / `CliApp::run_script()`**: dispatch a
  file of command lines through the same resolve → parse → execute path as
  `run()` — one non-blank, non-comment (`#`-prefixed) line per command,
  tokenized quote-aware via the existing `ReplParser::tokenize()`, parsed
  via `parse_typed()` so DD-024 repeatable options are fully preserved in
  batch scripts. `ScriptErrorPolicy::{Abort, Continue}` controls what
  happens when a line fails; `ScriptOutcome{lines_executed,
  lines_succeeded, failures}` reports the outcome, with every failure
  carrying its 1-based line number.
- **`:load <path>` REPL meta-command**: loads a script from an
  already-running REPL session, closing the gap where `run_script()` only
  works as a one-shot replacement for `run_cli()`/`run_repl()`.
  Deliberately stays on the REPL's existing scalar-only parse path
  (DD-024 addendum) rather than `run_script()`'s typed path — REPL's
  scalar-only scoping was a deliberate prior decision, not reopened here.
  No error-policy parameter: a failing line is displayed inline (matching
  how the REPL already surfaces errors for typed lines) and the load
  always continues, ending with a `succeeded/attempted` summary.

  This closes the scope originally filed as "ScriptLoaderPlugin": a
  `Plugin`-shaped implementation turned out to be architecturally
  impossible — `CommandHandler::execute` receives only
  `&mut dyn ExecutionContext`, never the `CommandRegistry`, so a
  plugin-contributed handler has no way to dispatch other registered
  commands. Reframed as methods on `CliInterface`/`CliApp` instead, which
  already own both — additive, no breaking change.

#### New example
- **`examples/advanced_rpn_calculator.rs`**: HP-41CX-flavored extension of
  `rpn_calculator.rs` — scientific functions (`sqrt`, `sq`, `inv`, `pow`,
  `exp`, `log10`, `sin`, `cos`, `tan`, `chs`, `pi`) and a 10-slot memory
  register bank (`sto`/`rcl`). Demonstrates `SysInfoPlugin` and
  `ConfigPlugin` registered together via `CliBuilder::register_plugin()`,
  `CliApp::run_script()` (`--script <file>`), and `:load` from inside the
  REPL. New `[[example]]` entry in `Cargo.toml` with
  `required-features = ["sysinfo-plugin", "config-plugin"]`.

### Fixed

- **`examples/rpn_calculator.rs` and `examples/advanced_rpn_calculator.rs`
  — `sub`/`div` operand order**: `binary_op()`'s subtraction and division
  closures computed `X - Y` / `X / Y` (`X` = most-recently-popped, the
  actual top of stack), not the standard HP RPN convention `Y - X` /
  `Y / X`. `"10 ENTER 3 -"` returned `-7` instead of the correct `7`. The
  bug was untested before now — only the commutative `mul` had a test,
  which masked the ordering issue. Regression tests added to both
  examples to lock in the corrected convention.

### Maintenance

- **Clippy lint drift** (43 pre-existing errors across `examples/`,
  `tests/integration/`, and unrelated `src/` modules, unrelated to this
  release's actual changes) traced to an unpinned CI toolchain: the
  `fmt`/`clippy`/`test`/`doc` jobs ran on whatever `ubuntu-latest`
  shipped, silently drifting to newer clippy lints as GitHub updated its
  runner image. Fixed at the root: all four jobs in `ci.yml` now pin
  `dtolnay/rust-toolchain@1.97.0` explicitly. `clippy` gained a combined
  `cargo clippy --all-features -- -D warnings` step, alongside the
  existing default-features and `--features wasm-plugins` runs.
  `coverage.yml` still pins only `@stable` (floating) — left as a possible
  follow-up.

**Breaking Changes**: None — every addition above is additive; the three
existing bugs fixed (`sub`/`div` operand order, clippy drift) are example-
and tooling-only, not public API changes.

**Roadmap follow-up**: `DD-027` ("Multi-line option accumulation in REPL
mode") was extracted from the DD-025 triage as a separate, not-yet-
scheduled design decision — the REPL's scalar-only scoping (DD-024
addendum) that `:load` deliberately respects above is exactly the
boundary DD-027 will revisit.

---

## [0.6.0] "Déjà Vu" - 2026-07-25

**Theme**: Advanced Options — Repeatable Options with Typed Sub-Parameters
**Decision**: [DD-024](https://github.com/biface/dcli/issues/21)

### Added

#### Repeatable options (`OptionDefinition::repeatable`)
- **`OptionDefinition` gains two additive fields** (`src/config/schema.rs`):
  ```rust
  #[serde(default)]
  pub repeatable: bool,

  #[serde(default)]
  pub option_parameters: HashMap<String, Vec<ArgumentDefinition>>,
  ```
  `choices` doubles as the set of valid discriminants when `repeatable: true`
  — no change to its existing meaning for scalar options. Both fields
  default to `false` / empty, so no existing YAML config breaks.
- **Config-load-time validation** (`src/config/validator.rs::validate_options()`):
  `repeatable: true` requires non-empty `choices`; `option_parameters` keys
  must equal `choices` exactly (no orphan key, no undeclared discriminant);
  each `option_parameters[discriminant]` is validated via the existing
  `validate_argument_names` / `validate_argument_types` (explicitly *not*
  `validate_argument_ordering`, meaningless for named `key=value` pairs);
  `repeatable: false` with non-empty `option_parameters` is rejected;
  `repeatable: true` with `default: Some(_)` is rejected — a repeatable
  option's absence means zero occurrences, not an implicit one.
- **New `ParseError` variants** (`src/error/types.rs`): `UnknownOptionParameter`,
  `MissingRequiredOptionParameter`, `UnknownDiscriminant`,
  `DuplicateOptionOccurrence` — each with an actionable `suggestion`
  pointing at `--help`, consistent with the DD-011 convention.
- **Parser support** (`src/parser/cli_parser.rs`): new `OptionOccurrence`
  and `ParsedValue::{Scalar, Repeated}` types; `--output csv file=... [k=v ...]`
  accumulates into `Vec<OptionOccurrence>` in command-line order. Two
  occurrences of the same discriminant with different `key=value` pairs are
  both kept; identical occurrences are rejected
  (`DuplicateOptionOccurrence`) — partially-overlapping occurrences are
  *not* framework-rejected, since `dynamic-cli` has no way to know which
  key is domain-significant; that stays the handler's responsibility.

### Changed — Breaking

- **`CommandHandler::execute` / `validate` and `AsyncCommandHandler::execute`
  / `validate`** now take `&ParsedArgs` instead of
  `&HashMap<String, String>`. `HashMap<String, String>` could not represent
  both a scalar option and a repeated-occurrence option in the same map;
  `validate()` moves alongside `execute()` so a handler sees the same
  argument shape in both — otherwise it could not validate repeatable-option
  occurrences before `execute()` runs.

  ```rust
  pub struct ParsedArgs(HashMap<String, ParsedValue>);

  impl ParsedArgs {
      pub fn get_scalar(&self, name: &str) -> Option<&str> { .. }
      pub fn get_repeated(&self, name: &str) -> Option<&[OptionOccurrence]> { .. }
      pub fn to_scalar_map(&self) -> HashMap<String, String> { .. }
  }
  ```

  **Before**:
  ```rust
  impl CommandHandler for MyCommand {
      fn execute(
          &self,
          context: &mut dyn ExecutionContext,
          args: &HashMap<String, String>,
      ) -> Result<()> {
          let name = args.get("name").unwrap();
          // ...
      }
  }
  ```

  **After**:
  ```rust
  impl CommandHandler for MyCommand {
      fn execute(
          &self,
          context: &mut dyn ExecutionContext,
          args: &ParsedArgs,
      ) -> Result<()> {
          let name = args.get_scalar("name").unwrap();
          // ...
      }
  }
  ```

  For handlers that never use repeatable options, migration is close to a
  find-and-replace: `args.get("x")` → `args.get_scalar("x")`. Subsystems
  that predate DD-024 and still expect a flat `HashMap<String, String>`
  (e.g. the WASM plugin ABI in `src/plugin/wasm.rs`, which serializes
  arguments across the guest boundary) bridge via
  `args.to_scalar_map()` rather than being rewritten — repeatable options
  are silently dropped for those consumers, matching the pre-DD-024
  behaviour they were built against.

- All in-crate examples (`examples/*.rs`) and the `README.md` / `README.fr.md`
  quick-start handler migrated to the new signature.

**Deviation from the v0.5.0 roadmap note**: this signature change was
previously expected to land at v1.0.0, batched with the removal of the
`register()` / `get_handler()` / `register_handler()` deprecated aliases.
It ships here instead, at v0.6.0, since DD-024 is itself a breaking change
and batching it further only delays real-world validation against
`chrom-rs`. The deprecated-alias removal is unaffected and still targets
v1.0.0.

### Documentation
- **`CONFIG_SYNTAX_REFERENCE.md`** / **`.fr.md`**: new section on
  `repeatable` and `option_parameters`, with the `--output csv file=...
  resolution=...` / `--output plot file=...` worked example.
- **`README.md`** / **`.fr.md`**: install snippet bumped to `0.6.0`;
  quick-start handler example migrated to `ParsedArgs`.
- **`DESIGN_DECISIONS.md`**: DD-024 moved from "decided — implementation
  pending" to closed.

**Breaking Changes**: Yes — every existing `CommandHandler` /
`AsyncCommandHandler` implementation must migrate both `execute()` and
`validate()`. `chrom-rs`'s migration is tracked separately in its own
issue tracker.

**Roadmap follow-up**: the `register()` / `get_handler()` /
`register_handler()` deprecated aliases (introduced in v0.5.0) are still
scheduled for removal at **v1.0.0**, unaffected by this release.

---

## [0.5.0] "Beyond Blocking" - 2026-07-11

**Theme**: Async Command Handlers
**Decision**: [DD-022](https://github.com/biface/dcli/issues/8)

### Added

#### `AsyncCommandHandler` trait (Option C — `async-trait`)
- **`AsyncCommandHandler` trait** (`src/executor/traits.rs`, alongside
  `CommandHandler`): additive async counterpart, object-safe via
  `#[async_trait]`, same contract as `CommandHandler` (`execute`/`validate`)
  aside from the `async` keyword.
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
- **Unified registry storage**: `CommandRegistry` stores sync and async
  handlers behind a single private `StoredHandler` enum instead of two
  parallel maps — one conflict check, one lookup path.
- **`CommandRegistry::register_async()` / `get_handler_async()`**:
  symmetric with the renamed `register_sync()` / `get_handler_sync()` (see
  Deprecated below). A command name resolves to exactly one handler kind;
  registering the same name as both sync and async fails at registration
  time.
- **`CliBuilder::register_async_handler()`**: symmetric with the renamed
  `register_sync_handler()`. `build()` now drains both handler maps,
  rejects a command registered both sync and async for the same
  `implementation` name, and accepts a required command satisfied by an
  async handler alone.
- **Dispatch** (`CliInterface::run()`, `ReplInterface::execute_line()`):
  sync handler tried first (unchanged behaviour); if none matches, the
  async handler is driven via `futures::executor::block_on(...)` — safe
  because both dispatch loops are already strictly sequential.
- New dependencies: `async-trait = "0.1"`, `futures = "0.3"` — normal
  (non-feature-gated) dependencies. Unlike `wasmtime` (v0.4.0),
  `async-trait` is a lightweight proc-macro with no runtime footprint, so
  gating it behind a feature flag would add complexity without a
  corresponding benefit.

### Deprecated

- **`CommandRegistry::register()`** → renamed `register_sync()`, for
  symmetry with `register_async()`. Old name kept as a `#[deprecated]`
  alias, scheduled for removal in **v1.0.0**.
- **`CommandRegistry::get_handler()`** → renamed `get_handler_sync()`, for
  symmetry with `get_handler_async()`. Same deprecation schedule.
- **`CliBuilder::register_handler()`** → renamed `register_sync_handler()`,
  for symmetry with `register_async_handler()`. Same deprecation schedule.
- Downstream consumers (including `chrom-rs`) should migrate to the `_sync`
  names now: `cargo clippy -- -D warnings` turns the deprecation into a
  build error, so the old names cannot be used silently going forward.
- Tracked for removal alongside DD-024's planned `CommandHandler::execute`
  signature change in the **v1.0.0 API cleanup** tracking issue.

### Documentation
- **`CONFIG_SYNTAX_REFERENCE.md`** / **`.fr.md`**: clarified that sync vs.
  async is not a config-file concern — the `implementation` field is
  agnostic to handler kind; the choice is made entirely on the Rust side
  via `register_sync_handler()` / `register_async_handler()`.
- `AsyncCommandHandler` re-exported from `prelude`, with the same rustdoc
  depth (contract, object-safety/Send+Sync notes, worked example) as
  `CommandHandler`.

### Testing
- `tests/integration/async_token_test.rs`: end-to-end proof that an
  `AsyncCommandHandler` reaches execution through the real dispatch path
  (not just a direct `.execute()` call) — alias resolution, a required
  command satisfied by an async handler alone, and elapsed-time assertions
  confirming `block_on` genuinely awaits the handler rather than returning
  early.
- `examples/async_token_demo.rs`: runnable demo of the same handler with a
  literal 10-second non-blocking delay (`futures_timer::Delay`, not
  `std::thread::sleep` — a thread sleep here would block the executor
  exactly like a sync call, defeating the purpose of the demo).
- Deprecated-alias coverage: dedicated tests (`#[allow(deprecated)]`) in
  `command_registry.rs` and `builder.rs` confirm `register()` /
  `get_handler()` / `register_handler()` behave identically to their
  renamed counterparts.
- Existing test suites, examples, and doc examples across the crate
  migrated to the new names to satisfy `cargo clippy -- -D warnings`.

**Breaking Changes**: None (fully additive; the three renames above are
deprecations, not removals — both names work until v1.0.0)

**Roadmap follow-up**: `CommandHandler::execute`'s signature is expected to
change under DD-024 (v0.6.0, repeatable options), batched with the removal
of the deprecated names above at v1.0.0. Concurrent/cancellable async
execution — explicitly out of scope here, since `ExecutionContext` is
borrowed non-`'static` — would require its own future design decision if
ever confirmed as a need.

---

## [0.4.0] "Plugin System" - 2026-06-19

**Theme**: Plugin System
**Decision**: [DD-021](https://github.com/biface/dcli/issues/10)

### Added

#### Static plugins (Option A)
- **`Plugin` trait** (`src/plugin/mod.rs`): declarative extension point.
  A plugin declares `name`/`version`/`description` and returns its handlers
  via `handlers() -> Vec<(String, Box<dyn CommandHandler>)>`. The framework
  controls registration — a plugin never receives a `&mut CommandRegistry`.
- **`SystemPlugin`** (`src/plugin/system.rs`): reference implementation
  supplying `system_help`, `system_version`, `system_exit`.
  - `system_exit` accepts an optional shutdown callback via
    `with_exit_fn()`, defaulting to `std::process::exit(0)`, for
    applications that need a clean shutdown sequence (flush logs, close
    connections) before exiting.
- **`CliBuilder::register_plugin(Box<dyn Plugin>)`**: additive, freely
  combinable with `register_handler()`.
- **Conflict detection**: `build()` fails with an actionable error if two
  sources (plugins or direct handlers) claim the same `implementation`
  name, rather than silently overwriting one with the other.

#### WASM plugins (Option C, opt-in)
- **`WasmPlugin`** (`src/plugin/wasm.rs`, feature `wasm-plugins`): loads a
  sandboxed `.wasm` (or `.wat`) module via `wasmtime`. Validates three
  mandatory exports at load time: `memory`, `dcli_alloc`, `dcli_dealloc`.
  Business functions are mapped via `with_function_map(impl_name, wasm_fn_name)`.
- **`WasmSerializationFormat`**: YAML by default (config-first principle),
  JSON available via `with_format()`.
- **`CliBuilder::register_wasm_plugin(path, function_map)`**: convenience
  wrapper; `function_map` is mandatory (no safe default — an empty map
  would register zero handlers).
- **`dcli_dealloc` always called**: on every exit path of a handler call,
  including guest error returns, preventing unfreed-buffer accumulation
  across a long-running REPL session.
- **Optional `dcli_last_error_message` export**: detailed error messages
  from the guest when a business function returns a non-zero code;
  degrades gracefully (`message: None`) when absent.
- **`WasmError`** (`src/error/types.rs`, feature `wasm-plugins`): typed
  error category — `LoadFailed`, `FunctionNotFound`, `GuestError`,
  `SerializationFailed`, `MemoryAccessFailed`.
- New optional dependency: `wasmtime = "45.0.0"` (only pulled in under
  `--features wasm-plugins`; zero cost otherwise).

#### Documentation
- **`PLUGIN_GUIDE.md`** / **`PLUGIN_GUIDE.fr.md`**: end-to-end plugin guide
  — common YAML contract, static plugins with a worked `chrom-rs` example,
  full WASM ABI contract (mandatory/optional exports, call sequence,
  host/guest terminology), and a distinction between WASM **restrictions**
  (no `ExecutionContext` access — permanent, security-motivated) and
  **limitations** (no host functions, no WASI — absent today, not excluded
  tomorrow).

### Excluded
- **Dynamic loading via `libloading`** (Option B): permanently excluded.
  Rust's ABI is not stable across compiler versions, making this approach
  structurally unsafe regardless of implementation quality. Will not be
  reconsidered in a future version.

### Testing
- `tests/integration/system_plugin_test.rs`: `SystemPlugin` through the
  full `CliBuilder` → `build()` → `CliApp` → `run_cli()` chain — dispatch,
  alias resolution, coexistence with native handlers, conflict detection.
- `tests/integration/wasm_plugin_test.rs` (feature `wasm-plugins`):
  `WasmPlugin` through `register_wasm_plugin()` and the full chain, using
  real `.wasm` files written via `NamedTempFile` — success path, guest
  error with and without the optional message export, coexistence with a
  native handler.
- CI (`ci.yml`, `coverage.yml`) extended to run `clippy`/`test`/coverage
  under `--features wasm-plugins` explicitly, alongside default features.

**Breaking Changes**: None (plugins are opt-in; `wasm-plugins` is an
optional feature flag)

**Roadmap follow-up**: granular static plugins
(`HelpPlugin`/`VersionPlugin`/`ExitPlugin` alongside the existing
`SystemPlugin`) and new official static plugins (`SysInfoPlugin`,
`EnvPlugin`, `ConfigPlugin`) deferred to
[v0.7.0 · Static Plugin Library](https://github.com/biface/dcli/milestone/?title=v0.7.0).

---

## [0.3.0] "Shell Completions and Advanced History" - 2026-05-30

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

## [0.2.0] "Help & Errors" - 2026-04-05

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
| **0.4.0** | Plugin System            | Extensible handlers                 | 4-6 weeks | ✅ Released           |
| **0.5.0** | Async Support            | Async handlers (optional)           | 4-6 weeks | ✅ Released           |
| **0.6.0** | Advanced Options         | Repeatable options, typed sub-params| 4-6 weeks | ✅ Released           |
| **0.7.0** | Static Plugin Library    | Granular plugins, batch execution   | —         | ✅ Released           |
| **1.0.0** | Stable                   | Production-ready, locked API        | -         | 🔵 Planned            |

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

**Last Updated**: 2026-08-23  
**Current Version**: 0.7.0  
**Next Release**: 1.0.0 (planned Q2 2027)
