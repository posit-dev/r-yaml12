library(devtools)

load_all(".", quiet = TRUE)

iterations <- as.integer(Sys.getenv("YAML12_BENCH_ITERATIONS", "15"))
stopifnot(length(iterations) == 1L, !is.na(iterations), iterations > 0L)

repetitions <- as.integer(Sys.getenv("YAML12_BENCH_REPETITIONS", "20"))
stopifnot(length(repetitions) == 1L, !is.na(repetitions), repetitions > 0L)

time_expr <- function(expr, env, iterations, repetitions) {
  timings <- numeric(iterations)
  for (i in seq_len(iterations)) {
    gc(FALSE)
    elapsed <- system.time({
      for (j in seq_len(repetitions)) {
        eval(expr, env)
      }
    })[["elapsed"]]
    timings[[i]] <- elapsed / repetitions
  }
  timings
}

bench_case <- function(name, expr) {
  timings <- time_expr(
    substitute(expr),
    parent.frame(),
    iterations,
    repetitions
  )
  data.frame(
    case = name,
    iterations = iterations,
    repetitions = repetitions,
    min = min(timings),
    median = median(timings),
    mean = mean(timings),
    max = max(timings),
    stringsAsFactors = FALSE
  )
}

flat_mapping_yaml <- paste0(
  sprintf("key_%05d: %d", seq_len(5000), seq_len(5000)),
  collapse = "\n"
)

deep_yaml <- local({
  depth <- 400L
  lines <- character(depth + 1L)
  for (i in seq_len(depth)) {
    indent <- paste(rep("  ", i - 1L), collapse = "")
    lines[[i]] <- sprintf("%slevel_%03d:", indent, i)
  }
  lines[[depth + 1L]] <- paste0(
    paste(rep("  ", depth), collapse = ""),
    "- leaf"
  )
  paste(lines, collapse = "\n")
})

scalar_yaml <- paste0(
  "- ",
  rep(c("plain string", "12345", "123.456", "true", "false", "null"), 2500),
  collapse = "\n"
)

named_list <- stats::setNames(
  as.list(seq_len(5000)),
  sprintf("key_%05d", seq_len(5000))
)

handler_yaml <- paste0(
  "values:\n",
  paste0("  - !upper value_", seq_len(2500), collapse = "\n")
)
handlers <- list("!upper" = function(x) toupper(x))

nested_handler_parse <- function() {
  handlers <- NULL
  handlers <- list(
    "!nest" = function(x) {
      level <- as.integer(x)
      if (identical(level, 20L)) {
        return(list(level = level, value = "leaf"))
      }
      child <- parse_yaml(
        sprintf("value: !nest %d", level + 1L),
        handlers = handlers
      )$value
      list(level = level, child = child)
    }
  )
  parse_yaml("value: !nest 1", handlers = handlers)
}

results <- rbind(
  bench_case("parse_large_flat_mapping", parse_yaml(flat_mapping_yaml)),
  bench_case("parse_deep_nested_mapping", parse_yaml(deep_yaml)),
  bench_case("parse_many_scalars", parse_yaml(scalar_yaml)),
  bench_case("format_named_list", format_yaml(named_list)),
  bench_case(
    "parse_handler_heavy",
    parse_yaml(handler_yaml, handlers = handlers)
  ),
  bench_case("nested_handler_callbacks", nested_handler_parse())
)

print(results, digits = 4, row.names = FALSE)

out <- Sys.getenv("YAML12_BENCH_OUT", "")
if (nzchar(out)) {
  write.csv(results, out, row.names = FALSE)
}
