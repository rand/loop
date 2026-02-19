//! C FFI bindings for rlm-core.
//!
//! This module provides C-compatible FFI functions for integration with
//! Go (CGO), Swift, and other languages that can call C APIs.
//!
//! ## Memory Management
//!
//! The FFI layer uses explicit memory management:
//! - Objects created by `*_new()` functions must be freed with corresponding `*_free()` functions
//! - Strings returned by the library must be freed with `rlm_string_free()`
//! - Caller-owned strings passed to functions are not freed by the library
//!
//! ## Error Handling
//!
//! Functions that can fail return:
//! - `NULL` for pointer-returning functions (check `rlm_last_error()` for details)
//! - `-1` or a negative value for integer-returning functions
//! - Error details are available via `rlm_last_error()` (thread-local)
//!
//! ## Thread Safety
//!
//! - The library is thread-safe; objects can be used from multiple threads
//! - Each thread has its own last error state

mod context;
mod cost;
mod epistemic;
mod error;
mod memory;
mod orchestrator;
mod reasoning;
mod repl;
mod trajectory;
mod types;

pub use context::*;
pub use cost::*;
pub use epistemic::*;
pub use error::*;
pub use memory::*;
pub use orchestrator::*;
pub use reasoning::*;
pub use repl::*;
pub use trajectory::*;
pub use types::*;

use std::ffi::CString;
use std::os::raw::c_char;

// ============================================================================
// Library Information
// ============================================================================

/// Get the library version string.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub extern "C" fn rlm_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    match CString::new(version) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the library version major number.
#[no_mangle]
pub extern "C" fn rlm_version_major() -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Get the library version minor number.
#[no_mangle]
pub extern "C" fn rlm_version_minor() -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    version
        .split('.')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Get the library version patch number.
#[no_mangle]
pub extern "C" fn rlm_version_patch() -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    version
        .split('.')
        .nth(2)
        .and_then(|s| s.split('-').next()) // Handle pre-release like 0.2.0-alpha
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Check if a feature is available.
///
/// Returns 1 if the feature is available, 0 if not, -1 if the feature name is invalid.
///
/// Available features:
/// - "gemini": Google/Gemini provider support
/// - "adversarial": Adversarial validation support (requires gemini)
/// - "python": Python bindings (PyO3)
///
/// # Safety
/// - `feature_name` must be a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn rlm_has_feature(feature_name: *const c_char) -> i32 {
    if feature_name.is_null() {
        return -1;
    }

    let name = match std::ffi::CStr::from_ptr(feature_name).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match name {
        "gemini" => {
            #[cfg(feature = "gemini")]
            return 1;
            #[cfg(not(feature = "gemini"))]
            return 0;
        }
        "adversarial" => {
            #[cfg(feature = "adversarial")]
            return 1;
            #[cfg(not(feature = "adversarial"))]
            return 0;
        }
        "python" => {
            #[cfg(feature = "python")]
            return 1;
            #[cfg(not(feature = "python"))]
            return 0;
        }
        _ => -1, // Unknown feature
    }
}

/// Get a comma-separated list of available features.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub extern "C" fn rlm_available_features() -> *mut c_char {
    let features: &[&str] = &[
        #[cfg(feature = "gemini")]
        "gemini",
        #[cfg(feature = "adversarial")]
        "adversarial",
        #[cfg(feature = "python")]
        "python",
    ];

    let features_str = features.join(",");
    match CString::new(features_str) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a string allocated by the library.
///
/// # Safety
/// - `s` must be a pointer returned by an rlm_* function, or NULL.
/// - After calling this function, `s` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the library. Must be called before any other functions.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn rlm_init() -> i32 {
    // Initialize logging, etc. if needed
    0
}

/// Shutdown the library. Should be called before program exit.
#[no_mangle]
pub extern "C" fn rlm_shutdown() {
    // Cleanup if needed
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_version() {
        let version = rlm_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version).to_str().unwrap() };
        assert!(!version_str.is_empty());
        assert!(version_str.contains('.'), "version should contain dots");
        unsafe { rlm_string_free(version) };
    }

    #[test]
    fn test_init_shutdown() {
        assert_eq!(rlm_init(), 0);
        rlm_shutdown();
    }

    #[test]
    fn test_error_handling() {
        // Initially no error
        rlm_clear_error();
        assert_eq!(rlm_has_error(), 0);

        // After setting an error
        set_last_error("test error");
        assert_eq!(rlm_has_error(), 1);

        let err = rlm_last_error();
        assert!(!err.is_null());
        let err_str = unsafe { CStr::from_ptr(err).to_str().unwrap() };
        assert_eq!(err_str, "test error");

        // Clear and verify
        rlm_clear_error();
        assert_eq!(rlm_has_error(), 0);
    }

    #[test]
    fn test_string_free_null_safe() {
        // Should not panic on null
        unsafe { rlm_string_free(std::ptr::null_mut()) };
    }

    #[test]
    fn test_session_context_lifecycle() {
        let ctx = rlm_session_context_new();
        assert!(!ctx.is_null());

        let count = unsafe { rlm_session_context_message_count(ctx) };
        assert_eq!(count, 0);

        unsafe { rlm_session_context_free(ctx) };
    }

    #[test]
    fn test_session_context_messages() {
        let ctx = rlm_session_context_new();
        assert!(!ctx.is_null());

        // Add user message
        let content = std::ffi::CString::new("Hello").unwrap();
        let result = unsafe { rlm_session_context_add_user_message(ctx, content.as_ptr()) };
        assert_eq!(result, 0);

        let count = unsafe { rlm_session_context_message_count(ctx) };
        assert_eq!(count, 1);

        // Add assistant message
        let content = std::ffi::CString::new("Hi there").unwrap();
        let result = unsafe { rlm_session_context_add_assistant_message(ctx, content.as_ptr()) };
        assert_eq!(result, 0);

        let count = unsafe { rlm_session_context_message_count(ctx) };
        assert_eq!(count, 2);

        unsafe { rlm_session_context_free(ctx) };
    }

    #[test]
    fn test_message_lifecycle() {
        let content = std::ffi::CString::new("Test message").unwrap();
        let msg = unsafe { rlm_message_user(content.as_ptr()) };
        assert!(!msg.is_null());

        let role = unsafe { rlm_message_role(msg) };
        assert_eq!(role, RlmRole::User);

        let msg_content = unsafe { rlm_message_content(msg) };
        assert!(!msg_content.is_null());
        let content_str = unsafe { CStr::from_ptr(msg_content).to_str().unwrap() };
        assert_eq!(content_str, "Test message");
        unsafe { rlm_string_free(msg_content) };

        unsafe { rlm_message_free(msg) };
    }

    #[test]
    fn test_memory_store_lifecycle() {
        let store = rlm_memory_store_in_memory();
        assert!(!store.is_null());

        let stats = unsafe { rlm_memory_store_stats(store) };
        assert!(!stats.is_null());
        unsafe { rlm_string_free(stats) };

        unsafe { rlm_memory_store_free(store) };
    }

    #[test]
    fn test_node_lifecycle() {
        let content = std::ffi::CString::new("Test fact").unwrap();
        let node = unsafe { rlm_node_new(RlmNodeType::Fact, content.as_ptr()) };
        assert!(!node.is_null());

        let node_type = unsafe { rlm_node_type(node) };
        assert_eq!(node_type, RlmNodeType::Fact);

        let node_content = unsafe { rlm_node_content(node) };
        assert!(!node_content.is_null());
        let content_str = unsafe { CStr::from_ptr(node_content).to_str().unwrap() };
        assert_eq!(content_str, "Test fact");
        unsafe { rlm_string_free(node_content) };

        unsafe { rlm_node_free(node) };
    }

    #[test]
    fn test_trajectory_event_lifecycle() {
        let content = std::ffi::CString::new("test query").unwrap();
        let event = unsafe { rlm_trajectory_event_rlm_start(content.as_ptr()) };
        assert!(!event.is_null());

        let event_type = unsafe { rlm_trajectory_event_type(event) };
        assert_eq!(event_type, RlmTrajectoryEventType::RlmStart);

        let is_final = unsafe { rlm_trajectory_event_is_final(event) };
        assert_eq!(is_final, 0);

        unsafe { rlm_trajectory_event_free(event) };
    }

    #[test]
    fn test_pattern_classifier() {
        let classifier = rlm_pattern_classifier_new();
        assert!(!classifier.is_null());

        let ctx = rlm_session_context_new();
        let query = std::ffi::CString::new("simple question").unwrap();

        let decision = unsafe { rlm_pattern_classifier_should_activate(classifier, query.as_ptr(), ctx) };
        assert!(!decision.is_null());

        let _should_activate = unsafe { rlm_activation_decision_should_activate(decision) };
        let _score = unsafe { rlm_activation_decision_score(decision) };

        unsafe { rlm_activation_decision_free(decision) };
        unsafe { rlm_session_context_free(ctx) };
        unsafe { rlm_pattern_classifier_free(classifier) };
    }

    #[test]
    fn test_cost_tracker_lifecycle() {
        let tracker = rlm_cost_tracker_new();
        assert!(!tracker.is_null());

        // Record some usage
        let model = std::ffi::CString::new("claude-sonnet").unwrap();
        let result = unsafe {
            rlm_cost_tracker_record(tracker, model.as_ptr(), 1000, 500, 0, 0, 0.01)
        };
        assert_eq!(result, 0);

        let input = unsafe { rlm_cost_tracker_total_input_tokens(tracker) };
        assert_eq!(input, 1000);

        let output = unsafe { rlm_cost_tracker_total_output_tokens(tracker) };
        assert_eq!(output, 500);

        unsafe { rlm_cost_tracker_free(tracker) };
    }

    #[test]
    fn test_orchestrator_mode() {
        let budget = rlm_execution_mode_budget_usd(RlmExecutionMode::Micro);
        assert!(budget > 0.0 && budget < 0.1);

        let budget = rlm_execution_mode_budget_usd(RlmExecutionMode::Thorough);
        assert!(budget >= 1.0);

        let name = rlm_execution_mode_name(RlmExecutionMode::Balanced);
        assert!(!name.is_null());
        unsafe { rlm_string_free(name) };
    }

    #[test]
    fn test_orchestrator_config() {
        let config = rlm_orchestrator_config_default();
        assert!(!config.is_null());

        let depth = unsafe { rlm_orchestrator_config_max_depth(config) };
        assert!(depth > 0);

        unsafe { rlm_orchestrator_config_free(config) };
    }

    #[test]
    fn test_kl_divergence_functions() {
        // Binary entropy of fair coin
        let entropy = rlm_binary_entropy_bits(0.5);
        assert!((entropy - 1.0).abs() < 0.001);

        // Surprise of certain event
        let surprise = rlm_surprise_bits(1.0);
        assert!(surprise.abs() < 0.001);

        // KL divergence of identical distributions
        let kl = rlm_kl_bernoulli_bits(0.5, 0.5);
        assert!(kl.abs() < 0.001);
    }

    #[test]
    fn test_claim_extractor() {
        let extractor = rlm_claim_extractor_new();
        assert!(!extractor.is_null());

        let text = std::ffi::CString::new("The sky is blue. Water is wet.").unwrap();
        let claims = unsafe { rlm_claim_extractor_extract(extractor, text.as_ptr()) };
        assert!(!claims.is_null());
        unsafe { rlm_string_free(claims) };

        unsafe { rlm_claim_extractor_free(extractor) };
    }

    #[test]
    fn test_reasoning_trace() {
        let goal = std::ffi::CString::new("test goal").unwrap();
        let session = std::ffi::CString::new("test-session").unwrap();

        let trace = unsafe { rlm_reasoning_trace_new(goal.as_ptr(), session.as_ptr()) };
        assert!(!trace.is_null());

        let trace_id = unsafe { rlm_reasoning_trace_id(trace) };
        assert!(!trace_id.is_null());
        unsafe { rlm_string_free(trace_id) };

        let json = unsafe { rlm_reasoning_trace_to_json(trace) };
        assert!(!json.is_null());
        unsafe { rlm_string_free(json) };

        unsafe { rlm_reasoning_trace_free(trace) };
    }

    #[test]
    fn test_version_components() {
        let major = rlm_version_major();
        let minor = rlm_version_minor();
        let patch = rlm_version_patch();

        // Version should be non-negative
        assert!(major >= 0);
        assert!(minor >= 0);
        assert!(patch >= 0);

        // Full version string should match components
        let version = rlm_version();
        assert!(!version.is_null());
        let version_str = unsafe { CStr::from_ptr(version).to_str().unwrap() };
        assert!(
            version_str.starts_with(&format!("{}.{}.{}", major, minor, patch)),
            "version {} should start with {}.{}.{}",
            version_str,
            major,
            minor,
            patch
        );
        unsafe { rlm_string_free(version) };
    }

    #[test]
    fn test_has_feature() {
        // Known features should return 0 or 1
        let gemini = std::ffi::CString::new("gemini").unwrap();
        let result = unsafe { rlm_has_feature(gemini.as_ptr()) };
        assert!(result == 0 || result == 1);

        let adversarial = std::ffi::CString::new("adversarial").unwrap();
        let result = unsafe { rlm_has_feature(adversarial.as_ptr()) };
        assert!(result == 0 || result == 1);

        let python = std::ffi::CString::new("python").unwrap();
        let result = unsafe { rlm_has_feature(python.as_ptr()) };
        assert!(result == 0 || result == 1);

        // Unknown feature should return -1
        let unknown = std::ffi::CString::new("unknown_feature").unwrap();
        let result = unsafe { rlm_has_feature(unknown.as_ptr()) };
        assert_eq!(result, -1);

        // Null should return -1
        let result = unsafe { rlm_has_feature(std::ptr::null()) };
        assert_eq!(result, -1);
    }

    #[test]
    fn test_available_features() {
        let features = rlm_available_features();
        assert!(!features.is_null());
        let features_str = unsafe { CStr::from_ptr(features).to_str().unwrap() };

        // Features string should be comma-separated or empty
        // Depending on compile flags, it may or may not contain certain features
        assert!(
            features_str.is_empty()
                || features_str.split(',').all(|f| ["gemini", "adversarial", "python"].contains(&f))
        );

        unsafe { rlm_string_free(features) };
    }

    #[test]
    fn test_available_features_matches_has_feature_contract() {
        let features = rlm_available_features();
        assert!(!features.is_null());
        let features_str = unsafe { CStr::from_ptr(features).to_str().unwrap() };
        let available_from_list: Vec<&str> = if features_str.is_empty() {
            Vec::new()
        } else {
            features_str.split(',').collect()
        };

        let known = ["gemini", "adversarial", "python"];
        let mut expected: Vec<&str> = Vec::new();
        for feature in known {
            let name = std::ffi::CString::new(feature).unwrap();
            if unsafe { rlm_has_feature(name.as_ptr()) } == 1 {
                expected.push(feature);
            }
        }

        assert_eq!(
            available_from_list, expected,
            "available features list should match rlm_has_feature() contract"
        );

        unsafe { rlm_string_free(features) };
    }
}
