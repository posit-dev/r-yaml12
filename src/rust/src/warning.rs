use extendr_api::prelude::*;
use extendr_ffi as ffi;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

#[allow(improper_ctypes)]
extern "C" {
    fn R_tryCatchError(
        body: Option<unsafe extern "C" fn(*mut c_void) -> ffi::SEXP>,
        body_data: *mut c_void,
        handler: Option<unsafe extern "C" fn(ffi::SEXP, *mut c_void) -> ffi::SEXP>,
        handler_data: *mut c_void,
    ) -> ffi::SEXP;
    fn Rf_warningcall(call: ffi::SEXP, format: *const c_char, ...);
}

struct WarningState<'a> {
    message: &'a CString,
    had_error: bool,
    jumped: bool,
}

unsafe extern "C" fn issue_warning_pointer(data: *mut c_void) -> ffi::SEXP {
    let state = &*(data as *const WarningState);
    Rf_warningcall(
        ffi::R_NilValue,
        b"%s\0".as_ptr() as *const c_char,
        state.message.as_ptr(),
    );
    ffi::R_NilValue
}

unsafe extern "C" fn handle_warning_as_error(_cond: ffi::SEXP, data: *mut c_void) -> ffi::SEXP {
    let state = &mut *(data as *mut WarningState);
    state.had_error = true;
    ffi::R_NilValue
}

unsafe extern "C" fn warn_via_trycatch(data: *mut c_void) -> ffi::SEXP {
    R_tryCatchError(
        Some(issue_warning_pointer),
        data,
        Some(handle_warning_as_error),
        data,
    )
}

unsafe extern "C" fn record_jump(data: *mut c_void, jump: ffi::Rboolean) {
    let state = &mut *(data as *mut WarningState);
    state.jumped = jump != ffi::Rboolean::FALSE;
}

pub(crate) fn emit_warning(message: &str) -> Result<()> {
    let c_message = CString::new(message)
        .map_err(|_| Error::Other("Warning message contains interior nul byte".to_string()))?;
    let mut state = WarningState {
        message: &c_message,
        had_error: false,
        jumped: false,
    };

    single_threaded(|| unsafe {
        let cont = ffi::R_MakeUnwindCont();
        ffi::Rf_protect(cont);

        let unwind_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ffi::R_UnwindProtect(
                Some(warn_via_trycatch),
                &mut state as *mut _ as *mut c_void,
                Some(record_jump),
                &mut state as *mut _ as *mut c_void,
                cont,
            )
        }));

        ffi::Rf_unprotect(1);

        match unwind_res {
            Ok(_) if !state.jumped => {
                if state.had_error {
                    Err(Error::Other(format!(
                        "{message} (converted to error by options(warn))"
                    )))
                } else {
                    Ok(())
                }
            }
            _ => Err(Error::Other(
                "R aborted while attempting to issue warning".to_string(),
            )),
        }
    })
}
