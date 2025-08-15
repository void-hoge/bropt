pub mod brainfuck;

use brainfuck::{compile, run_result};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uchar};
use std::ptr;

/// Execute Brainfuck code through a C-compatible interface and capture tape state.
///
/// Returns a null pointer on success or an error string on failure. The caller must
/// free the returned error string with `bropt_free_error`.
///
/// # Safety
/// `code` must be a valid null-terminated UTF-8 string and `out` must be a valid
/// buffer of at least `length` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bropt_run(
    code: *const c_char,
    length: usize,
    flush: c_uchar,
    out: *mut c_uchar,
) -> *mut c_char {
    if code.is_null() || out.is_null() {
        return CString::new("null pointer").unwrap().into_raw();
    }
    let c_str = unsafe { CStr::from_ptr(code) };
    let code_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return CString::new("invalid utf-8").unwrap().into_raw(),
    };
    let prog = match compile(code_str) {
        Ok(p) => p,
        Err(e) => return CString::new(e).unwrap().into_raw(),
    };
    let result = if flush != 0 {
        run_result::<true>(prog, length)
    } else {
        run_result::<false>(prog, length)
    };
    match result {
        Ok(data) => {
            unsafe { ptr::copy_nonoverlapping(data.as_ptr(), out, length); }
            ptr::null_mut()
        }
        Err(e) => CString::new(e).unwrap().into_raw(),
    }
}

/// Free an error string returned by `bropt_run`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bropt_free_error(err: *mut c_char) {
    if !err.is_null() {
        unsafe { drop(CString::from_raw(err)); }
    }
}
