//! FFI error handling.
//!
//! Provides thread-local error storage and retrieval for the C API.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Set the last error message for the current thread.
pub(crate) fn set_last_error(msg: &str) {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = CString::new(msg).ok();
    });
}

/// Set the last error from a Result::Err.
pub(crate) fn set_error_from_result<E: std::fmt::Display>(err: E) {
    set_last_error(&err.to_string());
}

/// Clear the last error.
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|e| {
        *e.borrow_mut() = None;
    });
}

/// Get the last error message for the current thread.
///
/// # Safety
/// The returned string is valid until the next call to any rlm_* function
/// on the same thread. Do not free the returned pointer.
#[no_mangle]
pub extern "C" fn rlm_last_error() -> *const c_char {
    LAST_ERROR.with(|e| {
        match e.borrow().as_ref() {
            Some(s) => s.as_ptr(),
            None => std::ptr::null(),
        }
    })
}

/// Check if there is a pending error.
///
/// Returns 1 if there is an error, 0 otherwise.
#[no_mangle]
pub extern "C" fn rlm_has_error() -> i32 {
    LAST_ERROR.with(|e| if e.borrow().is_some() { 1 } else { 0 })
}

/// Clear the last error for the current thread.
#[no_mangle]
pub extern "C" fn rlm_clear_error() {
    clear_last_error();
}

/// Helper macro to convert Result to FFI return value.
macro_rules! ffi_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => {
                $crate::ffi::error::clear_last_error();
                val
            }
            Err(e) => {
                $crate::ffi::error::set_error_from_result(e);
                return std::ptr::null_mut();
            }
        }
    };
    ($expr:expr, $err_val:expr) => {
        match $expr {
            Ok(val) => {
                $crate::ffi::error::clear_last_error();
                val
            }
            Err(e) => {
                $crate::ffi::error::set_error_from_result(e);
                return $err_val;
            }
        }
    };
}

pub(crate) use ffi_try;

/// Helper to convert a C string to Rust &str.
///
/// # Safety
/// The pointer must be valid and null-terminated.
pub(crate) unsafe fn cstr_to_str<'a>(s: *const c_char) -> Result<&'a str, &'static str> {
    if s.is_null() {
        return Err("null pointer");
    }
    CStr::from_ptr(s)
        .to_str()
        .map_err(|_| "invalid UTF-8")
}

/// Helper to convert Rust string to C string (caller must free).
pub(crate) fn str_to_cstring(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => {
            set_last_error("string contains null byte");
            std::ptr::null_mut()
        }
    }
}
