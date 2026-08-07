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
    YAML12_CHARSXP_NATIVE = 2,
    YAML12_CHARSXP_ASCII = 3
};

enum yaml12_list_element_kind {
    YAML12_LIST_NULL = 0,
    YAML12_LIST_LOGICAL = 1,
    YAML12_LIST_INTEGER = 2,
    YAML12_LIST_REAL = 3,
    YAML12_LIST_STRING = 4,
    YAML12_LIST_SKIP = 5
};

struct yaml12_string_data {
    const char *value;
    int value_len;
    int is_na;
};

struct yaml12_list_element {
    int kind;
    int int_value;
    double real_value;
    const char *string_value;
    int string_len;
    int string_is_na;
};

int yaml12_charsxp_encoding(SEXP value);
int yaml12_has_attributes(SEXP value);
SEXP yaml12_translate_char_utf8(SEXP value, const char **out, size_t *out_len);
SEXP yaml12_scalar_logical(int value);
SEXP yaml12_scalar_integer(int value);
SEXP yaml12_scalar_real(double value);
SEXP yaml12_scalar_string(const char *value, int value_len, int is_na);
SEXP yaml12_materialize_string_vector(
    const struct yaml12_string_data *values, R_xlen_t length);
SEXP yaml12_materialize_list(SEXP target,
                             const struct yaml12_list_element *elements,
                             const struct yaml12_string_data *names,
                             R_xlen_t length);
SEXP yaml12_call1(SEXP function, SEXP argument);

static SEXP yaml12_unwind_protect(SEXP (*fun)(void *data), void *data) {
    SEXP token = R_MakeUnwindCont();
    PROTECT(token);

    yaml12_jmp_buf buf;
    if (YAML12_SETJMP(buf)) {
        /*
         * Match Savvy's private tagged-result ABI. Bit 0 is not an R flag:
         * Savvy assumes it is clear on genuine SEXP pointers, uses it only
         * while carrying the continuation through Rust, and clears it before
         * calling R_ContinueUnwind().
         */
        return (SEXP)((uintptr_t)token | 1);
    }

    SEXP result = R_UnwindProtect(fun, data, yaml12_long_jump, &buf, token);
    UNPROTECT(1);
    return result;
}

int yaml12_charsxp_encoding(SEXP value) {
#if R_VERSION >= R_Version(4, 5, 0)
    if (Rf_charIsASCII(value) == TRUE) {
        return YAML12_CHARSXP_ASCII;
    }
#endif

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

int yaml12_has_attributes(SEXP value) {
#if R_VERSION >= R_Version(4, 5, 0)
    return ANY_ATTRIB(value);
#else
    return ATTRIB(value) != R_NilValue;
#endif
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

/*
 * Writing R Extensions and R's own construction loops attach fresh values
 * directly with SET_*_ELT. That is safe here because the targets are ordinary
 * rooted vectors with types and indices guaranteed by construction; their
 * successful setter paths perform the write barrier and store without
 * allocating. Rf_ScalarString() protects its CHARSXP argument across its own
 * allocation.
 */

static void yaml12_fill_string_vector(
    SEXP target, const struct yaml12_string_data *values, R_xlen_t length) {
    for (R_xlen_t i = 0; i < length; i++) {
        SET_STRING_ELT(target, i, yaml12_make_char(&values[i]));
    }
}

struct yaml12_materialize_string_vector_data {
    const struct yaml12_string_data *values;
    R_xlen_t length;
};

static SEXP yaml12_materialize_string_vector_impl(void *data) {
    struct yaml12_materialize_string_vector_data *batch = data;
    SEXP strings = PROTECT(Rf_allocVector(STRSXP, batch->length));
    yaml12_fill_string_vector(strings, batch->values, batch->length);
    UNPROTECT(1);
    return strings;
}

SEXP yaml12_materialize_string_vector(
    const struct yaml12_string_data *values, R_xlen_t length) {
    struct yaml12_materialize_string_vector_data data = {values, length};
    return yaml12_unwind_protect(yaml12_materialize_string_vector_impl, &data);
}

struct yaml12_materialize_list_data {
    SEXP target;
    const struct yaml12_list_element *elements;
    const struct yaml12_string_data *names;
    R_xlen_t length;
};

static SEXP yaml12_materialize_list_impl(void *data) {
    struct yaml12_materialize_list_data *batch = data;
    SEXP list = batch->target;
    int protect_count = 0;

    if (list == R_NilValue) {
        list = PROTECT(Rf_allocVector(VECSXP, batch->length));
        protect_count++;
    }

    SEXP names = R_NilValue;
    if (batch->names != NULL) {
        names = PROTECT(Rf_allocVector(STRSXP, batch->length));
        protect_count++;
    }

    for (R_xlen_t i = 0; i < batch->length; i++) {
        const struct yaml12_list_element *element = &batch->elements[i];

        switch (element->kind) {
        case YAML12_LIST_NULL:
            SET_VECTOR_ELT(list, i, R_NilValue);
            break;
        case YAML12_LIST_LOGICAL:
            SET_VECTOR_ELT(list, i, Rf_ScalarLogical(element->int_value));
            break;
        case YAML12_LIST_INTEGER:
            SET_VECTOR_ELT(list, i, Rf_ScalarInteger(element->int_value));
            break;
        case YAML12_LIST_REAL:
            SET_VECTOR_ELT(list, i, Rf_ScalarReal(element->real_value));
            break;
        case YAML12_LIST_STRING: {
            struct yaml12_string_data string = {
                element->string_value,
                element->string_len,
                element->string_is_na,
            };
            SET_VECTOR_ELT(list, i, Rf_ScalarString(yaml12_make_char(&string)));
            break;
        }
        case YAML12_LIST_SKIP:
            /* Rust already attached this value to the rooted target. */
            break;
        default:
            Rf_error("Internal error: unknown YAML list element kind");
        }
    }

    if (batch->names != NULL) {
        yaml12_fill_string_vector(names, batch->names, batch->length);
        Rf_setAttrib(list, R_NamesSymbol, names);
    }

    UNPROTECT(protect_count);
    return list;
}

SEXP yaml12_materialize_list(SEXP target,
                             const struct yaml12_list_element *elements,
                             const struct yaml12_string_data *names,
                             R_xlen_t length) {
    /*
     * A non-Nil target remains rooted by the borrowed Rust OwnedListSexp for
     * this synchronous call.
     */
    struct yaml12_materialize_list_data data = {target, elements, names, length};
    return yaml12_unwind_protect(yaml12_materialize_list_impl, &data);
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
