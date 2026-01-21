//! FFI bindings for cost tracking.
//!
//! Provides C-compatible bindings for:
//! - CostTracker: Accumulates token usage and costs
//! - TokenUsage: Token counts for a request
//! - ModelSpec: Model definitions with pricing
//!
//! These integrate with the Go budget package to provide
//! shared cost calculation and tracking logic.

use crate::llm::{CostTracker, ModelSpec, TokenUsage};
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::c_char;

// ============================================================================
// CostTracker FFI
// ============================================================================

/// Opaque handle for CostTracker.
pub struct RlmCostTracker(CostTracker);

/// Create a new cost tracker.
///
/// # Safety
/// The returned tracker must be freed with `rlm_cost_tracker_free()`.
#[no_mangle]
pub extern "C" fn rlm_cost_tracker_new() -> *mut RlmCostTracker {
    Box::into_raw(Box::new(RlmCostTracker(CostTracker::new())))
}

/// Free a cost tracker.
///
/// # Safety
/// - `tracker` must be a pointer returned by `rlm_cost_tracker_new()`, or NULL.
/// - After calling this function, `tracker` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_free(tracker: *mut RlmCostTracker) {
    if !tracker.is_null() {
        drop(Box::from_raw(tracker));
    }
}

/// Record token usage from a completion.
///
/// # Safety
/// - `tracker` must be a valid pointer.
/// - `model` must be a valid null-terminated string.
/// - `cost` is optional; pass negative value if not known.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_record(
    tracker: *mut RlmCostTracker,
    model: *const c_char,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_creation_tokens: u64,
    cost: f64,
) -> i32 {
    if tracker.is_null() || model.is_null() {
        return -1;
    }

    let model_str = match CStr::from_ptr(model).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let usage = TokenUsage {
        input_tokens,
        output_tokens,
        cache_read_tokens: if cache_read_tokens > 0 {
            Some(cache_read_tokens)
        } else {
            None
        },
        cache_creation_tokens: if cache_creation_tokens > 0 {
            Some(cache_creation_tokens)
        } else {
            None
        },
    };

    let cost_opt = if cost >= 0.0 { Some(cost) } else { None };

    (*tracker).0.record(model_str, &usage, cost_opt);
    0
}

/// Merge another tracker into this one.
///
/// # Safety
/// - Both `tracker` and `other` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_merge(
    tracker: *mut RlmCostTracker,
    other: *const RlmCostTracker,
) -> i32 {
    if tracker.is_null() || other.is_null() {
        return -1;
    }
    (*tracker).0.merge(&(*other).0);
    0
}

/// Get total input tokens.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_total_input_tokens(
    tracker: *const RlmCostTracker,
) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    (*tracker).0.total_input_tokens
}

/// Get total output tokens.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_total_output_tokens(
    tracker: *const RlmCostTracker,
) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    (*tracker).0.total_output_tokens
}

/// Get total cache read tokens.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_total_cache_read_tokens(
    tracker: *const RlmCostTracker,
) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    (*tracker).0.total_cache_read_tokens
}

/// Get total cache creation tokens.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_total_cache_creation_tokens(
    tracker: *const RlmCostTracker,
) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    (*tracker).0.total_cache_creation_tokens
}

/// Get total cost in USD.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_total_cost(tracker: *const RlmCostTracker) -> f64 {
    if tracker.is_null() {
        return 0.0;
    }
    (*tracker).0.total_cost
}

/// Get request count.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_request_count(tracker: *const RlmCostTracker) -> u64 {
    if tracker.is_null() {
        return 0;
    }
    (*tracker).0.request_count
}

/// Get per-model cost breakdown as JSON.
///
/// # Safety
/// - `tracker` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_by_model_json(
    tracker: *const RlmCostTracker,
) -> *mut c_char {
    if tracker.is_null() {
        return std::ptr::null_mut();
    }

    match serde_json::to_string(&(*tracker).0.by_model) {
        Ok(json) => match CString::new(json) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Serialize tracker to JSON.
///
/// # Safety
/// - `tracker` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_to_json(tracker: *const RlmCostTracker) -> *mut c_char {
    if tracker.is_null() {
        return std::ptr::null_mut();
    }

    match serde_json::to_string(&(*tracker).0) {
        Ok(json) => match CString::new(json) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize tracker from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated string.
/// - The returned tracker must be freed with `rlm_cost_tracker_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_cost_tracker_from_json(json: *const c_char) -> *mut RlmCostTracker {
    if json.is_null() {
        return std::ptr::null_mut();
    }

    let json_str = match CStr::from_ptr(json).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match serde_json::from_str::<CostTracker>(json_str) {
        Ok(tracker) => Box::into_raw(Box::new(RlmCostTracker(tracker))),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// ModelSpec FFI - Cost calculation helpers
// ============================================================================

/// Calculate cost for given token usage with a model spec.
///
/// # Safety
/// - `model_json` must be a valid null-terminated JSON string describing a ModelSpec.
///
/// Returns the cost in USD, or -1.0 on error.
#[no_mangle]
pub unsafe extern "C" fn rlm_calculate_cost(
    model_json: *const c_char,
    input_tokens: u64,
    output_tokens: u64,
) -> f64 {
    if model_json.is_null() {
        return -1.0;
    }

    let json_str = match CStr::from_ptr(model_json).to_str() {
        Ok(s) => s,
        Err(_) => return -1.0,
    };

    let model: ModelSpec = match serde_json::from_str(json_str) {
        Ok(m) => m,
        Err(_) => return -1.0,
    };

    model.calculate_cost(input_tokens, output_tokens)
}

/// Calculate cost using well-known model names.
///
/// Supported models: "claude-opus", "claude-sonnet", "claude-haiku",
/// "gpt-4o", "gpt-4o-mini"
///
/// # Safety
/// - `model_name` must be a valid null-terminated string.
///
/// Returns the cost in USD, or -1.0 on error (unknown model).
#[no_mangle]
pub unsafe extern "C" fn rlm_calculate_cost_by_name(
    model_name: *const c_char,
    input_tokens: u64,
    output_tokens: u64,
) -> f64 {
    if model_name.is_null() {
        return -1.0;
    }

    let name = match CStr::from_ptr(model_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1.0,
    };

    let model = match name {
        "claude-opus" | "claude-3-opus" => ModelSpec::claude_opus(),
        "claude-sonnet" | "claude-3-5-sonnet" | "claude-3-sonnet" => ModelSpec::claude_sonnet(),
        "claude-haiku" | "claude-3-5-haiku" | "claude-3-haiku" => ModelSpec::claude_haiku(),
        "gpt-4o" => ModelSpec::gpt4o(),
        "gpt-4o-mini" => ModelSpec::gpt4o_mini(),
        _ => return -1.0,
    };

    model.calculate_cost(input_tokens, output_tokens)
}

/// Get default model spec JSON for a well-known model.
///
/// Supported models: "claude-opus", "claude-sonnet", "claude-haiku",
/// "gpt-4o", "gpt-4o-mini"
///
/// # Safety
/// - `model_name` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_model_spec_json(model_name: *const c_char) -> *mut c_char {
    if model_name.is_null() {
        return std::ptr::null_mut();
    }

    let name = match CStr::from_ptr(model_name).to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let model = match name {
        "claude-opus" | "claude-3-opus" => ModelSpec::claude_opus(),
        "claude-sonnet" | "claude-3-5-sonnet" | "claude-3-sonnet" => ModelSpec::claude_sonnet(),
        "claude-haiku" | "claude-3-5-haiku" | "claude-3-haiku" => ModelSpec::claude_haiku(),
        "gpt-4o" => ModelSpec::gpt4o(),
        "gpt-4o-mini" => ModelSpec::gpt4o_mini(),
        _ => return std::ptr::null_mut(),
    };

    match serde_json::to_string(&model) {
        Ok(json) => match CString::new(json) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// TokenUsage FFI - Helpers
// ============================================================================

/// Calculate effective input tokens accounting for cache reads.
/// Cache reads are typically 90% cheaper, so we count them at 10%.
///
/// Returns: input_tokens - cache_read_tokens + (cache_read_tokens / 10)
#[no_mangle]
pub extern "C" fn rlm_effective_input_tokens(input_tokens: u64, cache_read_tokens: u64) -> u64 {
    if cache_read_tokens == 0 {
        return input_tokens;
    }
    input_tokens - cache_read_tokens + (cache_read_tokens / 10)
}
