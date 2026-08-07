
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
#include <R_ext/Rdynload.h>

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;
static SEXP path_expand_sym;

static inline Rboolean has_tilde_prefix(SEXP path) {
    if (TYPEOF(path) != STRSXP || XLENGTH(path) != 1) {
        return FALSE;
    }

    SEXP string = STRING_ELT(path, 0);
    return string != NA_STRING && CHAR(string)[0] == '~';
}

static inline SEXP expand_tilde_path(SEXP path) {
    SEXP call = PROTECT(Rf_lang2(path_expand_sym, path));
    SEXP expanded = Rf_eval(call, R_BaseEnv);
    UNPROTECT(1);
    return expanded;
}

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

SEXP savvy_format_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__multi, SEXP c_arg__width) {
    SEXP res = savvy_format_yaml_native__ffi(c_arg__value, c_arg__multi, c_arg__width);
    return handle_result(res);
}

SEXP savvy_init_yaml12__impl(DllInfo* c_arg___dll_info) {
    SEXP res = savvy_init_yaml12__ffi(c_arg___dll_info);
    return handle_result(res);
}

SEXP savvy_parse_yaml_native__impl(SEXP c_arg__text, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {
    SEXP res = savvy_parse_yaml_native__ffi(c_arg__text, c_arg__multi, c_arg__simplify, c_arg__handlers);
    return handle_result(res);
}

SEXP savvy_read_yaml_native__impl(SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {
    if (!has_tilde_prefix(c_arg__path)) {
        SEXP res = savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers);
        return handle_result(res);
    }

    c_arg__path = PROTECT(expand_tilde_path(c_arg__path));
    SEXP result = handle_result(savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers));
    UNPROTECT(1);
    return result;
}

SEXP savvy_write_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__width, SEXP c_arg__append) {
    if (!has_tilde_prefix(c_arg__path)) {
        SEXP res = savvy_write_yaml_native__ffi(c_arg__value, c_arg__path, c_arg__multi, c_arg__width, c_arg__append);
        return handle_result(res);
    }

    c_arg__path = PROTECT(expand_tilde_path(c_arg__path));
    SEXP result = handle_result(savvy_write_yaml_native__ffi(c_arg__value, c_arg__path, c_arg__multi, c_arg__width, c_arg__append));
    UNPROTECT(1);
    return result;
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_dbg_yaml_native__impl", (DL_FUNC) &savvy_dbg_yaml_native__impl, 1},
    {"savvy_format_yaml_native__impl", (DL_FUNC) &savvy_format_yaml_native__impl, 3},
    {"savvy_parse_yaml_native__impl", (DL_FUNC) &savvy_parse_yaml_native__impl, 4},
    {"savvy_read_yaml_native__impl", (DL_FUNC) &savvy_read_yaml_native__impl, 4},
    {"savvy_write_yaml_native__impl", (DL_FUNC) &savvy_write_yaml_native__impl, 5},
    {NULL, NULL, 0}
};

void R_init_yaml12(DllInfo *dll) {
    path_expand_sym = Rf_install("path.expand");
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);
    R_forceSymbols(dll, TRUE);

    // Functions for initialzation, if any.
    savvy_init_yaml12__impl(dll);
}
