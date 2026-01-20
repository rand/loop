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
mod epistemic;
mod error;
mod memory;
mod orchestrator;
mod reasoning;
mod repl;
mod trajectory;
mod types;

pub use context::*;
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
