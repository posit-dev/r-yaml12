run_nested_handler_parse <- function(error_at = NULL) {
  events <- character()
  handlers <- NULL

  handlers <- list(
    "!nest" = function(x) {
      level <- as.integer(x)
      events <<- c(events, sprintf("enter:%d", level))
      on.exit(events <<- c(events, sprintf("exit:%d", level)), add = TRUE)

      if (identical(level, error_at)) {
        stop(sprintf("deep failure at level %d", level), call. = FALSE)
      }

      if (identical(level, 4L)) {
        return(list(level = level, value = "leaf"))
      }

      child <- tryCatch(
        parse_yaml(
          sprintf("value: !nest %d", level + 1L),
          handlers = handlers
        )$value,
        error = function(err) {
          events <<- c(events, sprintf("catch:%d", level))
          stop(conditionMessage(err), call. = FALSE)
        }
      )

      list(level = level, child = child)
    }
  )

  path <- tempfile(fileext = ".yaml")
  writeLines("value: !nest 1", path)

  error <- NULL
  value <- tryCatch(
    read_yaml(path, handlers = handlers),
    error = function(err) {
      error <<- err
      NULL
    }
  )

  list(value = value, error = error, events = events)
}

test_that("API errors treat percent signs literally", {
  path <- file.path(tempdir(), "yaml12-missing-%s-%d.yaml")
  err <- tryCatch(read_yaml(path), error = identity)

  expect_s3_class(err, "error")
  expect_match(conditionMessage(err), path, fixed = TRUE)
  expect_false(grepl("Failed to read `.*Failed to read", conditionMessage(err)))
})

test_that("handler longjmps do not poison later handler calls", {
  handlers <- list(
    "!boom" = function(x) stop("handler %s boom", call. = FALSE),
    "!ok" = function(x) paste0("ok:", x)
  )

  for (i in seq_len(25)) {
    expect_error(
      parse_yaml("value: !boom bad", handlers = handlers),
      "handler %s boom",
      fixed = TRUE
    )
  }

  expect_identical(
    parse_yaml("value: !ok good", handlers = handlers),
    list(value = "ok:good")
  )
})

test_that("warnings promoted to errors do not poison later parsing", {
  handlers <- list(
    "!warn" = function(x) {
      warning("handler warning", call. = FALSE)
      x
    }
  )

  for (i in seq_len(25)) {
    expect_error(
      withr::with_options(
        list(warn = 2L),
        parse_yaml("value: !warn ok", handlers = handlers)
      ),
      "converted from warning"
    )
  }

  expect_identical(parse_yaml("value: ok"), list(value = "ok"))
})

test_that("nested handler calls return through multiple Rust entrypoints", {
  out <- run_nested_handler_parse()

  expect_null(out$error)
  expect_identical(
    out$value,
    list(
      value = list(
        level = 1L,
        child = list(
          level = 2L,
          child = list(
            level = 3L,
            child = list(level = 4L, value = "leaf")
          )
        )
      )
    )
  )
  expect_identical(
    out$events,
    c(
      "enter:1",
      "enter:2",
      "enter:3",
      "enter:4",
      "exit:4",
      "exit:3",
      "exit:2",
      "exit:1"
    )
  )
})

test_that("nested handler errors unwind R frames in order", {
  out <- run_nested_handler_parse(error_at = 4L)

  expect_s3_class(out$error, "error")
  expect_match(conditionMessage(out$error), "deep failure at level 4", fixed = TRUE)
  expect_null(out$value)
  expect_identical(
    out$events,
    c(
      "enter:1",
      "enter:2",
      "enter:3",
      "enter:4",
      "exit:4",
      "catch:3",
      "exit:3",
      "catch:2",
      "exit:2",
      "catch:1",
      "exit:1"
    )
  )

  for (i in seq_len(10)) {
    out <- run_nested_handler_parse(error_at = 4L)
    expect_s3_class(out$error, "error")
    expect_match(conditionMessage(out$error), "deep failure at level 4", fixed = TRUE)
  }

  expect_identical(parse_yaml("value: ok"), list(value = "ok"))
  expect_null(run_nested_handler_parse()$error)
})
