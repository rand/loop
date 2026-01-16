//! FFI bindings for trajectory types.

use std::os::raw::c_char;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use super::types::{RlmTrajectoryEvent, RlmTrajectoryEventType};
use crate::trajectory::TrajectoryEvent;

// ============================================================================
// TrajectoryEvent
// ============================================================================

/// Create a new trajectory event.
///
/// # Safety
/// - `content` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_trajectory_event_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_new(
    event_type: RlmTrajectoryEventType,
    depth: u32,
    content: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let content = ffi_try!(cstr_to_str(content));
    let event = TrajectoryEvent::new(event_type.into(), depth, content);
    Box::into_raw(Box::new(RlmTrajectoryEvent(event)))
}

/// Create an RLM start event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_rlm_start(
    query: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let query = ffi_try!(cstr_to_str(query));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::rlm_start(query))))
}

/// Create an analyze event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_analyze(
    depth: u32,
    analysis: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let analysis = ffi_try!(cstr_to_str(analysis));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::analyze(depth, analysis))))
}

/// Create a REPL exec event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_repl_exec(
    depth: u32,
    code: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let code = ffi_try!(cstr_to_str(code));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::repl_exec(depth, code))))
}

/// Create a REPL result event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_repl_result(
    depth: u32,
    result: *const c_char,
    success: i32,
) -> *mut RlmTrajectoryEvent {
    let result = ffi_try!(cstr_to_str(result));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::repl_result(
        depth,
        result,
        success != 0,
    ))))
}

/// Create a reason event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_reason(
    depth: u32,
    reasoning: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let reasoning = ffi_try!(cstr_to_str(reasoning));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::reason(depth, reasoning))))
}

/// Create a recurse start event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_recurse_start(
    depth: u32,
    query: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let query = ffi_try!(cstr_to_str(query));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::recurse_start(depth, query))))
}

/// Create a recurse end event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_recurse_end(
    depth: u32,
    result: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let result = ffi_try!(cstr_to_str(result));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::recurse_end(depth, result))))
}

/// Create a final answer event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_final_answer(
    depth: u32,
    answer: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let answer = ffi_try!(cstr_to_str(answer));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::final_answer(depth, answer))))
}

/// Create an error event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_error(
    depth: u32,
    error: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let error = ffi_try!(cstr_to_str(error));
    Box::into_raw(Box::new(RlmTrajectoryEvent(TrajectoryEvent::error(depth, error))))
}

/// Free a trajectory event.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_free(event: *mut RlmTrajectoryEvent) {
    if !event.is_null() {
        drop(Box::from_raw(event));
    }
}

/// Get the event type.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_type(
    event: *const RlmTrajectoryEvent,
) -> RlmTrajectoryEventType {
    if event.is_null() {
        return RlmTrajectoryEventType::Error;
    }
    RlmTrajectoryEventType::from((*event).0.event_type)
}

/// Get the event depth.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_depth(event: *const RlmTrajectoryEvent) -> u32 {
    if event.is_null() {
        return 0;
    }
    (*event).0.depth
}

/// Get the event content.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_content(
    event: *const RlmTrajectoryEvent,
) -> *mut c_char {
    if event.is_null() {
        set_last_error("null event pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*event).0.content)
}

/// Get the event timestamp (RFC3339 format).
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_timestamp(
    event: *const RlmTrajectoryEvent,
) -> *mut c_char {
    if event.is_null() {
        set_last_error("null event pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*event).0.timestamp.to_rfc3339())
}

/// Format the event as a log line.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_log_line(
    event: *const RlmTrajectoryEvent,
) -> *mut c_char {
    if event.is_null() {
        set_last_error("null event pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*event).0.as_log_line())
}

/// Check if the event is an error.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_is_error(event: *const RlmTrajectoryEvent) -> i32 {
    if event.is_null() {
        return 0;
    }
    if (*event).0.is_error() { 1 } else { 0 }
}

/// Check if the event is a final answer.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_is_final(event: *const RlmTrajectoryEvent) -> i32 {
    if event.is_null() {
        return 0;
    }
    if (*event).0.is_final() { 1 } else { 0 }
}

/// Serialize event to JSON.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_to_json(
    event: *const RlmTrajectoryEvent,
) -> *mut c_char {
    if event.is_null() {
        set_last_error("null event pointer");
        return std::ptr::null_mut();
    }
    let json = ffi_try!(serde_json::to_string(&(*event).0));
    str_to_cstring(&json)
}

/// Deserialize event from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_trajectory_event_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_trajectory_event_from_json(
    json: *const c_char,
) -> *mut RlmTrajectoryEvent {
    let json = ffi_try!(cstr_to_str(json));
    let event: TrajectoryEvent = ffi_try!(serde_json::from_str(json));
    Box::into_raw(Box::new(RlmTrajectoryEvent(event)))
}

// ============================================================================
// Event type utilities
// ============================================================================

/// Get the string name of an event type.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub extern "C" fn rlm_trajectory_event_type_name(
    event_type: RlmTrajectoryEventType,
) -> *mut c_char {
    let name = match event_type {
        RlmTrajectoryEventType::RlmStart => "RLM_START",
        RlmTrajectoryEventType::Analyze => "ANALYZE",
        RlmTrajectoryEventType::ReplExec => "REPL_EXEC",
        RlmTrajectoryEventType::ReplResult => "REPL_RESULT",
        RlmTrajectoryEventType::Reason => "REASON",
        RlmTrajectoryEventType::RecurseStart => "RECURSE_START",
        RlmTrajectoryEventType::RecurseEnd => "RECURSE_END",
        RlmTrajectoryEventType::Final => "FINAL",
        RlmTrajectoryEventType::Error => "ERROR",
        RlmTrajectoryEventType::ToolUse => "TOOL_USE",
        RlmTrajectoryEventType::CostReport => "COST_REPORT",
        RlmTrajectoryEventType::VerifyStart => "VERIFY_START",
        RlmTrajectoryEventType::ClaimExtracted => "CLAIM_EXTRACTED",
        RlmTrajectoryEventType::EvidenceChecked => "EVIDENCE_CHECKED",
        RlmTrajectoryEventType::BudgetComputed => "BUDGET_COMPUTED",
        RlmTrajectoryEventType::HallucinationFlag => "HALLUCINATION_FLAG",
        RlmTrajectoryEventType::VerifyComplete => "VERIFY_COMPLETE",
        RlmTrajectoryEventType::Memory => "MEMORY",
        RlmTrajectoryEventType::Externalize => "EXTERNALIZE",
        RlmTrajectoryEventType::Decompose => "DECOMPOSE",
        RlmTrajectoryEventType::Synthesize => "SYNTHESIZE",
    };
    str_to_cstring(name)
}
