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
    fn R_ContinueUnwind(cont: ffi::SEXP) -> !;
}

#[derive(Debug, Clone)]
pub struct LongjmpToken {
    tagged_ptr: ffi::SEXP,
}

impl LongjmpToken {
    fn cont_handle(&self) -> ffi::SEXP {
        let raw = self.tagged_ptr as usize & !1usize;
        raw as ffi::SEXP
    }

    pub unsafe fn resume(self) -> ! {
        let cont = self.cont_handle();
        // Seems to me we should release the token,
        // but neither Rcpp, nor cpp11, nor savvy release ... am I missing something?
        // ffi::R_ReleaseObject(cont);
        R_ContinueUnwind(cont);
    }

    pub fn from_tagged_ptr(tagged_ptr: ffi::SEXP) -> Self {
        Self { tagged_ptr }
    }
}

/// Run f inside R_UnwindProtect; returns Err when R longjmps.
///
/// Call this only after the entrypoint's Rust scope is clear of owned locals,
/// because `EvalError::Jump` resumes R's continuation and skips the rest of the
/// current frame. Wrap per-call work in a block that produces the `Fallible`
/// result so drops occur before delegating here.
///
/// Good:
/// ```
/// fn entrypoint() -> Robj {
///     let result: Fallible<_> = {
///         let _buf = String::from("tmp"); // drops before handle_eval_error
///         do_work()
///     };
///     match result {
///         Ok(val) => val,
///         Err(err) => handle_eval_error(err),
///     }
/// }
/// ```
///
/// Bad (skips `_buf` drop if a jump occurs):
/// ```
/// fn entrypoint_bad() -> Robj {
///     let _buf = String::from("tmp");
///     let result = do_work();
///     match result {
///         Ok(val) => val,
///         Err(err) => handle_eval_error(err), // jumps before _buf can drop
///     }
/// }
/// ```
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
