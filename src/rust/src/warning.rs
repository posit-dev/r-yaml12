use crate::{api_other, Fallible};
use savvy_ffi as ffi;
use std::ffi::CString;
use std::os::raw::c_char;

#[allow(improper_ctypes)]
extern "C" {
    fn Rf_warningcall(call: ffi::SEXP, format: *const c_char, ...);
}

pub(crate) fn emit_warning(message: &str) -> Fallible<()> {
    let c_message = CString::new(message)
        .map_err(|_| api_other("Warning message contains interior nul byte"))?;

    unsafe {
        savvy::unwind_protect(|| {
            Rf_warningcall(
                ffi::R_NilValue,
                b"%s\0".as_ptr() as *const c_char,
                c_message.as_ptr(),
            );
            ffi::R_NilValue
        })?;
    }
    Ok(())
}
