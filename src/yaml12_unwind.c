#if !defined(_WIN32) && !defined(__EMSCRIPTEN__) && !defined(_POSIX_C_SOURCE)
#define _POSIX_C_SOURCE 200809L
#endif

#include <setjmp.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include <Rinternals.h>
#include <Rversion.h>

#if defined(_WIN32) || defined(__EMSCRIPTEN__)
typedef jmp_buf yaml12_jmp_buf;

static void yaml12_long_jump(void *buf, Rboolean jump) {
    if (jump == TRUE) {
        longjmp(*(yaml12_jmp_buf *)buf, 1);
    }
}

#define YAML12_SETJMP(buf) setjmp(buf)
#else
typedef sigjmp_buf yaml12_jmp_buf;

static void yaml12_long_jump(void *buf, Rboolean jump) {
    if (jump == TRUE) {
        siglongjmp(*(yaml12_jmp_buf *)buf, 1);
    }
}

/* R errors do not need the process signal mask copied. */
#define YAML12_SETJMP(buf) sigsetjmp(buf, 0)
#endif

enum yaml12_charsxp_encoding {
    YAML12_CHARSXP_MUST_TRANSLATE = 0,
    YAML12_CHARSXP_UTF8 = 1,
    YAML12_CHARSXP_NATIVE = 2
};

int yaml12_charsxp_encoding(SEXP value);
SEXP yaml12_translate_char_utf8(SEXP value, const char **out, size_t *out_len);

static SEXP yaml12_unwind_protect(SEXP (*fun)(void *data), void *data) {
    SEXP token = R_MakeUnwindCont();
    PROTECT(token);

    yaml12_jmp_buf buf;
    if (YAML12_SETJMP(buf)) {
        /* The generated Savvy wrapper recognizes tagged continuations. */
        return (SEXP)((uintptr_t)token | 1);
    }

    SEXP result = R_UnwindProtect(fun, data, yaml12_long_jump, &buf, token);
    UNPROTECT(1);
    return result;
}

int yaml12_charsxp_encoding(SEXP value) {
    cetype_t encoding = Rf_getCharCE(value);

    if (encoding == CE_LATIN1 || encoding == CE_BYTES) {
        return YAML12_CHARSXP_MUST_TRANSLATE;
    }
    if (encoding == CE_UTF8) {
        return YAML12_CHARSXP_UTF8;
    }
#if R_VERSION >= R_Version(4, 5, 0)
    if (encoding == CE_NATIVE && Rf_charIsUTF8(value) == TRUE) {
        return YAML12_CHARSXP_UTF8;
    }
#endif
    return encoding == CE_NATIVE ? YAML12_CHARSXP_NATIVE
                                 : YAML12_CHARSXP_MUST_TRANSLATE;
}

struct yaml12_translate_data {
    SEXP value;
    const char *out;
    size_t out_len;
};

static SEXP yaml12_translate_char_utf8_impl(void *data) {
    struct yaml12_translate_data *translation = data;
    translation->out = Rf_translateCharUTF8(translation->value);
    translation->out_len = strlen(translation->out);
    return R_NilValue;
}

SEXP yaml12_translate_char_utf8(SEXP value, const char **out, size_t *out_len) {
    struct yaml12_translate_data data = {value, NULL, 0};
    SEXP result = yaml12_unwind_protect(yaml12_translate_char_utf8_impl, &data);

    if (((uintptr_t)result & 1) == 0) {
        *out = data.out;
        *out_len = data.out_len;
    }
    return result;
}
