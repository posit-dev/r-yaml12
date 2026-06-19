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
  "forced symbol lookup",
  "    R_useDynamicSymbols(dll, FALSE);\n\n    // Functions for initialzation, if any.",
  "    R_useDynamicSymbols(dll, FALSE);\n    R_forceSymbols(dll, TRUE);\n\n    // Functions for initialzation, if any."
)

contents <- require_once(
  contents,
  "package initialization hook",
  "    savvy_init_yaml12__impl(dll);"
)

writeLines(contents, path)
