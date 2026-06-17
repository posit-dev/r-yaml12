use extendr_api::prelude::*;
use extendr_ffi as ffi;
use std::os::raw::c_void;
use std::result::Result as StdResult;

#[allow(improper_ctypes)]
extern "C" {
    fn unwind_protect_wrapper(
        fun: Option<unsafe extern "C" fn(data: *mut c_void) -> ffi::SEXP>,
        data: *mut c_void,
    ) -> ffi::SEXP;
}

#[derive(Debug, Clone)]
pub struct LongjmpToken {
    tagged_ptr: ffi::SEXP,
}

impl LongjmpToken {
    pub fn into_tagged_sexp(self) -> ffi::SEXP {
        self.tagged_ptr
    }

    pub fn from_tagged_ptr(tagged_ptr: ffi::SEXP) -> Self {
        Self { tagged_ptr }
    }
}

/// Run `f` inside `R_UnwindProtect`.
///
/// A non-local jump from R is returned as an opaque tagged continuation token.
/// Rust only transports that token; the C entrypoint releases and resumes it
/// after Rust-owned values have been dropped.
pub fn run_with_unwind_protect<F>(f: F) -> StdResult<(), LongjmpToken>
where
    F: FnOnce() + Copy,
{
    unsafe extern "C" fn trampoline<F>(data: *mut c_void) -> ffi::SEXP
    where
        F: FnOnce() + Copy,
    {
        let data = data as *const ();
        let f: &F = &*(data as *const F);
        f();
        ffi::R_NilValue
    }

    let f_ptr = &f as *const F as *mut c_void;
    let res = unsafe { unwind_protect_wrapper(Some(trampoline::<F>), f_ptr) };
    if (res as usize & 1) == 1 {
        Err(LongjmpToken::from_tagged_ptr(res))
    } else {
        Ok(())
    }
}

#[derive(Debug)]
pub enum EvalError {
    Api(Error),
    Jump(LongjmpToken),
}

impl From<Error> for EvalError {
    fn from(err: Error) -> Self {
        EvalError::Api(err)
    }
}
