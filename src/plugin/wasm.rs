//! WASM plugin loader for `dynamic-cli` (Option C — DD-021)
//!
//! Provides [`WasmPlugin`], a [`Plugin`] implementation backed by a sandboxed
//! WebAssembly module loaded and executed via `wasmtime`. Only available
//! when the `wasm-plugins` feature is enabled.
//!
//! # Why WASM plugins
//!
//! Static plugins ([`SystemPlugin`][crate::plugin::SystemPlugin] and Option A
//! in general) must be compiled into the host binary. WASM plugins trade
//! that compile-time coupling for a safe, cross-platform sandbox: a `.wasm`
//! module can be distributed independently of the host application and
//! loaded at runtime, with no `unsafe` code on the host side.
//!
//! # ABI contract — mandatory exports
//!
//! Every WASM module loaded as a [`WasmPlugin`] **must** export:
//!
//! | Export | Signature | Purpose |
//! |--------|-----------|---------|
//! | `memory` | (standard linear memory) | Shared buffer for argument/result transfer |
//! | `dcli_alloc` | `(size: i32) -> i32` | Host asks the guest to reserve `size` bytes; returns the pointer |
//! | `dcli_dealloc` | `(ptr: i32, size: i32)` | Host asks the guest to free a buffer it previously allocated |
//! | *(business function)* | `(ptr: i32, len: i32) -> i32` | Reads serialized args at `ptr`/`len`; returns `0` on success, non-zero on error |
//!
//! The business function's exported name is chosen freely by the plugin
//! author and mapped to an `implementation` name via
//! [`WasmPlugin::with_function_map`].
//!
//! # ABI contract — optional exports
//!
//! | Export | Signature | Purpose |
//! |--------|-----------|---------|
//! | `dcli_last_error_message` | `() -> (ptr: i32, len: i32)` | Detailed error message when the business function returns non-zero |
//!
//! When absent, errors surface with the raw code only
//! ([`WasmError::guest_error_without_message`]).
//!
//! # Serialization
//!
//! Handler arguments (`HashMap<String, String>`) are serialized to a byte
//! buffer before crossing the host/guest boundary. YAML is the default,
//! consistent with the framework's config-first principle (DD-002); JSON is
//! available via [`WasmPlugin::with_format`].
//!
//! # Known limitation — no `ExecutionContext` access
//!
//! WASM handlers do **not** receive the host's [`ExecutionContext`]. Trait
//! objects cannot cross the WASM FFI boundary, and exposing arbitrary host
//! state to a sandboxed guest would defeat the purpose of the sandbox. WASM
//! plugins in this version only exchange serialized arguments and a result
//! code/message.
//!
//! Future work may introduce a restricted set of host functions (e.g.
//! `host_log`, `host_get_state`) or WASI integration for guests that need
//! controlled access to host capabilities — see DD-021 for the open
//! discussion. This version intentionally ships without them.
//!
//! Full reference: `WASM_PLUGIN_INTERFACE.md`.
//!
//! # Example
//!
//! ```no_run
//! use dynamic_cli::plugin::wasm::{WasmPlugin, WasmSerializationFormat};
//! use std::path::Path;
//!
//! # fn main() -> dynamic_cli::Result<()> {
//! let plugin = WasmPlugin::load(Path::new("plugins/greet.wasm"))?
//!     .with_function_map("greet_hello", "say_hello")
//!     .with_format(WasmSerializationFormat::Yaml)
//!     .with_metadata("greet", "1.0.0", "Greeting commands");
//! # Ok(())
//! # }
//! ```

use crate::context::ExecutionContext;
use crate::error::WasmError;
use crate::executor::CommandHandler;
use crate::parser::ParsedArgs;
use crate::plugin::Plugin;
use crate::Result;
use std::collections::HashMap;
use std::path::Path;
use wasmtime::{Engine, Instance, Module, Store};

// ============================================================================
// WasmSerializationFormat
// ============================================================================

/// Serialization format used to exchange handler arguments across the
/// host/guest boundary.
///
/// YAML is the default, consistent with the framework's config-first
/// principle (DD-002). Guests that prefer JSON can request it via
/// [`WasmPlugin::with_format`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WasmSerializationFormat {
    /// YAML-encoded arguments (default)
    #[default]
    Yaml,
    /// JSON-encoded arguments
    Json,
}

impl WasmSerializationFormat {
    /// Serialize a handler argument map into bytes using this format.
    fn serialize(self, args: &HashMap<String, String>) -> Result<Vec<u8>> {
        let bytes = match self {
            Self::Yaml => serde_yaml::to_string(args)
                .map_err(|e| WasmError::SerializationFailed(e.to_string()))?
                .into_bytes(),
            Self::Json => serde_json::to_vec(args)
                .map_err(|e| WasmError::SerializationFailed(e.to_string()))?,
        };
        Ok(bytes)
    }
}

// ============================================================================
// Mandatory export names
// ============================================================================

const EXPORT_MEMORY: &str = "memory";
const EXPORT_ALLOC: &str = "dcli_alloc";
const EXPORT_DEALLOC: &str = "dcli_dealloc";
const EXPORT_LAST_ERROR: &str = "dcli_last_error_message";

// ============================================================================
// WasmPlugin
// ============================================================================

/// A [`Plugin`] backed by a sandboxed WASM module.
///
/// Load a `.wasm` (or `.wat`, useful for testing) module with [`WasmPlugin::load`],
/// map its business functions to `implementation` names with
/// [`with_function_map`][Self::with_function_map], then register it via
/// [`CliBuilder::register_plugin`][crate::CliBuilder::register_plugin] or the
/// convenience [`CliBuilder::register_wasm_plugin`][crate::CliBuilder::register_wasm_plugin].
///
/// See the [module-level documentation](self) for the full ABI contract.
pub struct WasmPlugin {
    name: String,
    version: String,
    description: String,
    engine: Engine,
    module: Module,
    /// `implementation_name -> wasm_exported_function_name`
    function_map: HashMap<String, String>,
    format: WasmSerializationFormat,
}

impl WasmPlugin {
    /// Load a WASM module from disk and validate its mandatory exports.
    ///
    /// Accepts both binary `.wasm` and text `.wat` modules (the latter is
    /// primarily useful for tests and minimal fixtures).
    ///
    /// # Errors
    ///
    /// Returns [`WasmError::LoadFailed`] if the file cannot be read or fails
    /// to compile/validate. Returns [`WasmError::FunctionNotFound`] if
    /// `memory`, `dcli_alloc`, or `dcli_dealloc` is missing — these three
    /// are mandatory regardless of which business functions are mapped
    /// later.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::plugin::wasm::WasmPlugin;
    /// use std::path::Path;
    ///
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let plugin = WasmPlugin::load(Path::new("plugins/greet.wasm"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|e| WasmError::LoadFailed {
            path: path.to_path_buf(),
            source: anyhow::Error::new(e),
            suggestion: Some("Verify the path is correct and the file is readable.".to_string()),
        })?;

        Self::from_bytes(&bytes, path)
    }

    /// Load a WASM module from raw bytes (binary `.wasm` or text `.wat`).
    ///
    /// Used internally by [`load`][Self::load]. Exposed for tests and for
    /// embedding scenarios where the module bytes are not stored on disk
    /// (e.g. fetched over the network).
    ///
    /// Validates the same mandatory exports as [`load`][Self::load].
    pub fn from_bytes(bytes: &[u8], origin: &Path) -> Result<Self> {
        let engine = Engine::default();

        let module = Module::new(&engine, bytes).map_err(|e| WasmError::LoadFailed {
            path: origin.to_path_buf(),
            source: e.into(),
            suggestion: Some("Verify the file is a valid WASM binary or WAT module.".to_string()),
        })?;

        let module_label = origin.display().to_string();
        Self::validate_mandatory_exports(&module, &module_label)?;

        let default_name = origin
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("wasm-plugin")
            .to_string();

        Ok(Self {
            name: default_name,
            version: "0.0.0".to_string(),
            description: "WASM plugin (no metadata provided)".to_string(),
            engine,
            module,
            function_map: HashMap::new(),
            format: WasmSerializationFormat::default(),
        })
    }

    /// Verify that `memory`, `dcli_alloc`, and `dcli_dealloc` are exported.
    ///
    /// These three exports are mandatory for every WASM plugin regardless
    /// of which business functions are mapped — without them the host has
    /// no safe way to exchange data with the guest or to free guest memory
    /// after a call.
    fn validate_mandatory_exports(module: &Module, module_label: &str) -> Result<()> {
        let exported_names: Vec<&str> = module.exports().map(|e| e.name()).collect();

        for required in [EXPORT_MEMORY, EXPORT_ALLOC, EXPORT_DEALLOC] {
            if !exported_names.contains(&required) {
                return Err(WasmError::missing_mandatory_export(required, module_label).into());
            }
        }
        Ok(())
    }

    /// Map an `implementation` name to a WASM-exported business function.
    ///
    /// The exported function name is chosen freely by the plugin author —
    /// `dynamic-cli` imposes no naming convention beyond the four reserved
    /// names (`memory`, `dcli_alloc`, `dcli_dealloc`, `dcli_last_error_message`).
    ///
    /// Existence of `wasm_fn_name` in the module is verified at handler
    /// construction time, in [`Plugin::handlers`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dynamic_cli::plugin::wasm::WasmPlugin;
    /// use std::path::Path;
    ///
    /// # fn main() -> dynamic_cli::Result<()> {
    /// let plugin = WasmPlugin::load(Path::new("plugins/greet.wasm"))?
    ///     .with_function_map("greet_hello", "say_hello");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_function_map(mut self, impl_name: &str, wasm_fn_name: &str) -> Self {
        self.function_map
            .insert(impl_name.to_string(), wasm_fn_name.to_string());
        self
    }

    /// Set the serialization format used for argument exchange.
    ///
    /// Defaults to [`WasmSerializationFormat::Yaml`].
    pub fn with_format(mut self, format: WasmSerializationFormat) -> Self {
        self.format = format;
        self
    }

    /// Set plugin metadata (name, version, description).
    ///
    /// When not called, defaults are derived from the module's file name
    /// (`name`), `"0.0.0"` (`version`), and a generic description.
    pub fn with_metadata(mut self, name: &str, version: &str, description: &str) -> Self {
        self.name = name.to_string();
        self.version = version.to_string();
        self.description = description.to_string();
        self
    }
}

impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn handlers(&self) -> Vec<(String, Box<dyn CommandHandler>)> {
        self.function_map
            .iter()
            .map(|(impl_name, wasm_fn_name)| {
                let handler: Box<dyn CommandHandler> = Box::new(WasmHandler {
                    engine: self.engine.clone(),
                    module: self.module.clone(),
                    module_label: self.name.clone(),
                    wasm_fn_name: wasm_fn_name.clone(),
                    format: self.format,
                });
                (impl_name.clone(), handler)
            })
            .collect()
    }
}

// ============================================================================
// WasmHandler (private)
// ============================================================================

/// [`CommandHandler`] that invokes a single exported WASM business function.
///
/// One instance is created per mapped `implementation` name in
/// [`WasmPlugin::handlers`]. Each call to [`execute`][CommandHandler::execute]
/// creates a fresh `Store` and `Instance` — WASM instantiation is cheap and
/// this keeps each invocation isolated, with no state leaking between calls.
struct WasmHandler {
    engine: Engine,
    module: Module,
    /// Plugin name, used only to identify the module in error messages
    module_label: String,
    wasm_fn_name: String,
    format: WasmSerializationFormat,
}

impl WasmHandler {
    /// Run the full call sequence: alloc → write → call → dealloc → result.
    ///
    /// `dcli_dealloc` is invoked on every exit path — including when the
    /// business function itself returns a non-zero error code — so the
    /// guest never accumulates unfreed buffers across repeated calls.
    fn call_guest(&self, args: &HashMap<String, String>) -> Result<()> {
        let mut store = Store::new(&self.engine, ());
        let instance =
            Instance::new(&mut store, &self.module, &[]).map_err(|e| WasmError::LoadFailed {
                path: std::path::PathBuf::from(&self.module_label),
                source: e.into(),
                suggestion: Some("Failed to instantiate the WASM module.".to_string()),
            })?;

        let memory = instance
            .get_memory(&mut store, EXPORT_MEMORY)
            .ok_or_else(|| {
                WasmError::missing_mandatory_export(EXPORT_MEMORY, &self.module_label)
            })?;

        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, EXPORT_ALLOC)
            .map_err(|_| WasmError::missing_mandatory_export(EXPORT_ALLOC, &self.module_label))?;

        let dealloc = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, EXPORT_DEALLOC)
            .map_err(|_| WasmError::missing_mandatory_export(EXPORT_DEALLOC, &self.module_label))?;

        let business_fn = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, self.wasm_fn_name.as_str())
            .map_err(|_| WasmError::FunctionNotFound {
                function: self.wasm_fn_name.clone(),
                module: self.module_label.clone(),
                suggestion: Some(format!(
                    "Export `fn {}(ptr: i32, len: i32) -> i32` from the WASM module, \
                     or check the name passed to `with_function_map`.",
                    self.wasm_fn_name
                )),
            })?;

        // Serialize arguments and request guest-owned buffer space.
        let payload = self.format.serialize(args)?;
        let len = payload.len() as i32;

        let ptr = alloc
            .call(&mut store, len)
            .map_err(|e| WasmError::MemoryAccessFailed {
                reason: e.to_string(),
            })?;

        memory
            .write(&mut store, ptr as usize, &payload)
            .map_err(|e| WasmError::MemoryAccessFailed {
                reason: e.to_string(),
            })?;

        // Invoke the business function. Regardless of outcome, the guest
        // buffer is freed before this function returns (see below).
        let call_result = business_fn.call(&mut store, (ptr, len));

        // Always free the buffer we allocated, on every exit path.
        let dealloc_result = dealloc.call(&mut store, (ptr, len));

        let code = call_result.map_err(|e| WasmError::MemoryAccessFailed {
            reason: format!("business function trapped: {e}"),
        })?;

        // A failure to deallocate is logged via the returned error only if
        // the business call itself succeeded — otherwise the guest error
        // takes priority as the more actionable failure.
        if code == 0 {
            dealloc_result.map_err(|e| WasmError::MemoryAccessFailed {
                reason: format!("dcli_dealloc failed: {e}"),
            })?;
            return Ok(());
        }

        // Non-zero return code: attempt to retrieve a detailed message via
        // the optional `dcli_last_error_message` export.
        //
        // dealloc_result is intentionally not surfaced here even if it
        // failed: the guest's own error code is the more actionable signal
        // for the caller, and a secondary dealloc failure on an already
        // failing call would only obscure it.
        let message = self.read_last_error_message(&mut store, &instance);
        Err(WasmError::GuestError { code, message }.into())
    }

    /// Best-effort retrieval of a detailed error message from the guest.
    ///
    /// Returns `None` when the module does not export
    /// `dcli_last_error_message`, or when reading the message fails for any
    /// reason. A missing or unreadable message degrades to the raw error
    /// code — it never escalates into a separate error of its own.
    fn read_last_error_message(
        &self,
        store: &mut Store<()>,
        instance: &Instance,
    ) -> Option<String> {
        let last_error_fn = instance
            .get_typed_func::<(), (i32, i32)>(&mut *store, EXPORT_LAST_ERROR)
            .ok()?;
        let (ptr, len) = last_error_fn.call(&mut *store, ()).ok()?;
        if len <= 0 {
            return None;
        }
        let memory = instance.get_memory(&mut *store, EXPORT_MEMORY)?;
        let mut buf = vec![0u8; len as usize];
        memory.read(&mut *store, ptr as usize, &mut buf).ok()?;
        String::from_utf8(buf).ok()
    }
}

impl CommandHandler for WasmHandler {
    fn execute(&self, _ctx: &mut dyn ExecutionContext, args: &ParsedArgs) -> Result<()> {
        // ExecutionContext is intentionally not forwarded to the guest —
        // see the module-level "Known limitation" section.
        //
        // The WASM ABI (DD-021) predates repeatable options (DD-024) and
        // has not been extended to represent them across the host/guest
        // boundary; any `ParsedValue::Repeated` entry is silently dropped
        // by `to_scalar_map()`, same rationale as the REPL path.
        self.call_guest(&args.to_scalar_map())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::path::PathBuf;

    #[derive(Default)]
    struct TestContext;

    impl ExecutionContext for TestContext {
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    /// A minimal valid module: memory + dcli_alloc + dcli_dealloc + a
    /// business function `ok_handler` that always returns 0 (success).
    ///
    /// Allocation/deallocation are no-ops here (a fixed bump pointer at
    /// offset 1024) — sufficient for ABI-contract tests, not a realistic
    /// allocator.
    const WAT_MINIMAL_OK: &str = r#"
        (module
            (memory (export "memory") 1)
            (func (export "dcli_alloc") (param i32) (result i32)
                i32.const 1024)
            (func (export "dcli_dealloc") (param i32 i32))
            (func (export "ok_handler") (param i32 i32) (result i32)
                i32.const 0)
        )
    "#;

    /// Same as above, but the business function always returns error code 1,
    /// with no `dcli_last_error_message` export.
    const WAT_MINIMAL_ERROR: &str = r#"
        (module
            (memory (export "memory") 1)
            (func (export "dcli_alloc") (param i32) (result i32)
                i32.const 1024)
            (func (export "dcli_dealloc") (param i32 i32))
            (func (export "err_handler") (param i32 i32) (result i32)
                i32.const 1)
        )
    "#;

    /// Same as the error module, but also exports `dcli_last_error_message`
    /// pointing at a fixed "boom" string baked into a data segment.
    const WAT_WITH_ERROR_MESSAGE: &str = r#"
        (module
            (memory (export "memory") 1)
            (data (i32.const 2048) "boom")
            (func (export "dcli_alloc") (param i32) (result i32)
                i32.const 1024)
            (func (export "dcli_dealloc") (param i32 i32))
            (func (export "err_handler") (param i32 i32) (result i32)
                i32.const 1)
            (func (export "dcli_last_error_message") (result i32 i32)
                i32.const 2048
                i32.const 4)
        )
    "#;

    /// Missing `dcli_dealloc` entirely.
    const WAT_MISSING_DEALLOC: &str = r#"
        (module
            (memory (export "memory") 1)
            (func (export "dcli_alloc") (param i32) (result i32)
                i32.const 1024)
            (func (export "ok_handler") (param i32 i32) (result i32)
                i32.const 0)
        )
    "#;

    /// Missing `memory` entirely.
    const WAT_MISSING_MEMORY: &str = r#"
        (module
            (func (export "dcli_alloc") (param i32) (result i32)
                i32.const 1024)
            (func (export "dcli_dealloc") (param i32 i32))
            (func (export "ok_handler") (param i32 i32) (result i32)
                i32.const 0)
        )
    "#;

    fn load_wat(wat: &str) -> Result<WasmPlugin> {
        WasmPlugin::from_bytes(wat.as_bytes(), &PathBuf::from("test.wat"))
    }

    // -------------------------------------------------------------------------
    // Loading & mandatory export validation
    // -------------------------------------------------------------------------

    #[test]
    fn test_load_minimal_valid_module_succeeds() {
        let plugin = load_wat(WAT_MINIMAL_OK);
        assert!(plugin.is_ok());
    }

    #[test]
    fn test_load_missing_dealloc_fails() {
        let result = load_wat(WAT_MISSING_DEALLOC);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_missing_memory_fails() {
        let result = load_wat(WAT_MISSING_MEMORY);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_bytes_fails() {
        let result = WasmPlugin::from_bytes(b"not a wasm module", &PathBuf::from("bad.wasm"));
        assert!(result.is_err());
    }

    #[test]
    fn test_default_metadata_derived_from_filename() {
        let plugin = WasmPlugin::from_bytes(
            WAT_MINIMAL_OK.as_bytes(),
            &PathBuf::from("plugins/greet.wat"),
        )
        .unwrap();
        assert_eq!(plugin.name(), "greet");
    }

    // -------------------------------------------------------------------------
    // Metadata builder methods
    // -------------------------------------------------------------------------

    #[test]
    fn test_with_metadata_overrides_defaults() {
        let plugin =
            load_wat(WAT_MINIMAL_OK)
                .unwrap()
                .with_metadata("custom", "2.5.0", "A custom plugin");
        assert_eq!(plugin.name(), "custom");
        assert_eq!(plugin.version(), "2.5.0");
        assert_eq!(plugin.description(), "A custom plugin");
    }

    #[test]
    fn test_with_format_defaults_to_yaml() {
        let plugin = load_wat(WAT_MINIMAL_OK).unwrap();
        assert_eq!(plugin.format, WasmSerializationFormat::Yaml);
    }

    #[test]
    fn test_with_format_can_be_set_to_json() {
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_format(WasmSerializationFormat::Json);
        assert_eq!(plugin.format, WasmSerializationFormat::Json);
    }

    // -------------------------------------------------------------------------
    // Plugin::handlers()
    // -------------------------------------------------------------------------

    #[test]
    fn test_handlers_reflects_function_map() {
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_function_map("my_command", "ok_handler");
        let handlers = plugin.handlers();
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].0, "my_command");
    }

    #[test]
    fn test_handlers_empty_without_function_map() {
        let plugin = load_wat(WAT_MINIMAL_OK).unwrap();
        assert_eq!(plugin.handlers().len(), 0);
    }

    // -------------------------------------------------------------------------
    // Execution — success path
    // -------------------------------------------------------------------------

    #[test]
    fn test_execute_success_returns_ok() {
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_function_map("cmd", "ok_handler");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let args = ParsedArgs::from_scalars(HashMap::new());
        assert!(handler.execute(&mut ctx, &args).is_ok());
    }

    #[test]
    fn test_execute_with_args_serializes_without_error() {
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_function_map("cmd", "ok_handler");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());
        args.insert("count".to_string(), "3".to_string());
        let args = ParsedArgs::from_scalars(args);
        assert!(handler.execute(&mut ctx, &args).is_ok());
    }

    #[test]
    fn test_execute_repeated_calls_do_not_exhaust_guest() {
        // Verifies that dcli_dealloc being called on every exit path allows
        // the same handler to be invoked many times without issue — a
        // regression test for the "always deallocate" requirement.
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_function_map("cmd", "ok_handler");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        for _ in 0..50 {
            let args = ParsedArgs::from_scalars(HashMap::new());
            assert!(handler.execute(&mut ctx, &args).is_ok());
        }
    }

    // -------------------------------------------------------------------------
    // Execution — error path
    // -------------------------------------------------------------------------

    #[test]
    fn test_execute_guest_error_without_message() {
        let plugin = load_wat(WAT_MINIMAL_ERROR)
            .unwrap()
            .with_function_map("cmd", "err_handler");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let args = ParsedArgs::from_scalars(HashMap::new());
        let result = handler.execute(&mut ctx, &args);
        assert!(result.is_err());

        match result.unwrap_err() {
            crate::error::DynamicCliError::Wasm(WasmError::GuestError { code, message }) => {
                assert_eq!(code, 1);
                assert!(message.is_none());
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn test_execute_guest_error_with_message() {
        let plugin = load_wat(WAT_WITH_ERROR_MESSAGE)
            .unwrap()
            .with_function_map("cmd", "err_handler");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let args = ParsedArgs::from_scalars(HashMap::new());
        let result = handler.execute(&mut ctx, &args);
        assert!(result.is_err());

        match result.unwrap_err() {
            crate::error::DynamicCliError::Wasm(WasmError::GuestError { code, message }) => {
                assert_eq!(code, 1);
                assert_eq!(message, Some("boom".to_string()));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn test_execute_unmapped_function_name_fails() {
        let plugin = load_wat(WAT_MINIMAL_OK)
            .unwrap()
            .with_function_map("cmd", "does_not_exist");
        let handlers = plugin.handlers();
        let (_, handler) = &handlers[0];

        let mut ctx = TestContext;
        let args = ParsedArgs::from_scalars(HashMap::new());
        assert!(handler.execute(&mut ctx, &args).is_err());
    }

    // -------------------------------------------------------------------------
    // Thread safety
    // -------------------------------------------------------------------------

    #[test]
    fn test_wasm_plugin_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WasmPlugin>();
    }
}
