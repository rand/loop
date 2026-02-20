//! FFI bindings for REPL subprocess management.

use std::os::raw::c_char;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use super::types::{RlmReplHandle, RlmReplPool};
use crate::repl::{ReplConfig, ReplHandle, ReplPool};

// ============================================================================
// ReplConfig
// ============================================================================

/// Create a default REPL configuration.
///
/// Returns a JSON string that can be modified and passed to spawn functions.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub extern "C" fn rlm_repl_config_default() -> *mut c_char {
    let config = ReplConfig::default();
    let json = serde_json::json!({
        "python_path": config.python_path,
        "repl_package_path": config.repl_package_path,
        "timeout_ms": config.timeout_ms,
        "max_memory_bytes": config.max_memory_bytes,
        "max_cpu_seconds": config.max_cpu_seconds,
    });
    match serde_json::to_string(&json) {
        Ok(s) => str_to_cstring(&s),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

// ============================================================================
// ReplHandle
// ============================================================================

/// Spawn a new REPL subprocess with default configuration.
///
/// # Safety
/// The returned pointer must be freed with `rlm_repl_handle_free()`.
#[no_mangle]
pub extern "C" fn rlm_repl_handle_spawn_default() -> *mut RlmReplHandle {
    let config = ReplConfig::default();
    match ReplHandle::spawn(config) {
        Ok(handle) => Box::into_raw(Box::new(RlmReplHandle(handle))),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Spawn a new REPL subprocess with custom configuration.
///
/// # Safety
/// - `config_json` must be a valid JSON string with configuration options.
/// - The returned pointer must be freed with `rlm_repl_handle_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_spawn(config_json: *const c_char) -> *mut RlmReplHandle {
    let json_str = ffi_try!(cstr_to_str(config_json));
    let json: serde_json::Value = ffi_try!(serde_json::from_str(json_str));

    let mut config = ReplConfig::default();

    if let Some(s) = json.get("python_path").and_then(|v| v.as_str()) {
        config.python_path = s.to_string();
    }
    if let Some(s) = json.get("repl_package_path").and_then(|v| v.as_str()) {
        config.repl_package_path = Some(s.to_string());
    }
    if let Some(n) = json.get("timeout_ms").and_then(|v| v.as_u64()) {
        config.timeout_ms = n;
    }
    if let Some(n) = json.get("max_memory_bytes").and_then(|v| v.as_u64()) {
        config.max_memory_bytes = Some(n);
    }
    if let Some(n) = json.get("max_cpu_seconds").and_then(|v| v.as_u64()) {
        config.max_cpu_seconds = Some(n);
    }

    let handle = ffi_try!(ReplHandle::spawn(config));
    Box::into_raw(Box::new(RlmReplHandle(handle)))
}

/// Free a REPL handle.
///
/// # Safety
/// - `handle` must be a valid pointer or NULL.
/// - After calling this function, `handle` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_free(handle: *mut RlmReplHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Execute Python code in the REPL.
///
/// Returns a JSON string with the execution result.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - `code` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_execute(
    handle: *mut RlmReplHandle,
    code: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return std::ptr::null_mut();
    }
    let code = ffi_try!(cstr_to_str(code));
    let result = ffi_try!((*handle).0.execute(code));
    let json = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json)
}

/// Get a variable from the REPL namespace.
///
/// Returns a JSON string with the variable value.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - `name` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_get_variable(
    handle: *mut RlmReplHandle,
    name: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return std::ptr::null_mut();
    }
    let name = ffi_try!(cstr_to_str(name));
    let value = ffi_try!((*handle).0.get_variable(name));
    let json = ffi_try!(serde_json::to_string(&value));
    str_to_cstring(&json)
}

/// Set a variable in the REPL namespace.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - `name` must be a valid null-terminated string.
/// - `value_json` must be a valid JSON string.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_set_variable(
    handle: *mut RlmReplHandle,
    name: *const c_char,
    value_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return -1;
    }
    let name = ffi_try!(cstr_to_str(name), -1);
    let value_str = ffi_try!(cstr_to_str(value_json), -1);
    let value: serde_json::Value = ffi_try!(serde_json::from_str(value_str), -1);
    ffi_try!((*handle).0.set_variable(name, value), -1);
    0
}

/// Resolve a deferred operation.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - `operation_id` must be a valid null-terminated string.
/// - `result_json` must be a valid JSON string.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_resolve_operation(
    handle: *mut RlmReplHandle,
    operation_id: *const c_char,
    result_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return -1;
    }
    let op_id = ffi_try!(cstr_to_str(operation_id), -1);
    let result_str = ffi_try!(cstr_to_str(result_json), -1);
    let result: serde_json::Value = ffi_try!(serde_json::from_str(result_str), -1);
    ffi_try!((*handle).0.resolve_operation(op_id, result), -1);
    0
}

/// List all variables in the REPL namespace.
///
/// Returns a JSON object mapping variable names to their type descriptions.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_list_variables(handle: *mut RlmReplHandle) -> *mut c_char {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return std::ptr::null_mut();
    }
    let vars = ffi_try!((*handle).0.list_variables());
    let json = ffi_try!(serde_json::to_string(&vars));
    str_to_cstring(&json)
}

/// Get REPL status.
///
/// Returns a JSON object with status information.
///
/// # Safety
/// - `handle` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_status(handle: *mut RlmReplHandle) -> *mut c_char {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return std::ptr::null_mut();
    }
    let status = ffi_try!((*handle).0.status());
    let json = serde_json::json!({
        "ready": status.ready,
        "pending_operations": status.pending_operations,
        "variables_count": status.variables_count,
        "memory_usage_bytes": status.memory_usage_bytes,
    });
    let json_str = ffi_try!(serde_json::to_string(&json));
    str_to_cstring(&json_str)
}

/// Reset the REPL state.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_reset(handle: *mut RlmReplHandle) -> i32 {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return -1;
    }
    ffi_try!((*handle).0.reset(), -1);
    0
}

/// Shutdown the REPL subprocess.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_shutdown(handle: *mut RlmReplHandle) -> i32 {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return -1;
    }
    ffi_try!((*handle).0.shutdown(), -1);
    0
}

/// Check if the REPL subprocess is still running.
///
/// Returns 1 if alive, 0 if not, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_handle_is_alive(handle: *mut RlmReplHandle) -> i32 {
    if handle.is_null() {
        set_last_error("null handle pointer");
        return -1;
    }
    if (*handle).0.is_alive() {
        1
    } else {
        0
    }
}

// ============================================================================
// ReplPool
// ============================================================================

/// Create a new REPL pool with default configuration.
///
/// # Arguments
/// - `max_size`: Maximum number of REPL handles to keep in the pool.
///
/// # Safety
/// The returned pointer must be freed with `rlm_repl_pool_free()`.
#[no_mangle]
pub extern "C" fn rlm_repl_pool_new_default(max_size: usize) -> *mut RlmReplPool {
    let config = ReplConfig::default();
    let pool = ReplPool::new(config, max_size);
    Box::into_raw(Box::new(RlmReplPool(pool)))
}

/// Create a new REPL pool with custom configuration.
///
/// # Safety
/// - `config_json` must be a valid JSON string with configuration options.
/// - The returned pointer must be freed with `rlm_repl_pool_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_pool_new(
    config_json: *const c_char,
    max_size: usize,
) -> *mut RlmReplPool {
    let json_str = ffi_try!(cstr_to_str(config_json));
    let json: serde_json::Value = ffi_try!(serde_json::from_str(json_str));

    let mut config = ReplConfig::default();

    if let Some(s) = json.get("python_path").and_then(|v| v.as_str()) {
        config.python_path = s.to_string();
    }
    if let Some(s) = json.get("repl_package_path").and_then(|v| v.as_str()) {
        config.repl_package_path = Some(s.to_string());
    }
    if let Some(n) = json.get("timeout_ms").and_then(|v| v.as_u64()) {
        config.timeout_ms = n;
    }
    if let Some(n) = json.get("max_memory_bytes").and_then(|v| v.as_u64()) {
        config.max_memory_bytes = Some(n);
    }
    if let Some(n) = json.get("max_cpu_seconds").and_then(|v| v.as_u64()) {
        config.max_cpu_seconds = Some(n);
    }

    let pool = ReplPool::new(config, max_size);
    Box::into_raw(Box::new(RlmReplPool(pool)))
}

/// Free a REPL pool.
///
/// # Safety
/// - `pool` must be a valid pointer or NULL.
/// - After calling this function, `pool` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_pool_free(pool: *mut RlmReplPool) {
    if !pool.is_null() {
        drop(Box::from_raw(pool));
    }
}

/// Acquire a REPL handle from the pool.
///
/// This will either return an existing idle handle or spawn a new one.
///
/// # Safety
/// - `pool` must be a valid pointer.
/// - The returned pointer must be freed with `rlm_repl_handle_free()` or
///   returned to the pool with `rlm_repl_pool_release()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_pool_acquire(pool: *const RlmReplPool) -> *mut RlmReplHandle {
    if pool.is_null() {
        set_last_error("null pool pointer");
        return std::ptr::null_mut();
    }
    let handle = ffi_try!((*pool).0.acquire());
    Box::into_raw(Box::new(RlmReplHandle(handle)))
}

/// Release a REPL handle back to the pool.
///
/// # Safety
/// - `pool` must be a valid pointer.
/// - `handle` must be a valid pointer.
/// - After calling this function, `handle` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_repl_pool_release(
    pool: *const RlmReplPool,
    handle: *mut RlmReplHandle,
) {
    if pool.is_null() || handle.is_null() {
        return;
    }
    let handle = Box::from_raw(handle);
    (*pool).0.release(handle.0);
}
