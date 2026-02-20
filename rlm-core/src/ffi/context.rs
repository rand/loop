//! FFI bindings for context types.

use std::os::raw::c_char;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use super::types::{RlmMessage, RlmRole, RlmSessionContext, RlmToolOutput};
use crate::context::{Message, Role, SessionContext, ToolOutput};

// ============================================================================
// SessionContext
// ============================================================================

/// Create a new empty session context.
///
/// # Safety
/// The returned pointer must be freed with `rlm_session_context_free()`.
#[no_mangle]
pub extern "C" fn rlm_session_context_new() -> *mut RlmSessionContext {
    Box::into_raw(Box::new(RlmSessionContext(SessionContext::new())))
}

/// Free a session context.
///
/// # Safety
/// - `ctx` must be a valid pointer returned by `rlm_session_context_new()`, or NULL.
/// - After calling this function, `ctx` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_free(ctx: *mut RlmSessionContext) {
    if !ctx.is_null() {
        drop(Box::from_raw(ctx));
    }
}

/// Add a message to the session context.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `msg` must be a valid pointer to a message.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_add_message(
    ctx: *mut RlmSessionContext,
    msg: *const RlmMessage,
) -> i32 {
    if ctx.is_null() || msg.is_null() {
        set_last_error("null pointer");
        return -1;
    }
    (*ctx).0.add_message((*msg).0.clone());
    0
}

/// Add a user message to the session context.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `content` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_add_user_message(
    ctx: *mut RlmSessionContext,
    content: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("null context pointer");
        return -1;
    }
    let content = ffi_try!(cstr_to_str(content), -1);
    (*ctx).0.add_user_message(content);
    0
}

/// Add an assistant message to the session context.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `content` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_add_assistant_message(
    ctx: *mut RlmSessionContext,
    content: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("null context pointer");
        return -1;
    }
    let content = ffi_try!(cstr_to_str(content), -1);
    (*ctx).0.add_assistant_message(content);
    0
}

/// Cache a file's contents in the session context.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `path` and `content` must be valid null-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_cache_file(
    ctx: *mut RlmSessionContext,
    path: *const c_char,
    content: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("null context pointer");
        return -1;
    }
    let path = ffi_try!(cstr_to_str(path), -1);
    let content = ffi_try!(cstr_to_str(content), -1);
    (*ctx).0.cache_file(path, content);
    0
}

/// Get a cached file's contents.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `path` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_get_file(
    ctx: *const RlmSessionContext,
    path: *const c_char,
) -> *mut c_char {
    if ctx.is_null() {
        set_last_error("null context pointer");
        return std::ptr::null_mut();
    }
    let path = ffi_try!(cstr_to_str(path));
    match (*ctx).0.get_file(path) {
        Some(content) => str_to_cstring(content),
        None => std::ptr::null_mut(),
    }
}

/// Add a tool output to the session context.
///
/// # Safety
/// - `ctx` must be a valid pointer to a session context.
/// - `output` must be a valid pointer to a tool output.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_add_tool_output(
    ctx: *mut RlmSessionContext,
    output: *const RlmToolOutput,
) -> i32 {
    if ctx.is_null() || output.is_null() {
        set_last_error("null pointer");
        return -1;
    }
    (*ctx).0.add_tool_output((*output).0.clone());
    0
}

/// Get the number of messages in the context.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_message_count(ctx: *const RlmSessionContext) -> i64 {
    if ctx.is_null() {
        return 0;
    }
    (*ctx).0.messages.len() as i64
}

/// Get the number of cached files in the context.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_file_count(ctx: *const RlmSessionContext) -> i64 {
    if ctx.is_null() {
        return 0;
    }
    (*ctx).0.files.len() as i64
}

/// Get the number of tool outputs in the context.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_tool_output_count(
    ctx: *const RlmSessionContext,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }
    (*ctx).0.tool_outputs.len() as i64
}

/// Check if files span multiple directories.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_spans_multiple_directories(
    ctx: *const RlmSessionContext,
) -> i32 {
    if ctx.is_null() {
        return 0;
    }
    if (*ctx).0.spans_multiple_directories() {
        1
    } else {
        0
    }
}

/// Get total approximate tokens in messages.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_total_message_tokens(
    ctx: *const RlmSessionContext,
) -> i64 {
    if ctx.is_null() {
        return 0;
    }
    (*ctx).0.total_message_tokens() as i64
}

/// Serialize context to JSON.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_to_json(ctx: *const RlmSessionContext) -> *mut c_char {
    if ctx.is_null() {
        set_last_error("null context pointer");
        return std::ptr::null_mut();
    }
    match serde_json::to_string(&(*ctx).0) {
        Ok(json) => str_to_cstring(&json),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Deserialize context from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_session_context_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_session_context_from_json(
    json: *const c_char,
) -> *mut RlmSessionContext {
    let json = ffi_try!(cstr_to_str(json));
    let ctx: SessionContext = ffi_try!(serde_json::from_str(json));
    Box::into_raw(Box::new(RlmSessionContext(ctx)))
}

// ============================================================================
// Message
// ============================================================================

/// Create a new message.
///
/// # Safety
/// - `content` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_message_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_new(role: RlmRole, content: *const c_char) -> *mut RlmMessage {
    let content = ffi_try!(cstr_to_str(content));
    let msg = Message::new(Role::from(role), content);
    Box::into_raw(Box::new(RlmMessage(msg)))
}

/// Create a user message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_user(content: *const c_char) -> *mut RlmMessage {
    let content = ffi_try!(cstr_to_str(content));
    Box::into_raw(Box::new(RlmMessage(Message::user(content))))
}

/// Create an assistant message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_assistant(content: *const c_char) -> *mut RlmMessage {
    let content = ffi_try!(cstr_to_str(content));
    Box::into_raw(Box::new(RlmMessage(Message::assistant(content))))
}

/// Create a system message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_system(content: *const c_char) -> *mut RlmMessage {
    let content = ffi_try!(cstr_to_str(content));
    Box::into_raw(Box::new(RlmMessage(Message::system(content))))
}

/// Create a tool message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_tool(content: *const c_char) -> *mut RlmMessage {
    let content = ffi_try!(cstr_to_str(content));
    Box::into_raw(Box::new(RlmMessage(Message::tool(content))))
}

/// Free a message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_free(msg: *mut RlmMessage) {
    if !msg.is_null() {
        drop(Box::from_raw(msg));
    }
}

/// Get the role of a message.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_role(msg: *const RlmMessage) -> RlmRole {
    if msg.is_null() {
        return RlmRole::User;
    }
    RlmRole::from((*msg).0.role)
}

/// Get the content of a message.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_content(msg: *const RlmMessage) -> *mut c_char {
    if msg.is_null() {
        set_last_error("null message pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*msg).0.content)
}

/// Get the timestamp of a message (RFC3339 format).
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
/// Returns NULL if no timestamp is set.
#[no_mangle]
pub unsafe extern "C" fn rlm_message_timestamp(msg: *const RlmMessage) -> *mut c_char {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    match (*msg).0.timestamp {
        Some(ts) => str_to_cstring(&ts.to_rfc3339()),
        None => std::ptr::null_mut(),
    }
}

// ============================================================================
// ToolOutput
// ============================================================================

/// Create a new tool output.
///
/// # Safety
/// - `tool_name` and `content` must be valid null-terminated strings.
/// - The returned pointer must be freed with `rlm_tool_output_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_new(
    tool_name: *const c_char,
    content: *const c_char,
) -> *mut RlmToolOutput {
    let tool_name = ffi_try!(cstr_to_str(tool_name));
    let content = ffi_try!(cstr_to_str(content));
    Box::into_raw(Box::new(RlmToolOutput(ToolOutput::new(tool_name, content))))
}

/// Create a tool output with an exit code.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_new_with_exit_code(
    tool_name: *const c_char,
    content: *const c_char,
    exit_code: i32,
) -> *mut RlmToolOutput {
    let tool_name = ffi_try!(cstr_to_str(tool_name));
    let content = ffi_try!(cstr_to_str(content));
    let output = ToolOutput::new(tool_name, content).with_exit_code(exit_code);
    Box::into_raw(Box::new(RlmToolOutput(output)))
}

/// Free a tool output.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_free(output: *mut RlmToolOutput) {
    if !output.is_null() {
        drop(Box::from_raw(output));
    }
}

/// Get the tool name.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_tool_name(output: *const RlmToolOutput) -> *mut c_char {
    if output.is_null() {
        set_last_error("null pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*output).0.tool_name)
}

/// Get the content.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_content(output: *const RlmToolOutput) -> *mut c_char {
    if output.is_null() {
        set_last_error("null pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*output).0.content)
}

/// Get the exit code. Returns -1 if not set.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_exit_code(output: *const RlmToolOutput) -> i32 {
    if output.is_null() {
        return -1;
    }
    (*output).0.exit_code.unwrap_or(-1)
}

/// Check if the exit code is set.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_has_exit_code(output: *const RlmToolOutput) -> i32 {
    if output.is_null() {
        return 0;
    }
    if (*output).0.exit_code.is_some() {
        1
    } else {
        0
    }
}

/// Check if the tool output indicates success.
#[no_mangle]
pub unsafe extern "C" fn rlm_tool_output_is_success(output: *const RlmToolOutput) -> i32 {
    if output.is_null() {
        return 0;
    }
    if (*output).0.is_success() {
        1
    } else {
        0
    }
}

// ============================================================================
// PatternClassifier
// ============================================================================

/// Create a new pattern classifier with default settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_pattern_classifier_free()`.
#[no_mangle]
pub extern "C" fn rlm_pattern_classifier_new() -> *mut super::types::RlmPatternClassifier {
    Box::into_raw(Box::new(super::types::RlmPatternClassifier(
        crate::complexity::PatternClassifier::new(),
    )))
}

/// Create a pattern classifier with a custom threshold.
#[no_mangle]
pub extern "C" fn rlm_pattern_classifier_with_threshold(
    threshold: i32,
) -> *mut super::types::RlmPatternClassifier {
    Box::into_raw(Box::new(super::types::RlmPatternClassifier(
        crate::complexity::PatternClassifier::with_threshold(threshold),
    )))
}

/// Free a pattern classifier.
#[no_mangle]
pub unsafe extern "C" fn rlm_pattern_classifier_free(
    classifier: *mut super::types::RlmPatternClassifier,
) {
    if !classifier.is_null() {
        drop(Box::from_raw(classifier));
    }
}

/// Check if RLM should activate for a query.
///
/// # Safety
/// - `classifier` must be a valid pointer.
/// - `query` must be a valid null-terminated string.
/// - `ctx` must be a valid pointer to a session context.
/// - The returned pointer must be freed with `rlm_activation_decision_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_pattern_classifier_should_activate(
    classifier: *const super::types::RlmPatternClassifier,
    query: *const c_char,
    ctx: *const RlmSessionContext,
) -> *mut super::types::RlmActivationDecision {
    if classifier.is_null() || ctx.is_null() {
        set_last_error("null pointer");
        return std::ptr::null_mut();
    }
    let query = ffi_try!(cstr_to_str(query));
    let decision = (*classifier).0.should_activate(query, &(*ctx).0);
    Box::into_raw(Box::new(super::types::RlmActivationDecision(decision)))
}

// ============================================================================
// ActivationDecision
// ============================================================================

/// Free an activation decision.
#[no_mangle]
pub unsafe extern "C" fn rlm_activation_decision_free(
    decision: *mut super::types::RlmActivationDecision,
) {
    if !decision.is_null() {
        drop(Box::from_raw(decision));
    }
}

/// Check if RLM should activate.
#[no_mangle]
pub unsafe extern "C" fn rlm_activation_decision_should_activate(
    decision: *const super::types::RlmActivationDecision,
) -> i32 {
    if decision.is_null() {
        return 0;
    }
    if (*decision).0.should_activate {
        1
    } else {
        0
    }
}

/// Get the decision reason.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_activation_decision_reason(
    decision: *const super::types::RlmActivationDecision,
) -> *mut c_char {
    if decision.is_null() {
        set_last_error("null pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*decision).0.reason)
}

/// Get the complexity score.
#[no_mangle]
pub unsafe extern "C" fn rlm_activation_decision_score(
    decision: *const super::types::RlmActivationDecision,
) -> i32 {
    if decision.is_null() {
        return 0;
    }
    (*decision).0.score
}
