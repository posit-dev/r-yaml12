// We need to forward routine registration from C to Rust
// to avoid the linker removing the static library.

#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <R_ext/Utils.h>
#include <setjmp.h>
#include <stdint.h>
#include <string.h>

SEXP unwind_protect_wrapper(SEXP (*fun)(void *data), void *data);
void not_so_long_jump(void *jmpbuf, Rboolean jump);

SEXP yaml12_dbg_yaml_ffi(SEXP text);
SEXP yaml12_format_yaml_ffi(SEXP value, SEXP multi, SEXP width);
SEXP yaml12_parse_yaml_ffi(SEXP text, SEXP multi, SEXP simplify, SEXP handlers);
SEXP yaml12_read_yaml_ffi(
    const char *path,
    size_t path_len,
    SEXP multi,
    SEXP simplify,
    SEXP handlers
);
SEXP yaml12_write_yaml_ffi(
    SEXP value,
    const char *path,
    size_t path_len,
    SEXP multi,
    SEXP width
);

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;
static SEXP path_expand_sym;

static inline SEXP path_string(SEXP path) {
    if (TYPEOF(path) != STRSXP || XLENGTH(path) != 1) {
        Rf_error("`path` must be a single, non-missing string");
    }

    SEXP string = STRING_ELT(path, 0);
    if (string == NA_STRING) {
        Rf_error("`path` must be a single, non-missing string");
    }
    return string;
}

static inline Rboolean maybe_path_expand(SEXP *path, SEXP *string) {
    if (CHAR(*string)[0] != '~') {
        return FALSE;
    }
    if (memchr(CHAR(*string), '\0', (size_t)XLENGTH(*string)) != NULL) {
        Rf_error("`path` must not contain embedded NUL bytes");
    }

    if (Rf_getCharCE(*string) == CE_NATIVE) {
        const char *expanded = R_ExpandFileName(CHAR(*string));
        if (expanded[0] == '~') {
            Rf_error("`path` could not be expanded");
        }
        *path = Rf_mkString(expanded);
    } else {
        SEXP call = PROTECT(Rf_lang2(path_expand_sym, *path));
        *path = Rf_eval(call, R_BaseEnv);
        *string = STRING_ELT(*path, 0);
        if (CHAR(*string)[0] == '~') {
            Rf_error("`path` could not be expanded");
        }
        UNPROTECT(1);
        return TRUE;
    }

    *string = STRING_ELT(*path, 0);
    return TRUE;
}

static SEXP handle_result(SEXP res_, const char *call_name) {
    uintptr_t res = (uintptr_t)res_;
    if ((res & TAGGED_POINTER_MASK) == 1) {
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);
        if (TYPEOF(res_aligned) == CHARSXP) {
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

SEXP wrap__format_yaml(SEXP value, SEXP multi, SEXP width) {
    return handle_result(yaml12_format_yaml_ffi(value, multi, width), "format_yaml");
}

SEXP wrap__parse_yaml(SEXP text, SEXP multi, SEXP simplify, SEXP handlers) {
    return handle_result(yaml12_parse_yaml_ffi(text, multi, simplify, handlers), "parse_yaml");
}

SEXP wrap__read_yaml(SEXP path, SEXP multi, SEXP simplify, SEXP handlers) {
    SEXP string = path_string(path);
    if (maybe_path_expand(&path, &string)) {
        path = PROTECT(path);
        SEXP result = handle_result(
            yaml12_read_yaml_ffi(
                CHAR(string),
                (size_t)XLENGTH(string),
                multi,
                simplify,
                handlers
            ),
            "read_yaml"
        );
        UNPROTECT(1);
        return result;
    }

    return handle_result(
        yaml12_read_yaml_ffi(
            CHAR(string),
            (size_t)XLENGTH(string),
            multi,
            simplify,
            handlers
        ),
        "read_yaml"
    );
}

SEXP wrap__write_yaml(SEXP value, SEXP path, SEXP multi, SEXP width) {
    if (path == R_NilValue) {
        return handle_result(
            yaml12_write_yaml_ffi(value, NULL, 0, multi, width),
            "write_yaml"
        );
    }

    SEXP string = path_string(path);
    if (maybe_path_expand(&path, &string)) {
        path = PROTECT(path);
        SEXP result = handle_result(
            yaml12_write_yaml_ffi(
                value,
                CHAR(string),
                (size_t)XLENGTH(string),
                multi,
                width
            ),
            "write_yaml"
        );
        UNPROTECT(1);
        return result;
    }

    return handle_result(
        yaml12_write_yaml_ffi(
            value,
            CHAR(string),
            (size_t)XLENGTH(string),
            multi,
            width
        ),
        "write_yaml"
    );
}

static const R_CallMethodDef CallEntries[] = {
    {"wrap__parse_yaml", (DL_FUNC)&wrap__parse_yaml, 4},
    {"wrap__dbg_yaml", (DL_FUNC)&wrap__dbg_yaml, 1},
    {"wrap__format_yaml", (DL_FUNC)&wrap__format_yaml, 3},
    {"wrap__read_yaml", (DL_FUNC)&wrap__read_yaml, 4},
    {"wrap__write_yaml", (DL_FUNC)&wrap__write_yaml, 4},
    {NULL, NULL, 0}
};

void R_init_yaml12(void *dll) {
    path_expand_sym = Rf_install("path.expand");
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
    SEXP res = R_UnwindProtect(fun, data, (void (*)(void *, Rboolean)) not_so_long_jump, &jmpbuf, token);
    UNPROTECT(1);
    return res;
}

void not_so_long_jump(void *jmpbuf, Rboolean jump) {
    if (jump == TRUE) {
        longjmp(*(jmp_buf *)jmpbuf, 1);
    }
}
