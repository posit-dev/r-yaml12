// We need to forward routine registration from C to Rust
// to avoid the linker removing the static library.

#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <Rversion.h>
#include <setjmp.h>
#include <stdint.h>

SEXP unwind_protect_wrapper(SEXP (*fun)(void *data), void *data);
static void yaml12_not_so_long_jump(void *jmpbuf, Rboolean jump);
SEXP yaml12_closure_env(SEXP fun);

SEXP yaml12_dbg_yaml_ffi(SEXP text);
SEXP yaml12_format_yaml_ffi(SEXP value, SEXP multi);
SEXP yaml12_parse_yaml_ffi(SEXP text, SEXP multi, SEXP simplify, SEXP handlers);
SEXP yaml12_read_yaml_ffi(SEXP path, SEXP multi, SEXP simplify, SEXP handlers);
SEXP yaml12_write_yaml_ffi(SEXP value, SEXP path, SEXP multi);

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

static SEXP handle_result(SEXP res_, const char *call_name) {
    uintptr_t res = (uintptr_t)res_;
    if ((res & TAGGED_POINTER_MASK) == 1) {
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);
        if (TYPEOF(res_aligned) == CHARSXP) {
            PROTECT(res_aligned);
            SEXP call = PROTECT(Rf_lang1(Rf_install(call_name)));
            Rf_errorcall(call, "%s", CHAR(res_aligned));
        } else {
            R_ReleaseObject(res_aligned);
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP wrap__dbg_yaml(SEXP text) {
    return handle_result(yaml12_dbg_yaml_ffi(text), "dbg_yaml");
}

SEXP wrap__format_yaml(SEXP value, SEXP multi) {
    return handle_result(yaml12_format_yaml_ffi(value, multi), "format_yaml");
}

SEXP wrap__parse_yaml(SEXP text, SEXP multi, SEXP simplify, SEXP handlers) {
    return handle_result(yaml12_parse_yaml_ffi(text, multi, simplify, handlers), "parse_yaml");
}

SEXP wrap__read_yaml(SEXP path, SEXP multi, SEXP simplify, SEXP handlers) {
    return handle_result(yaml12_read_yaml_ffi(path, multi, simplify, handlers), "read_yaml");
}

SEXP wrap__write_yaml(SEXP value, SEXP path, SEXP multi) {
    return handle_result(yaml12_write_yaml_ffi(value, path, multi), "write_yaml");
}

static const R_CallMethodDef CallEntries[] = {
    {"wrap__parse_yaml", (DL_FUNC)&wrap__parse_yaml, 4},
    {"wrap__dbg_yaml", (DL_FUNC)&wrap__dbg_yaml, 1},
    {"wrap__format_yaml", (DL_FUNC)&wrap__format_yaml, 2},
    {"wrap__read_yaml", (DL_FUNC)&wrap__read_yaml, 4},
    {"wrap__write_yaml", (DL_FUNC)&wrap__write_yaml, 3},
    {NULL, NULL, 0}
};

void R_init_yaml12(void *dll) {
    R_registerRoutines((DllInfo *)dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols((DllInfo *)dll, FALSE);
    R_forceSymbols((DllInfo *)dll, TRUE);
}

SEXP unwind_protect_wrapper(SEXP (*fun)(void *data), void *data) {
    SEXP token = R_MakeUnwindCont();
    PROTECT(token);
    jmp_buf jmpbuf;
    if (setjmp(jmpbuf)) {
        // keep token alive; tag pointer with low bit so Rust can detect jump
        R_PreserveObject(token);
        UNPROTECT(1);
        return (SEXP)((uintptr_t)token | 1);
    }
    SEXP res = R_UnwindProtect(fun, data, (void (*)(void *, Rboolean)) yaml12_not_so_long_jump, &jmpbuf, token);
    UNPROTECT(1);
    return res;
}

static void yaml12_not_so_long_jump(void *jmpbuf, Rboolean jump) {
    if (jump == TRUE) {
        longjmp(*(jmp_buf *)jmpbuf, 1);
    }
}

SEXP yaml12_closure_env(SEXP fun) {
#if R_VERSION < R_Version(4, 5, 0)
    return CLOENV(fun);
#else
    return R_ClosureEnv(fun);
#endif
}
