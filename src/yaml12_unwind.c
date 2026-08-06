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
SEXP yaml12_scalar_logical(int value);
SEXP yaml12_scalar_integer(int value);
SEXP yaml12_scalar_real(double value);
SEXP yaml12_scalar_string(const char *value, int value_len, int is_na);
SEXP yaml12_set_string_elt(SEXP strings, R_xlen_t index, const char *value,
                           int value_len, int is_na);
SEXP yaml12_set_name(SEXP list, R_xlen_t index, const char *value,
                     int value_len, int is_na);
SEXP yaml12_call1(SEXP function, SEXP argument);

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

struct yaml12_logical_data {
    int value;
};

static SEXP yaml12_scalar_logical_impl(void *data) {
    struct yaml12_logical_data *scalar = data;
    return Rf_ScalarLogical(scalar->value);
}

SEXP yaml12_scalar_logical(int value) {
    struct yaml12_logical_data data = {value};
    return yaml12_unwind_protect(yaml12_scalar_logical_impl, &data);
}

struct yaml12_integer_data {
    int value;
};

static SEXP yaml12_scalar_integer_impl(void *data) {
    struct yaml12_integer_data *scalar = data;
    return Rf_ScalarInteger(scalar->value);
}

SEXP yaml12_scalar_integer(int value) {
    struct yaml12_integer_data data = {value};
    return yaml12_unwind_protect(yaml12_scalar_integer_impl, &data);
}

struct yaml12_real_data {
    double value;
};

static SEXP yaml12_scalar_real_impl(void *data) {
    struct yaml12_real_data *scalar = data;
    return Rf_ScalarReal(scalar->value);
}

SEXP yaml12_scalar_real(double value) {
    struct yaml12_real_data data = {value};
    return yaml12_unwind_protect(yaml12_scalar_real_impl, &data);
}

struct yaml12_string_data {
    const char *value;
    int value_len;
    int is_na;
};

static SEXP yaml12_make_char(const struct yaml12_string_data *string) {
    if (string->is_na) {
        return NA_STRING;
    }
    return Rf_mkCharLenCE(string->value, string->value_len, CE_UTF8);
}

static SEXP yaml12_scalar_string_impl(void *data) {
    struct yaml12_string_data *string = data;
    SEXP charsxp = PROTECT(yaml12_make_char(string));
    SEXP result = Rf_ScalarString(charsxp);
    UNPROTECT(1);
    return result;
}

SEXP yaml12_scalar_string(const char *value, int value_len, int is_na) {
    struct yaml12_string_data data = {value, value_len, is_na};
    return yaml12_unwind_protect(yaml12_scalar_string_impl, &data);
}

struct yaml12_set_string_data {
    SEXP target;
    R_xlen_t index;
    struct yaml12_string_data string;
    int set_name;
};

static SEXP yaml12_set_string_impl(void *data) {
    struct yaml12_set_string_data *element = data;
    SEXP target = element->target;
    if (element->set_name) {
        target = Rf_getAttrib(target, R_NamesSymbol);
    }

    SEXP charsxp = PROTECT(yaml12_make_char(&element->string));
    SET_STRING_ELT(target, element->index, charsxp);
    UNPROTECT(1);
    return R_NilValue;
}

static SEXP yaml12_set_string(SEXP target, R_xlen_t index, const char *value,
                              int value_len, int is_na, int set_name) {
    struct yaml12_set_string_data data = {
        target, index, {value, value_len, is_na}, set_name};
    return yaml12_unwind_protect(yaml12_set_string_impl, &data);
}

SEXP yaml12_set_string_elt(SEXP strings, R_xlen_t index, const char *value,
                           int value_len, int is_na) {
    return yaml12_set_string(strings, index, value, value_len, is_na, 0);
}

SEXP yaml12_set_name(SEXP list, R_xlen_t index, const char *value,
                     int value_len, int is_na) {
    return yaml12_set_string(list, index, value, value_len, is_na, 1);
}

struct yaml12_call1_data {
    SEXP function;
    SEXP argument;
};

static SEXP yaml12_call1_impl(void *data) {
    struct yaml12_call1_data *call_data = data;
    SEXP call = PROTECT(Rf_lang2(call_data->function, call_data->argument));
    SEXP result = Rf_eval(call, R_GlobalEnv);
    UNPROTECT(1);
    return result;
}

SEXP yaml12_call1(SEXP function, SEXP argument) {
    /* Protect a fresh argument before the unwind runner allocates its token. */
    PROTECT(argument);
    struct yaml12_call1_data data = {function, argument};
    SEXP result = yaml12_unwind_protect(yaml12_call1_impl, &data);

    if (((uintptr_t)result & 1) == 1) {
        /* R_ContinueUnwind() will restore the protection stack. */
        return result;
    }
    UNPROTECT(1);
    return result;
}
