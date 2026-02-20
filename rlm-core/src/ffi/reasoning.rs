//! FFI bindings for reasoning traces (Deciduous-style provenance tracking).
//!
//! This module exposes the reasoning trace system for use via CGO.
//! Provides creation, manipulation, analysis, and storage of decision trees.

use std::os::raw::c_char;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use crate::reasoning::{
    DecisionNodeId, ReasoningTrace, ReasoningTraceStore, TraceAnalyzer, TraceId,
};

// ============================================================================
// ReasoningTrace
// ============================================================================

/// Opaque handle for ReasoningTrace.
pub struct RlmReasoningTrace(ReasoningTrace);

/// Create a new reasoning trace with a root goal.
///
/// # Safety
/// - `goal` and `session_id` must be valid null-terminated strings.
/// - The returned pointer must be freed with `rlm_reasoning_trace_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_new(
    goal: *const c_char,
    session_id: *const c_char,
) -> *mut RlmReasoningTrace {
    let goal = match cstr_to_str(goal) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };
    let session_id = match cstr_to_str(session_id) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };

    Box::into_raw(Box::new(RlmReasoningTrace(ReasoningTrace::new(
        goal, session_id,
    ))))
}

/// Free a reasoning trace.
///
/// # Safety
/// - `trace` must be a valid pointer or NULL.
/// - After calling this function, `trace` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_free(trace: *mut RlmReasoningTrace) {
    if !trace.is_null() {
        drop(Box::from_raw(trace));
    }
}

/// Get the trace ID as a string.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_id(trace: *const RlmReasoningTrace) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*trace).0.id.to_string())
}

/// Get the root goal node ID as a string.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_root_id(
    trace: *const RlmReasoningTrace,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*trace).0.root_goal.to_string())
}

/// Link the trace to a git commit.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - `commit` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_set_git_commit(
    trace: *mut RlmReasoningTrace,
    commit: *const c_char,
) -> i32 {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return -1;
    }
    let commit = ffi_try!(cstr_to_str(commit), -1);
    (*trace).0.git_commit = Some(commit.to_string());
    0
}

/// Link the trace to a git commit (alias for rlm_reasoning_trace_set_git_commit).
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - `commit_sha` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_link_commit(
    trace: *mut RlmReasoningTrace,
    commit_sha: *const c_char,
) -> i32 {
    rlm_reasoning_trace_set_git_commit(trace, commit_sha)
}

/// Link the trace to a git branch.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - `branch` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_set_git_branch(
    trace: *mut RlmReasoningTrace,
    branch: *const c_char,
) -> i32 {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return -1;
    }
    let branch = ffi_try!(cstr_to_str(branch), -1);
    (*trace).0.git_branch = Some(branch.to_string());
    0
}

/// Log a decision with options and return the chosen option's node ID.
///
/// Returns a JSON object with `decision_id` and `chosen_id`.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - `parent_id`, `question`, `options_json`, `reason` must be valid null-terminated strings.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_log_decision(
    trace: *mut RlmReasoningTrace,
    parent_id: *const c_char,
    question: *const c_char,
    options_json: *const c_char,
    chosen_index: i32,
    reason: *const c_char,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }

    let parent_id_str = ffi_try!(cstr_to_str(parent_id));
    let parent_id = ffi_try!(DecisionNodeId::parse(parent_id_str));
    let question = ffi_try!(cstr_to_str(question));
    let options_str = ffi_try!(cstr_to_str(options_json));
    let options: Vec<String> = ffi_try!(serde_json::from_str(options_str));
    let reason = ffi_try!(cstr_to_str(reason));

    let options_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
    let chosen_id = (*trace).0.log_decision(
        &parent_id,
        question,
        &options_refs,
        chosen_index as usize,
        reason,
    );

    let result = serde_json::json!({
        "chosen_id": chosen_id.to_string(),
    });
    let json = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json)
}

/// Log an action and its outcome.
///
/// Returns JSON with `action_id` and `outcome_id`.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - All string parameters must be valid null-terminated strings.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_log_action(
    trace: *mut RlmReasoningTrace,
    parent_id: *const c_char,
    action: *const c_char,
    outcome: *const c_char,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }

    let parent_id_str = ffi_try!(cstr_to_str(parent_id));
    let parent_id = ffi_try!(DecisionNodeId::parse(parent_id_str));
    let action = ffi_try!(cstr_to_str(action));
    let outcome = ffi_try!(cstr_to_str(outcome));

    let (action_id, outcome_id) = (*trace).0.log_action(&parent_id, action, outcome);
    let result = serde_json::json!({
        "action_id": action_id.to_string(),
        "outcome_id": outcome_id.to_string(),
    });
    let json = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json)
}

/// Get the number of nodes in the trace.
///
/// # Safety
/// - `trace` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_node_count(trace: *const RlmReasoningTrace) -> i64 {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return -1;
    }
    (*trace).0.nodes.len() as i64
}

/// Get the number of edges in the trace.
///
/// # Safety
/// - `trace` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_edge_count(trace: *const RlmReasoningTrace) -> i64 {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return -1;
    }
    (*trace).0.edges.len() as i64
}

/// Export the trace as JSON.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_to_json(
    trace: *const RlmReasoningTrace,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }
    let json = ffi_try!(serde_json::to_string(&(*trace).0));
    str_to_cstring(&json)
}

/// Import a trace from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_reasoning_trace_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_from_json(
    json: *const c_char,
) -> *mut RlmReasoningTrace {
    let json_str = match cstr_to_str(json) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };
    let trace: ReasoningTrace = match serde_json::from_str(json_str) {
        Ok(t) => t,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };
    Box::into_raw(Box::new(RlmReasoningTrace(trace)))
}

/// Export the trace as a Mermaid diagram.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_to_mermaid(
    trace: *const RlmReasoningTrace,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }
    let mermaid = (*trace).0.to_mermaid();
    str_to_cstring(&mermaid)
}

// ============================================================================
// TraceAnalyzer
// ============================================================================

/// Get trace statistics as JSON.
///
/// Returns JSON with: decision_count, option_count, chosen_count, rejected_count,
/// total_nodes, total_edges, max_depth.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_stats(trace: *const RlmReasoningTrace) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }

    let stats = (*trace).0.stats();

    let result = serde_json::json!({
        "decision_count": stats.decision_count,
        "option_count": stats.option_count,
        "chosen_count": stats.chosen_count,
        "rejected_count": stats.rejected_count,
        "total_nodes": stats.total_nodes,
        "total_edges": stats.total_edges,
        "max_depth": stats.max_depth,
    });

    let json = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json)
}

/// Analyze a trace and return analysis results as JSON.
///
/// Returns JSON with: overall_confidence, decision_count, action_count, outcome_count, narrative.
///
/// # Safety
/// - `trace` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_analyze(
    trace: *const RlmReasoningTrace,
) -> *mut c_char {
    if trace.is_null() {
        set_last_error("null trace pointer");
        return std::ptr::null_mut();
    }

    let analyzer = TraceAnalyzer::new(&(*trace).0);
    let stats = (*trace).0.stats();

    let result = serde_json::json!({
        "overall_confidence": analyzer.overall_confidence(),
        "decision_count": stats.decision_count,
        "option_count": stats.option_count,
        "chosen_count": stats.chosen_count,
        "rejected_count": stats.rejected_count,
        "total_nodes": stats.total_nodes,
        "total_edges": stats.total_edges,
        "max_depth": stats.max_depth,
        "narrative": analyzer.narrative(),
    });

    let json = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json)
}

// ============================================================================
// ReasoningTraceStore
// ============================================================================

/// Opaque handle for ReasoningTraceStore.
pub struct RlmReasoningTraceStore(ReasoningTraceStore);

/// Create an in-memory reasoning trace store.
///
/// # Safety
/// - The returned pointer must be freed with `rlm_reasoning_trace_store_free()`.
#[no_mangle]
pub extern "C" fn rlm_reasoning_trace_store_in_memory() -> *mut RlmReasoningTraceStore {
    match ReasoningTraceStore::in_memory() {
        Ok(store) => Box::into_raw(Box::new(RlmReasoningTraceStore(store))),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Create a reasoning trace store backed by a file.
///
/// # Safety
/// - `path` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_reasoning_trace_store_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_open(
    path: *const c_char,
) -> *mut RlmReasoningTraceStore {
    let path = match cstr_to_str(path) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };

    match crate::memory::SqliteMemoryStore::open(path) {
        Ok(memory) => Box::into_raw(Box::new(RlmReasoningTraceStore(ReasoningTraceStore::new(
            memory,
        )))),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Free a reasoning trace store.
///
/// # Safety
/// - `store` must be a valid pointer or NULL.
/// - After calling this function, `store` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_free(store: *mut RlmReasoningTraceStore) {
    if !store.is_null() {
        drop(Box::from_raw(store));
    }
}

/// Save a trace to the store.
///
/// # Safety
/// - `store` and `trace` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_save(
    store: *const RlmReasoningTraceStore,
    trace: *const RlmReasoningTrace,
) -> i32 {
    if store.is_null() {
        set_last_error("null store pointer");
        return -1;
    }
    if trace.is_null() {
        set_last_error("null trace pointer");
        return -1;
    }

    match (*store).0.save_trace(&(*trace).0) {
        Ok(()) => 0,
        Err(e) => {
            set_last_error(&e.to_string());
            -1
        }
    }
}

/// Load a trace from the store by ID.
///
/// # Safety
/// - `store` must be a valid pointer.
/// - `trace_id` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_reasoning_trace_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_load(
    store: *const RlmReasoningTraceStore,
    trace_id: *const c_char,
) -> *mut RlmReasoningTrace {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }

    let trace_id_str = match cstr_to_str(trace_id) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };

    let id = match TraceId::parse(trace_id_str) {
        Ok(id) => id,
        Err(e) => {
            set_last_error(&e.to_string());
            return std::ptr::null_mut();
        }
    };

    match (*store).0.load_trace(&id) {
        Ok(Some(trace)) => Box::into_raw(Box::new(RlmReasoningTrace(trace))),
        Ok(None) => {
            set_last_error("trace not found");
            std::ptr::null_mut()
        }
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Query traces by session ID.
///
/// Returns a JSON array of trace IDs.
///
/// # Safety
/// - `store` must be a valid pointer.
/// - `session_id` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_find_by_session(
    store: *const RlmReasoningTraceStore,
    session_id: *const c_char,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }

    let session_id = ffi_try!(cstr_to_str(session_id));

    match (*store).0.find_by_session(session_id) {
        Ok(trace_ids) => {
            let ids: Vec<String> = trace_ids.iter().map(|t| t.to_string()).collect();
            let json = ffi_try!(serde_json::to_string(&ids));
            str_to_cstring(&json)
        }
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Query traces by git commit.
///
/// Returns a JSON array of trace IDs.
///
/// # Safety
/// - `store` must be a valid pointer.
/// - `commit` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_find_by_commit(
    store: *const RlmReasoningTraceStore,
    commit: *const c_char,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }

    let commit = ffi_try!(cstr_to_str(commit));

    match (*store).0.find_by_commit(commit) {
        Ok(trace_ids) => {
            let ids: Vec<String> = trace_ids.iter().map(|t| t.to_string()).collect();
            let json = ffi_try!(serde_json::to_string(&ids));
            str_to_cstring(&json)
        }
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Get store statistics.
///
/// Returns JSON with total_traces, total_nodes, total_edges.
///
/// # Safety
/// - `store` must be a valid pointer.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_reasoning_trace_store_stats(
    store: *const RlmReasoningTraceStore,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }

    match (*store).0.stats() {
        Ok(stats) => {
            let result = serde_json::json!({
                "total_traces": stats.total_traces,
                "total_decision_nodes": stats.total_decision_nodes,
                "total_memory_nodes": stats.total_memory_nodes,
                "total_edges": stats.total_edges,
            });
            let json = ffi_try!(serde_json::to_string(&result));
            str_to_cstring(&json)
        }
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}
