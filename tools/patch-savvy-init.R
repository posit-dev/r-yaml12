path <- "src/init.c"
stopifnot(file.exists(path))

contents <- paste(readLines(path, warn = FALSE), collapse = "\n")

count_fixed <- function(contents, needle) {
  starts <- gregexpr(needle, contents, fixed = TRUE)[[1L]]
  if (starts[[1L]] == -1L) {
    return(0L)
  }
  length(starts)
}

patch_once <- function(contents, label, old, new) {
  old_count <- count_fixed(contents, old)
  new_count <- count_fixed(contents, new)

  if (new_count == 1L && old_count == 0L) {
    return(contents)
  }

  if (old_count != 1L || new_count != 0L) {
    stop(
      "Unexpected savvy init.c shape while patching ",
      label,
      ": found old snippet ",
      old_count,
      " time(s) and patched snippet ",
      new_count,
      " time(s).",
      call. = FALSE
    )
  }

  sub(old, new, contents, fixed = TRUE)
}

require_once <- function(contents, label, needle) {
  count <- count_fixed(contents, needle)
  if (count != 1L) {
    stop(
      "Unexpected savvy init.c shape: expected ",
      label,
      " once, found ",
      count,
      " time(s).",
      call. = FALSE
    )
  }

  contents
}

contents <- patch_once(
  contents,
  "Rdynload include",
  "#include <R_ext/Parse.h>\n\n#include \"rust/api.h\"",
  "#include <R_ext/Parse.h>\n#include <R_ext/Rdynload.h>\n\n#include \"rust/api.h\""
)

contents <- patch_once(
  contents,
  "native error call attribution",
  "            // In case 1, the result is an error message that can be passed to\n            // Rf_errorcall() directly.\n            Rf_errorcall(R_NilValue, \"%s\", CHAR(res_aligned));",
  "            // In case 1, throw a regular R error so R reports the public\n            // wrapper call from the active evaluation context.\n            Rf_error(\"%s\", CHAR(res_aligned));"
)

contents <- patch_once(
  contents,
  "tilde path helpers",
  "static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;\n\nSEXP handle_result",
  paste0(
    "static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;\n",
    "static SEXP path_expand_sym;\n\n",
    "static inline Rboolean has_tilde_prefix(SEXP path) {\n",
    "    if (TYPEOF(path) != STRSXP || XLENGTH(path) != 1) {\n",
    "        return FALSE;\n",
    "    }\n\n",
    "    SEXP string = STRING_ELT(path, 0);\n",
    "    return string != NA_STRING && CHAR(string)[0] == '~';\n",
    "}\n\n",
    "static inline SEXP expand_tilde_path(SEXP path) {\n",
    "    SEXP call = PROTECT(Rf_lang2(path_expand_sym, path));\n",
    "    SEXP expanded = Rf_eval(call, R_BaseEnv);\n",
    "    UNPROTECT(1);\n",
    "    return expanded;\n",
    "}\n\n",
    "SEXP handle_result"
  )
)

contents <- patch_once(
  contents,
  "read path expansion",
  paste0(
    "SEXP savvy_read_yaml_native__impl(SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {\n",
    "    SEXP res = savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers);\n",
    "    return handle_result(res);\n",
    "}"
  ),
  paste0(
    "SEXP savvy_read_yaml_native__impl(SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__simplify, SEXP c_arg__handlers) {\n",
    "    if (!has_tilde_prefix(c_arg__path)) {\n",
    "        SEXP res = savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers);\n",
    "        return handle_result(res);\n",
    "    }\n\n",
    "    c_arg__path = PROTECT(expand_tilde_path(c_arg__path));\n",
    "    SEXP result = handle_result(savvy_read_yaml_native__ffi(c_arg__path, c_arg__multi, c_arg__simplify, c_arg__handlers));\n",
    "    UNPROTECT(1);\n",
    "    return result;\n",
    "}"
  )
)

contents <- patch_once(
  contents,
  "write path expansion",
  paste0(
    "SEXP savvy_write_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__width, SEXP c_arg__append) {\n",
    "    SEXP res = savvy_write_yaml_native__ffi(c_arg__value, c_arg__path, c_arg__multi, c_arg__width, c_arg__append);\n",
    "    return handle_result(res);\n",
    "}"
  ),
  paste0(
    "SEXP savvy_write_yaml_native__impl(SEXP c_arg__value, SEXP c_arg__path, SEXP c_arg__multi, SEXP c_arg__width, SEXP c_arg__append) {\n",
    "    if (!has_tilde_prefix(c_arg__path)) {\n",
    "        SEXP res = savvy_write_yaml_native__ffi(c_arg__value, c_arg__path, c_arg__multi, c_arg__width, c_arg__append);\n",
    "        return handle_result(res);\n",
    "    }\n\n",
    "    c_arg__path = PROTECT(expand_tilde_path(c_arg__path));\n",
    "    SEXP result = handle_result(savvy_write_yaml_native__ffi(c_arg__value, c_arg__path, c_arg__multi, c_arg__width, c_arg__append));\n",
    "    UNPROTECT(1);\n",
    "    return result;\n",
    "}"
  )
)

contents <- patch_once(
  contents,
  "forced symbol lookup",
  "    R_useDynamicSymbols(dll, FALSE);\n\n    // Functions for initialzation, if any.",
  "    R_useDynamicSymbols(dll, FALSE);\n    R_forceSymbols(dll, TRUE);\n\n    // Functions for initialzation, if any."
)

contents <- patch_once(
  contents,
  "path.expand symbol initialization",
  "void R_init_yaml12(DllInfo *dll) {\n    R_registerRoutines",
  "void R_init_yaml12(DllInfo *dll) {\n    path_expand_sym = Rf_install(\"path.expand\");\n    R_registerRoutines"
)

contents <- require_once(
  contents,
  "package initialization hook",
  "    savvy_init_yaml12__impl(dll);"
)

writeLines(contents, path)

api_path <- "src/rust/api.h"
stopifnot(file.exists(api_path))
api <- readLines(api_path, warn = FALSE)
while (length(api) > 0L && !nzchar(api[[length(api)]])) {
  api <- api[-length(api)]
}
writeLines(api, api_path)
