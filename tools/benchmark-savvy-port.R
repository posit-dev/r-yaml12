if (!requireNamespace("bench", quietly = TRUE)) {
  stop("Install the `bench` package to run this benchmark.", call. = FALSE)
}

run_benchmarks <- function() {
  library_path <- Sys.getenv("YAML12_BENCH_LIB", "")
  if (nzchar(library_path)) {
    .libPaths(c(library_path, .libPaths()))
  }
  suppressPackageStartupMessages(library(yaml12))

  package_path <- find.package("yaml12")
  dll_path <- getLoadedDLLs()[["yaml12"]][["path"]]
  if (nzchar(library_path)) {
    library_path <- normalizePath(library_path, winslash = "/", mustWork = TRUE)
    package_path <- normalizePath(package_path, winslash = "/", mustWork = TRUE)
    stopifnot(startsWith(package_path, paste0(library_path, "/")))
  }

  implementation <- Sys.getenv("YAML12_BENCH_IMPLEMENTATION", "unknown")
  revision <- Sys.getenv("YAML12_BENCH_REVISION", "unknown")
  run <- as.integer(Sys.getenv("YAML12_BENCH_RUN", "1"))
  min_time <- as.numeric(Sys.getenv("YAML12_BENCH_MIN_TIME", "0.2"))
  max_iterations <- as.integer(
    Sys.getenv("YAML12_BENCH_MAX_ITERATIONS", "100000")
  )

  stopifnot(
    length(run) == 1L,
    !is.na(run),
    run > 0L,
    length(min_time) == 1L,
    is.finite(min_time),
    min_time > 0,
    length(max_iterations) == 1L,
    !is.na(max_iterations),
    max_iterations > 0L
  )

  integer_yaml <- function(n) {
    paste0("[", paste(seq_len(n), collapse = ", "), "]")
  }

  mixed_node <- list(
    str = c(
      "Lorem ipsum dolor sit amet, vel accumsan vitae faucibus ultrices leo",
      "neque? Et cursus lacinia, ut, sit donec facilisi eu interdum. Dui",
      "ipsum, vitae ligula commodo convallis ac sed nunc. Ipsum at nec lacus",
      "eros suscipit vitae."
    ),
    block_str = "lorem \n ipsum \n dolor\n",
    bools = c(TRUE, FALSE),
    ints = c(123L, -123L),
    floats = c(123.456, -123.456),
    null = NULL
  )

  mixed_yaml_node <- paste(
    "- str:",
    "  - Lorem ipsum dolor sit amet",
    "  - neque et cursus lacinia",
    "  block_str: |-",
    "    lorem",
    "     ipsum",
    "     dolor",
    "  bools: [true, false]",
    "  ints: [123, -123]",
    "  floats: [123.456, -123.456]",
    "  null: null",
    sep = "\n"
  )

  mixed_yaml <- function(n) {
    paste(rep(mixed_yaml_node, n), collapse = "\n")
  }

  handler_yaml <- function(n) {
    paste(
      "values:",
      paste0("  - !identity value_", seq_len(n), collapse = "\n"),
      sep = "\n"
    )
  }

  handlers <- list("!identity" = identity)
  failing_handlers <- list("!fail" = function(value) stop(value, call. = FALSE))
  handler_yaml_1 <- handler_yaml(1L)
  handler_yaml_1000 <- handler_yaml(1000L)
  marked_utf8 <- rawToChar(as.raw(c(0xc3, 0xa9)))
  Encoding(marked_utf8) <- "UTF-8"
  latin1 <- marked_utf8
  Encoding(latin1) <- "latin1"
  marked_utf8_line <- rawToChar(as.raw(c(0x2d, 0x20, 0xc3, 0xa9)))
  Encoding(marked_utf8_line) <- "UTF-8"
  latin1_line <- marked_utf8_line
  Encoding(latin1_line) <- "latin1"
  ascii_65536 <- strrep("a", 65536L)
  empty_text <- character()
  write_value <- list(value = 1L)

  home <- normalizePath(path.expand("~"), winslash = "/", mustWork = TRUE)
  read_path <- tempfile(
    pattern = ".yaml12-benchmark-read-",
    fileext = ".yaml",
    tmpdir = home
  )
  write_path <- tempfile(
    pattern = ".yaml12-benchmark-write-",
    fileext = ".yaml",
    tmpdir = home
  )
  read_tilde_path <- paste0("~", substring(read_path, nchar(home) + 1L))
  write_tilde_path <- paste0("~", substring(write_path, nchar(home) + 1L))
  writeLines("value: 1", read_path)
  on.exit(unlink(c(read_path, write_path)), add = TRUE)

  cases <- list(
    parse_empty = function() parse_yaml(empty_text),
    parse_scalar = function() parse_yaml("value"),
    parse_ascii_65536 = function() parse_yaml(ascii_65536),
    format_null = function() format_yaml(NULL, width = Inf),
    format_ascii_65536 = function() format_yaml(ascii_65536, width = Inf),
    parse_error = function() {
      tryCatch(parse_yaml(NA_character_), error = identity)
    },
    handler_error = function() {
      tryCatch(
        parse_yaml("!fail value", handlers = failing_handlers),
        error = identity
      )
    },
    handler_1 = function() {
      parse_yaml(handler_yaml_1, handlers = handlers)
    },
    handler_1000 = function() {
      parse_yaml(handler_yaml_1000, handlers = handlers)
    },
    read_absolute_path = function() read_yaml(read_path),
    read_tilde_path = function() read_yaml(read_tilde_path),
    write_absolute_path = function() {
      write_yaml(write_value, write_path, width = Inf)
    },
    write_tilde_path = function() {
      write_yaml(write_value, write_tilde_path, width = Inf)
    }
  )

  for (n in c(64L, 4096L)) {
    local({
      size <- n
      suffix <- as.character(size)
      unnamed <- rep(list(1L), size)
      named <- stats::setNames(unnamed, sprintf("key_%05d", seq_len(size)))
      strings <- rep(c("alpha", "beta"), length.out = size)
      integers <- integer_yaml(size)
      string_lines <- rep("- value", size)
      mapping <- paste(
        sprintf("key_%05d: null", seq_len(size)),
        collapse = "\n"
      )

      cases[[paste0("format_list_", suffix)]] <<- function() {
        format_yaml(unnamed, width = Inf)
      }
      cases[[paste0("format_named_list_", suffix)]] <<- function() {
        format_yaml(named, width = Inf)
      }
      cases[[paste0("format_strings_", suffix)]] <<- function() {
        format_yaml(strings, width = Inf)
      }
      cases[[paste0("parse_integer_vector_", suffix)]] <<- function() {
        parse_yaml(integers)
      }
      cases[[paste0("parse_integer_list_", suffix)]] <<- function() {
        parse_yaml(integers, simplify = FALSE)
      }
      cases[[paste0("parse_string_lines_", suffix)]] <<- function() {
        parse_yaml(string_lines, simplify = FALSE)
      }
      cases[[paste0("parse_mapping_", suffix)]] <<- function() {
        parse_yaml(mapping, simplify = FALSE)
      }

      if (size == 4096L) {
        marked_utf8_strings <- rep(marked_utf8, size)
        latin1_strings <- rep(latin1, size)
        marked_utf8_lines <- rep(marked_utf8_line, size)
        latin1_lines <- rep(latin1_line, size)

        cases$format_strings_utf8_4096 <<- function() {
          format_yaml(marked_utf8_strings, width = Inf)
        }
        cases$format_strings_latin1_4096 <<- function() {
          format_yaml(latin1_strings, width = Inf)
        }
        cases$parse_string_lines_utf8_4096 <<- function() {
          parse_yaml(marked_utf8_lines, simplify = FALSE)
        }
        cases$parse_string_lines_latin1_4096 <<- function() {
          parse_yaml(latin1_lines, simplify = FALSE)
        }
      }
    })
  }

  for (n in c(32L, 1024L)) {
    local({
      size <- n
      suffix <- as.character(size)
      object <- rep(list(mixed_node), size)
      yaml <- mixed_yaml(size)

      cases[[paste0("format_mixed_", suffix)]] <<- function() {
        format_yaml(object, width = Inf)
      }
      cases[[paste0("parse_mixed_", suffix)]] <<- function() {
        parse_yaml(yaml, simplify = FALSE)
      }
    })
  }

  case_pattern <- Sys.getenv("YAML12_BENCH_CASE_PATTERN", "")
  if (nzchar(case_pattern)) {
    cases <- cases[grepl(case_pattern, names(cases))]
  }
  if (length(cases) == 0L) {
    stop(
      "No benchmark cases matched `YAML12_BENCH_CASE_PATTERN`.",
      call. = FALSE
    )
  }

  stopifnot(
    is.null(parse_yaml(empty_text)),
    identical(parse_yaml("value"), "value"),
    identical(format_yaml(NULL, width = Inf), "~"),
    length(parse_yaml(mixed_yaml(1L), simplify = FALSE)) == 1L
  )

  set.seed(run)
  case_order <- sample(names(cases))
  results <- vector("list", length(case_order))

  for (i in seq_along(case_order)) {
    case <- case_order[[i]]
    fn <- cases[[case]]
    invisible(fn())
    gc(FALSE)

    mark <- bench::mark(
      fn(),
      check = FALSE,
      memory = FALSE,
      filter_gc = FALSE,
      min_time = min_time,
      max_iterations = max_iterations
    )

    results[[i]] <- data.frame(
      implementation = implementation,
      revision = revision,
      run = run,
      order = i,
      case = case,
      median_seconds = as.numeric(mark$median),
      iterations_per_second = as.numeric(mark$`itr/sec`),
      iterations = mark$n_itr,
      garbage_collections = mark$n_gc,
      package_version = as.character(utils::packageVersion("yaml12")),
      package_path = package_path,
      dll_path = dll_path,
      r_version = R.version.string,
      platform = R.version$platform,
      stringsAsFactors = FALSE
    )
  }

  results <- do.call(rbind, results)
  results <- results[order(results$case), ]
  print(results, digits = 4, row.names = FALSE)

  out <- Sys.getenv("YAML12_BENCH_OUT", "")
  if (nzchar(out)) {
    utils::write.csv(results, out, row.names = FALSE)
  }
}

run_benchmarks()
