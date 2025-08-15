pub mod brainfuck;

use brainfuck::{compile, get_offset, unsafe_run};
use std::ffi::CStr;
use std::os::raw::{c_char, c_uchar};

/// Execute Brainfuck code through a C-compatible interface.
///
/// # Safety
/// `code` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bropt_run(code: *const c_char, length: usize, flush: c_uchar) {
    if code.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(code) };
    if let Ok(code_str) = c_str.to_str() {
        let prog = compile(code_str);
        let offset = get_offset(&prog);
        if flush != 0 {
            unsafe_run::<true>(prog, length, offset);
        } else {
            unsafe_run::<false>(prog, length, offset);
        }
    }
}
