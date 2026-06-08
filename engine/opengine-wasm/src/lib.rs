//! opengine-wasm — the browser ABI for the wargame.
//!
//! Deliberately *not* using wasm-bindgen: the project values minimal tooling, so
//! this is a plain `cdylib` with a tiny JSON-over-linear-memory protocol that a
//! hand-written JS loader (`web/wargame.js`) drives. All game logic lives in
//! `opengine_core::wargame::Session`; this file is only memory plumbing.
//!
//! Protocol (see `web/wargame.js`):
//!   1. JS calls `alloc(len)` -> ptr, writes `len` UTF-8 bytes of a command JSON.
//!   2. JS calls `dispatch(ptr, len)` -> response ptr. `dispatch` takes ownership
//!      of the input buffer (frees it), runs the command, and leaks a response
//!      buffer; its length is read via `last_response_len()`.
//!   3. JS reads the response bytes, then calls `dealloc(resp_ptr, resp_len)`.

use std::cell::{Cell, RefCell};

use opengine_core::wargame::Session;

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::new());
    static LAST_LEN: Cell<usize> = const { Cell::new(0) };
}

/// Allocate `len` bytes in wasm linear memory and return the pointer. JS writes
/// the command JSON here before calling `dispatch`.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Free a buffer previously returned by `alloc`/`dispatch` (`len` must match).
///
/// # Safety
/// `ptr` must come from this module's allocator with the given `len`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len != 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

/// Run one command. Consumes the `[ptr, len]` input buffer, returns a pointer to
/// a freshly-allocated UTF-8 JSON response whose length is `last_response_len()`.
///
/// # Safety
/// `ptr`/`len` must describe a buffer from `alloc` holding valid command bytes.
#[no_mangle]
pub unsafe extern "C" fn dispatch(ptr: *mut u8, len: usize) -> *mut u8 {
    let input = Vec::from_raw_parts(ptr, len, len);
    let cmd = String::from_utf8_lossy(&input);
    let resp = SESSION.with(|s| s.borrow_mut().dispatch_json(&cmd));
    // input drops here, freeing the JS-allocated command buffer.

    let bytes = resp.into_bytes().into_boxed_slice();
    LAST_LEN.with(|c| c.set(bytes.len()));
    Box::into_raw(bytes) as *mut u8
}

/// Length of the response buffer from the most recent `dispatch`.
#[no_mangle]
pub extern "C" fn last_response_len() -> usize {
    LAST_LEN.with(|c| c.get())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exercise the ABI end-to-end natively (rlib crate-type makes this testable).
    #[test]
    fn roundtrip_through_abi() {
        let cmd = r#"{"cmd":"list"}"#;
        let bytes = cmd.as_bytes();
        let p = alloc(bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            let rp = dispatch(p, bytes.len());
            let rl = last_response_len();
            let resp = std::slice::from_raw_parts(rp, rl);
            let text = std::str::from_utf8(resp).unwrap();
            assert!(text.contains(r#""ok":true"#));
            dealloc(rp, rl);
        }
    }
}
