//! FFI bindings for the orchestrator module.
//!
//! Provides C-compatible bindings for:
//! - ExecutionMode: Mode selection for orchestration (Micro, Fast, Balanced, Thorough)
//! - OrchestratorConfig: Configuration for orchestration behavior
//! - OrchestratorBuilder: Builder pattern for creating configs
//!
//! Note: The actual Orchestrator trait involves async LLM calls and streams,
//! which remain in language-specific implementations. This module exposes
//! the pure configuration and mode selection logic.

use crate::complexity::TaskComplexitySignals;
use crate::orchestrator::{ExecutionMode, OrchestratorBuilder, OrchestratorConfig};
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;

// ============================================================================
// ExecutionMode FFI
// ============================================================================

/// Execution mode enum for C FFI.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlmExecutionMode {
    Micro = 0,
    Fast = 1,
    Balanced = 2,
    Thorough = 3,
}

impl From<ExecutionMode> for RlmExecutionMode {
    fn from(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Micro => RlmExecutionMode::Micro,
            ExecutionMode::Fast => RlmExecutionMode::Fast,
            ExecutionMode::Balanced => RlmExecutionMode::Balanced,
            ExecutionMode::Thorough => RlmExecutionMode::Thorough,
        }
    }
}

impl From<RlmExecutionMode> for ExecutionMode {
    fn from(mode: RlmExecutionMode) -> Self {
        match mode {
            RlmExecutionMode::Micro => ExecutionMode::Micro,
            RlmExecutionMode::Fast => ExecutionMode::Fast,
            RlmExecutionMode::Balanced => ExecutionMode::Balanced,
            RlmExecutionMode::Thorough => ExecutionMode::Thorough,
        }
    }
}

/// Get the typical cost budget in USD for an execution mode.
///
/// # Safety
/// This function is safe to call with any valid RlmExecutionMode value.
#[no_mangle]
pub extern "C" fn rlm_execution_mode_budget_usd(mode: RlmExecutionMode) -> f64 {
    let mode: ExecutionMode = mode.into();
    mode.typical_budget_usd()
}

/// Get the maximum recursion depth for an execution mode.
///
/// # Safety
/// This function is safe to call with any valid RlmExecutionMode value.
#[no_mangle]
pub extern "C" fn rlm_execution_mode_max_depth(mode: RlmExecutionMode) -> u32 {
    let mode: ExecutionMode = mode.into();
    mode.max_depth()
}

/// Get the display name for an execution mode.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub extern "C" fn rlm_execution_mode_name(mode: RlmExecutionMode) -> *mut c_char {
    let mode: ExecutionMode = mode.into();
    let name = mode.to_string();
    match CString::new(name) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Select execution mode based on complexity signals.
///
/// # Safety
/// - `signals_json` must be a valid null-terminated JSON string, or NULL.
///
/// If signals_json is NULL, returns Micro mode as the default.
/// The JSON should contain complexity signal fields like:
/// - debugging_task, multi_file_refs, architecture_analysis
/// - user_wants_fast, user_wants_thorough, requires_exhaustive_search
#[no_mangle]
pub unsafe extern "C" fn rlm_execution_mode_from_signals(
    signals_json: *const c_char,
) -> RlmExecutionMode {
    if signals_json.is_null() {
        return RlmExecutionMode::Micro;
    }

    let json_str = match CStr::from_ptr(signals_json).to_str() {
        Ok(s) => s,
        Err(_) => return RlmExecutionMode::Micro,
    };

    let signals: TaskComplexitySignals = match serde_json::from_str(json_str) {
        Ok(s) => s,
        Err(_) => return RlmExecutionMode::Micro,
    };

    ExecutionMode::from_signals(&signals).into()
}

// ============================================================================
// OrchestratorConfig FFI
// ============================================================================

/// Opaque handle for OrchestratorConfig.
pub struct RlmOrchestratorConfig(OrchestratorConfig);

/// Create a new orchestrator config with default values.
///
/// # Safety
/// The returned config must be freed with `rlm_orchestrator_config_free()`.
#[no_mangle]
pub extern "C" fn rlm_orchestrator_config_default() -> *mut RlmOrchestratorConfig {
    Box::into_raw(Box::new(RlmOrchestratorConfig(
        OrchestratorConfig::default(),
    )))
}

/// Free an orchestrator config.
///
/// # Safety
/// - `config` must be a pointer returned by an rlm_orchestrator_config_* function, or NULL.
/// - After calling this function, `config` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_free(config: *mut RlmOrchestratorConfig) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Get the max depth from config.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_max_depth(
    config: *const RlmOrchestratorConfig,
) -> u32 {
    if config.is_null() {
        return 3; // default
    }
    (*config).0.max_depth
}

/// Get whether REPL spawning is enabled by default.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_default_spawn_repl(
    config: *const RlmOrchestratorConfig,
) -> i32 {
    if config.is_null() {
        return 1; // default true
    }
    if (*config).0.default_spawn_repl {
        1
    } else {
        0
    }
}

/// Get the REPL timeout in milliseconds.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_repl_timeout_ms(
    config: *const RlmOrchestratorConfig,
) -> u64 {
    if config.is_null() {
        return 30_000; // default
    }
    (*config).0.repl_timeout_ms
}

/// Get the max tokens per call.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_max_tokens_per_call(
    config: *const RlmOrchestratorConfig,
) -> u64 {
    if config.is_null() {
        return 4096; // default
    }
    (*config).0.max_tokens_per_call
}

/// Get the total token budget.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_total_token_budget(
    config: *const RlmOrchestratorConfig,
) -> u64 {
    if config.is_null() {
        return 100_000; // default
    }
    (*config).0.total_token_budget
}

/// Get the cost budget in USD.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_cost_budget_usd(
    config: *const RlmOrchestratorConfig,
) -> f64 {
    if config.is_null() {
        return 1.0; // default
    }
    (*config).0.cost_budget_usd
}

/// Serialize config to JSON.
///
/// # Safety
/// - `config` must be a valid pointer or NULL.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_to_json(
    config: *const RlmOrchestratorConfig,
) -> *mut c_char {
    if config.is_null() {
        return std::ptr::null_mut();
    }

    match serde_json::to_string(&(*config).0) {
        Ok(json) => match CString::new(json) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize config from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated JSON string.
/// - The returned config must be freed with `rlm_orchestrator_config_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_config_from_json(
    json: *const c_char,
) -> *mut RlmOrchestratorConfig {
    if json.is_null() {
        return std::ptr::null_mut();
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match serde_json::from_str::<OrchestratorConfig>(json_str) {
        Ok(config) => Box::into_raw(Box::new(RlmOrchestratorConfig(config))),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// OrchestratorBuilder FFI
// ============================================================================

/// Opaque handle for OrchestratorBuilder.
pub struct RlmOrchestratorBuilder(OrchestratorBuilder);

/// Create a new orchestrator builder with default values.
///
/// # Safety
/// The returned builder must be freed with `rlm_orchestrator_builder_free()`.
#[no_mangle]
pub extern "C" fn rlm_orchestrator_builder_new() -> *mut RlmOrchestratorBuilder {
    Box::into_raw(Box::new(RlmOrchestratorBuilder(OrchestratorBuilder::new())))
}

/// Free an orchestrator builder.
///
/// # Safety
/// - `builder` must be a pointer returned by an rlm_orchestrator_builder_* function, or NULL.
/// - After calling this function, `builder` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_free(builder: *mut RlmOrchestratorBuilder) {
    if !builder.is_null() {
        drop(Box::from_raw(builder));
    }
}

/// Set the maximum recursion depth.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_max_depth(
    builder: *mut RlmOrchestratorBuilder,
    depth: u32,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(b.0.max_depth(depth))))
}

/// Set whether to spawn REPL by default.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_default_spawn_repl(
    builder: *mut RlmOrchestratorBuilder,
    spawn: i32,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(
        b.0.default_spawn_repl(spawn != 0),
    )))
}

/// Set the REPL timeout in milliseconds.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_repl_timeout_ms(
    builder: *mut RlmOrchestratorBuilder,
    timeout: u64,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(
        b.0.repl_timeout_ms(timeout),
    )))
}

/// Set the total token budget.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_total_token_budget(
    builder: *mut RlmOrchestratorBuilder,
    budget: u64,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(
        b.0.total_token_budget(budget),
    )))
}

/// Set the cost budget in USD.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_cost_budget_usd(
    builder: *mut RlmOrchestratorBuilder,
    budget: f64,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(
        b.0.cost_budget_usd(budget),
    )))
}

/// Set the execution mode.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - Returns a new builder pointer; the old one is consumed.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_execution_mode(
    builder: *mut RlmOrchestratorBuilder,
    mode: RlmExecutionMode,
) -> *mut RlmOrchestratorBuilder {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorBuilder(
        b.0.execution_mode(mode.into()),
    )))
}

/// Build the config from the builder.
///
/// # Safety
/// - `builder` must be a valid pointer.
/// - The builder is consumed; don't use it after this call.
/// - The returned config must be freed with `rlm_orchestrator_config_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_build(
    builder: *mut RlmOrchestratorBuilder,
) -> *mut RlmOrchestratorConfig {
    if builder.is_null() {
        return std::ptr::null_mut();
    }
    let b = Box::from_raw(builder);
    Box::into_raw(Box::new(RlmOrchestratorConfig(b.0.build_config())))
}

/// Get the execution mode from the builder.
///
/// # Safety
/// - `builder` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn rlm_orchestrator_builder_get_mode(
    builder: *const RlmOrchestratorBuilder,
) -> RlmExecutionMode {
    if builder.is_null() {
        return RlmExecutionMode::Balanced; // default
    }
    (*builder).0.get_mode().into()
}

// ============================================================================
// TaskComplexitySignals FFI
// ============================================================================

/// Create complexity signals from JSON.
///
/// Returns a JSON string with the signals after parsing and validation.
/// This can be used to verify signal parsing before mode selection.
///
/// # Safety
/// - `json` must be a valid null-terminated JSON string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_complexity_signals_parse(json: *const c_char) -> *mut c_char {
    if json.is_null() {
        return std::ptr::null_mut();
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let signals: TaskComplexitySignals = match serde_json::from_str(json_str) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match serde_json::to_string(&signals) {
        Ok(json) => match CString::new(json) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the complexity score from signals JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated JSON string, or NULL.
///
/// Returns the score, or 0 if parsing fails.
#[no_mangle]
pub unsafe extern "C" fn rlm_complexity_signals_score(json: *const c_char) -> i32 {
    if json.is_null() {
        return 0;
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let signals: TaskComplexitySignals = match serde_json::from_str(json_str) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    signals.score()
}

/// Check if signals have a strong signal (above threshold).
///
/// # Safety
/// - `json` must be a valid null-terminated JSON string, or NULL.
///
/// Returns 1 if strong signal present, 0 otherwise.
#[no_mangle]
pub unsafe extern "C" fn rlm_complexity_signals_has_strong_signal(json: *const c_char) -> i32 {
    if json.is_null() {
        return 0;
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let signals: TaskComplexitySignals = match serde_json::from_str(json_str) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if signals.has_strong_signal() {
        1
    } else {
        0
    }
}
