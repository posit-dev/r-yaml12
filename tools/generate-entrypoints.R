#!/usr/bin/env Rscript

args <- commandArgs(trailingOnly = TRUE)
check <- "--check" %in% args
root_args <- args[args != "--check"]
root <- if (length(root_args)) root_args[[1]] else "."
root <- normalizePath(root, mustWork = TRUE)

lib_path <- file.path(root, "src/rust/src/lib.rs")
c_path <- file.path(root, "src/entrypoint.c")
r_path <- file.path(root, "R/extendr-wrappers.R")

read_text <- function(path) {
  paste(readLines(path, warn = FALSE), collapse = "\n")
}

chars <- function(text) {
  strsplit(text, "", fixed = TRUE, useBytes = TRUE)[[1]]
}

find_matching <- function(text, open_pos, open = "{", close = "}") {
  x <- chars(text)
  depth <- 0L
  state <- "normal"
  i <- open_pos

  while (i <= length(x)) {
    ch <- x[[i]]
    next_ch <- if (i < length(x)) x[[i + 1L]] else ""

    if (state == "line_comment") {
      if (ch == "\n") {
        state <- "normal"
      }
      i <- i + 1L
      next
    }

    if (state == "block_comment") {
      if (ch == "*" && next_ch == "/") {
        state <- "normal"
        i <- i + 2L
      } else {
        i <- i + 1L
      }
      next
    }

    if (state == "string") {
      if (ch == "\\") {
        i <- i + 2L
      } else if (ch == "\"") {
        state <- "normal"
        i <- i + 1L
      } else {
        i <- i + 1L
      }
      next
    }

    if (state == "char") {
      if (ch == "\\") {
        i <- i + 2L
      } else if (ch == "'") {
        state <- "normal"
        i <- i + 1L
      } else {
        i <- i + 1L
      }
      next
    }

    if (ch == "/" && next_ch == "/") {
      state <- "line_comment"
      i <- i + 2L
      next
    }
    if (ch == "/" && next_ch == "*") {
      state <- "block_comment"
      i <- i + 2L
      next
    }
    if (ch == "\"") {
      state <- "string"
      i <- i + 1L
      next
    }
    if (ch == "'") {
      state <- "char"
      i <- i + 1L
      next
    }
    if (ch == open) {
      depth <- depth + 1L
    } else if (ch == close) {
      depth <- depth - 1L
      if (depth == 0L) {
        return(i)
      }
    }

    i <- i + 1L
  }

  stop("Could not find matching `", close, "`", call. = FALSE)
}

extract_macro_blocks <- function(text) {
  matches <- gregexpr("r_entrypoint!\\s*\\{", text, perl = TRUE)[[1]]
  if (identical(matches, -1L)) {
    stop("No `r_entrypoint!` blocks found", call. = FALSE)
  }

  vapply(
    matches,
    function(start) {
      prefix <- substr(text, start, nchar(text))
      open_rel <- regexpr("\\{", prefix, perl = TRUE)[[1]]
      open_pos <- start + open_rel - 1L
      close_pos <- find_matching(text, open_pos)
      substr(text, open_pos + 1L, close_pos - 1L)
    },
    character(1)
  )
}

split_top_level_commas <- function(text) {
  text <- trimws(text)
  if (!nzchar(text)) {
    return(character())
  }

  x <- chars(text)
  depth_paren <- 0L
  depth_bracket <- 0L
  depth_angle <- 0L
  state <- "normal"
  start <- 1L
  out <- character()
  i <- 1L

  while (i <= length(x)) {
    ch <- x[[i]]

    if (state == "string") {
      if (ch == "\\") {
        i <- i + 2L
      } else if (ch == "\"") {
        state <- "normal"
        i <- i + 1L
      } else {
        i <- i + 1L
      }
      next
    }

    if (ch == "\"") {
      state <- "string"
    } else if (ch == "(") {
      depth_paren <- depth_paren + 1L
    } else if (ch == ")") {
      depth_paren <- depth_paren - 1L
    } else if (ch == "[") {
      depth_bracket <- depth_bracket + 1L
    } else if (ch == "]") {
      depth_bracket <- depth_bracket - 1L
    } else if (ch == "<") {
      depth_angle <- depth_angle + 1L
    } else if (ch == ">") {
      depth_angle <- depth_angle - 1L
    } else if (
      ch == "," &&
        depth_paren == 0L &&
        depth_bracket == 0L &&
        depth_angle == 0L
    ) {
      out <- c(out, trimws(substr(text, start, i - 1L)))
      start <- i + 1L
    }

    i <- i + 1L
  }

  out <- c(out, trimws(substr(text, start, nchar(text))))
  out[nzchar(out)]
}

capture_one <- function(pattern, text, label) {
  match <- regexec(pattern, text, perl = TRUE)
  parts <- regmatches(text, match)[[1]]
  if (!length(parts)) {
    stop("Could not parse ", label, call. = FALSE)
  }
  parts[[2]]
}

parse_default <- function(arg) {
  match <- regexec(
    '#\\s*\\[\\s*extendr\\s*\\(\\s*default\\s*=\\s*"([^"]*)"\\s*\\)\\s*\\]',
    arg,
    perl = TRUE
  )
  parts <- regmatches(arg, match)[[1]]
  if (length(parts)) parts[[2]] else NA_character_
}

parse_args <- function(args_src) {
  args <- split_top_level_commas(args_src)
  if (!length(args)) {
    return(data.frame(
      name = character(),
      type = character(),
      default = character()
    ))
  }

  records <- lapply(args, function(arg) {
    arg_without_attrs <- gsub("#\\s*\\[[^]]+\\]\\s*", "", arg, perl = TRUE)
    name <- capture_one(
      "^\\s*([A-Za-z_][A-Za-z0-9_]*)\\s*:",
      arg_without_attrs,
      paste("argument in", arg)
    )
    type <- sub(
      "^\\s*[A-Za-z_][A-Za-z0-9_]*\\s*:\\s*",
      "",
      arg_without_attrs,
      perl = TRUE
    )
    data.frame(name = name, type = trimws(type), default = parse_default(arg))
  })

  do.call(rbind, records)
}

parse_block <- function(block) {
  fn_match <- regexpr(
    "fn\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*\\(",
    block,
    perl = TRUE
  )
  if (identical(fn_match[[1]], -1L)) {
    stop("Could not find entrypoint function", call. = FALSE)
  }

  fn_start <- fn_match[[1]]
  fn_header <- regmatches(block, fn_match)
  name <- capture_one(
    "fn\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*\\(",
    fn_header,
    "function name"
  )
  paren_start <- fn_start + attr(fn_match, "match.length") - 1L
  paren_end <- find_matching(block, paren_start, "(", ")")
  args_src <- substr(block, paren_start + 1L, paren_end - 1L)

  before_fn <- substr(block, 1L, fn_start - 1L)
  before_lines <- strsplit(before_fn, "\n", fixed = TRUE)[[1]]
  doc_lines <- grep("^\\s*///", before_lines, value = TRUE)
  doc_lines <- sub("^\\s*/// ?", "", doc_lines, perl = TRUE)
  invisible <- grepl(
    "#\\s*\\[\\s*extendr\\s*\\(\\s*invisible\\s*\\)\\s*\\]",
    before_fn,
    perl = TRUE
  )

  after_fn <- substr(block, paren_end + 1L, nchar(block))
  ffi_match <- regexpr("ffi\\s*\\(", after_fn, perl = TRUE)
  if (identical(ffi_match[[1]], -1L)) {
    stop("Could not find ffi block for `", name, "`", call. = FALSE)
  }
  ffi_paren_start <- paren_end +
    ffi_match[[1]] +
    attr(ffi_match, "match.length") -
    1L
  ffi_paren_end <- find_matching(block, ffi_paren_start, "(", ")")
  ffi_args <- split_top_level_commas(substr(
    block,
    ffi_paren_start + 1L,
    ffi_paren_end - 1L
  ))

  args <- parse_args(args_src)
  if (nrow(args) != length(ffi_args)) {
    stop("Typed and FFI arity disagree for `", name, "`", call. = FALSE)
  }

  list(
    name = name,
    docs = doc_lines,
    invisible = invisible,
    args = args,
    ffi_args = ffi_args
  )
}

c_formals <- function(args) {
  if (!length(args)) {
    return("void")
  }
  paste(sprintf("SEXP %s", args), collapse = ", ")
}

c_call_args <- function(args) {
  paste(args, collapse = ", ")
}

generate_c <- function(entries) {
  externs <- vapply(
    entries,
    function(entry) {
      sprintf(
        "SEXP yaml12_%s_ffi(%s);",
        entry$name,
        c_formals(entry$ffi_args)
      )
    },
    character(1)
  )

  wrappers <- vapply(
    entries,
    function(entry) {
      call_args <- c_call_args(entry$ffi_args)
      if (nzchar(call_args)) {
        call <- sprintf("yaml12_%s_ffi(%s)", entry$name, call_args)
      } else {
        call <- sprintf("yaml12_%s_ffi()", entry$name)
      }

      sprintf(
        "SEXP wrap__%s(%s) {\n    return handle_result(%s, \"%s\");\n}",
        entry$name,
        c_formals(entry$ffi_args),
        call,
        entry$name
      )
    },
    character(1)
  )

  call_entries <- vapply(
    entries,
    function(entry) {
      sprintf(
        "    {\"wrap__%s\", (DL_FUNC)&wrap__%s, %d},",
        entry$name,
        entry$name,
        length(entry$ffi_args)
      )
    },
    character(1)
  )

  wrappers <- paste(wrappers, collapse = "\n\n")

  paste(
    c(
      "// Generated by tools/generate-entrypoints.R: do not edit by hand.",
      "// We need to forward routine registration from C to Rust",
      "// to avoid the linker removing the static library.",
      "",
      "#include <Rinternals.h>",
      "#include <R_ext/Rdynload.h>",
      "#include <setjmp.h>",
      "#include <stdint.h>",
      "",
      "SEXP unwind_protect_wrapper(SEXP (*fun)(void *data), void *data);",
      "void not_so_long_jump(void *jmpbuf, Rboolean jump);",
      "",
      externs,
      "",
      "static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;",
      "",
      "static SEXP handle_result(SEXP res_, const char *call_name) {",
      "    uintptr_t res = (uintptr_t)res_;",
      "    if ((res & TAGGED_POINTER_MASK) == 1) {",
      "        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);",
      "        if (TYPEOF(res_aligned) == CHARSXP) {",
      "            SEXP call = PROTECT(Rf_lang1(Rf_install(call_name)));",
      "            Rf_errorcall(call, \"%s\", CHAR(res_aligned));",
      "        } else {",
      "            R_ReleaseObject(res_aligned);",
      "            R_ContinueUnwind(res_aligned);",
      "        }",
      "    }",
      "",
      "    return (SEXP)res;",
      "}",
      "",
      wrappers,
      "",
      "static const R_CallMethodDef CallEntries[] = {",
      call_entries,
      "    {NULL, NULL, 0}",
      "};",
      "",
      "void R_init_yaml12(void *dll) {",
      "    R_registerRoutines((DllInfo *)dll, NULL, CallEntries, NULL, NULL);",
      "    R_useDynamicSymbols((DllInfo *)dll, FALSE);",
      "    R_forceSymbols((DllInfo *)dll, TRUE);",
      "}",
      "",
      "SEXP unwind_protect_wrapper(SEXP (*fun)(void *data), void *data) {",
      "    SEXP token = R_MakeUnwindCont();",
      "    PROTECT(token);",
      "    jmp_buf jmpbuf;",
      "    if (setjmp(jmpbuf)) {",
      "        // keep token alive; tag pointer with low bit so Rust can detect jump",
      "        R_PreserveObject(token);",
      "        UNPROTECT(1);",
      "        return (SEXP)((uintptr_t)token | 1);",
      "    }",
      "    SEXP res = R_UnwindProtect(fun, data, (void (*)(void *, Rboolean)) not_so_long_jump, &jmpbuf, token);",
      "    UNPROTECT(1);",
      "    return res;",
      "}",
      "",
      "void not_so_long_jump(void *jmpbuf, Rboolean jump) {",
      "    if (jump == TRUE) {",
      "        longjmp(*(jmp_buf *)jmpbuf, 1);",
      "    }",
      "}"
    ),
    collapse = "\n"
  )
}

r_formals <- function(args) {
  if (!nrow(args)) {
    return("")
  }

  formals <- ifelse(
    is.na(args$default),
    args$name,
    sprintf("%s = %s", args$name, args$default)
  )
  paste(formals, collapse = ", ")
}

r_call <- function(entry) {
  call_args <- paste(
    c(sprintf("wrap__%s", entry$name), entry$args$name),
    collapse = ", "
  )
  call <- sprintf(".Call(%s)", call_args)
  if (entry$invisible) {
    sprintf("invisible(%s)", call)
  } else {
    call
  }
}

generate_r <- function(entries) {
  blocks <- list(c(
    "# Generated by tools/generate-entrypoints.R: Do not edit by hand",
    "",
    "# nolint start",
    "",
    "#' @usage NULL",
    "#' @useDynLib yaml12, .registration = TRUE",
    "NULL",
    ""
  ))

  for (entry in entries) {
    docs <- paste0("#' ", entry$docs)
    docs[entry$docs == ""] <- "#'"
    wrapper <- sprintf(
      "%s <- function(%s) %s",
      entry$name,
      r_formals(entry$args),
      r_call(entry)
    )
    blocks[[length(blocks) + 1L]] <- c(docs, wrapper, "")
  }

  blocks[[length(blocks) + 1L]] <- "# nolint end"
  paste(unlist(blocks), collapse = "\n")
}

write_if_changed <- function(path, contents) {
  old <- if (file.exists(path)) read_text(path) else NULL
  if (identical(old, contents)) {
    return(invisible(FALSE))
  }

  if (check) {
    stop(
      path,
      " is not up to date; run tools/generate-entrypoints.R",
      call. = FALSE
    )
  }

  writeLines(contents, path, useBytes = TRUE)
  message("Wrote ", path)
  invisible(TRUE)
}

entries <- lapply(extract_macro_blocks(read_text(lib_path)), parse_block)
write_if_changed(c_path, generate_c(entries))
write_if_changed(r_path, generate_r(entries))
