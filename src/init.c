
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
#include <R_ext/Rdynload.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, throw a regular R error so R reports the public
            // wrapper call from the active evaluation context.
            Rf_error("%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP savvy_dbg_yaml_native__impl(SEXP c_arg__text) {
    SEXP res = savvy_dbg_yaml_native__ffi(c_arg__text);
    return handle_result(res);
}

SEXP savvy_format_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__multi) {
    SEXP res = savvy_format_yaml_native__ffi(c_arg__value, c_arg__multi);
    return handle_result(res);
}

SEXP savvy_parse_yaml_native__impl(SEXP c_arg__text, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {
    SEXP res = savvy_parse_yaml_native__ffi(c_arg__text, c_arg__multi, c_arg__simplify, c_arg__handlers);
    return handle_result(res);
}

SEXP savvy_read_yaml_native__impl(SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {
    SEXP res = savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers);
    return handle_result(res);
}

SEXP savvy_write_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__multi, SEXP c_arg__path) {
    SEXP res = savvy_write_yaml_native__ffi(c_arg__value, c_arg__multi, c_arg__path);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_dbg_yaml_native__impl", (DL_FUNC) &savvy_dbg_yaml_native__impl, 1},
    {"savvy_format_yaml_native__impl", (DL_FUNC) &savvy_format_yaml_native__impl, 2},
    {"savvy_parse_yaml_native__impl", (DL_FUNC) &savvy_parse_yaml_native__impl, 4},
    {"savvy_read_yaml_native__impl", (DL_FUNC) &savvy_read_yaml_native__impl, 4},
    {"savvy_write_yaml_native__impl", (DL_FUNC) &savvy_write_yaml_native__impl, 3},
    {NULL, NULL, 0}
};

void R_init_yaml12(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
    R_forceSymbols(dll, TRUE);

    // Functions for initialization, if any.

}
