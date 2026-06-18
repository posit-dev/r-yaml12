path <- "src/init.c"
stopifnot(file.exists(path))

contents <- paste(readLines(path, warn = FALSE), collapse = "\n")

patch_once <- function(contents, old, new) {
  if (grepl(new, contents, fixed = TRUE)) {
    return(contents)
  }
  stopifnot(grepl(old, contents, fixed = TRUE))
  sub(old, new, contents, fixed = TRUE)
}

contents <- patch_once(
  contents,
  "#include <Rinternals.h>\n#include <R_ext/Parse.h>",
  "#include <Rinternals.h>\n#include <R_ext/Parse.h>\n#include <R_ext/Rdynload.h>"
)

contents <- patch_once(
  contents,
  "            // In case 1, the result is an error message that can be passed to\n            // Rf_errorcall() directly.\n            Rf_errorcall(R_NilValue, \"%s\", CHAR(res_aligned));",
  "            // In case 1, throw a regular R error so R reports the public\n            // wrapper call from the active evaluation context.\n            Rf_error(\"%s\", CHAR(res_aligned));"
)

contents <- patch_once(
  contents,
  "    R_useDynamicSymbols(dll, FALSE);\n\n    // Functions for initialization, if any.",
  "    R_useDynamicSymbols(dll, FALSE);\n    R_forceSymbols(dll, TRUE);\n\n    // Functions for initialization, if any."
)

writeLines(contents, path)
