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

run_nested_handler_calling_error <- function() {
  events <- character()
  handlers <- NULL

  handlers <- list(
    "!nest" = function(x) {
      level <- as.integer(x)
      events <<- c(events, sprintf("enter:%d", level))
      on.exit(events <<- c(events, sprintf("exit:%d", level)), add = TRUE)

      if (identical(level, 4L)) {
        stop(sprintf("deep failure at level %d", level), call. = FALSE)
      }

      child <- parse_yaml(
        sprintf("value: !nest %d", level + 1L),
        handlers = handlers
      )$value

      list(level = level, child = child)
    }
  )

  path <- tempfile(fileext = ".yaml")
  writeLines("value: !nest 1", path)

  error <- tryCatch(
    withCallingHandlers(
      read_yaml(path, handlers = handlers),
      error = function(err) {
        events <<- c(events, sprintf("calling:%s", conditionMessage(err)))
      }
    ),
    error = identity
  )

  list(error = error, events = events)
}

run_nested_handler_restart_escape <- function() {
  events <- character()
  handlers <- NULL

  handlers <- list(
    "!nest" = function(x) {
      level <- as.integer(x)
      events <<- c(events, sprintf("enter:%d", level))
      on.exit(events <<- c(events, sprintf("exit:%d", level)), add = TRUE)

      if (identical(level, 4L)) {
        invokeRestart("yaml12_test_fallback", sprintf("restart:%d", level))
      }

      child <- parse_yaml(
        sprintf("value: !nest %d", level + 1L),
        handlers = handlers
      )$value

      list(level = level, child = child)
    }
  )

  path <- tempfile(fileext = ".yaml")
  writeLines("value: !nest 1", path)

  value <- withRestarts(
    read_yaml(path, handlers = handlers),
    yaml12_test_fallback = function(value) {
      list(fallback = value)
    }
  )

  list(value = value, events = events)
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

test_that("calling handlers muffle warnings from R handlers", {
  events <- character()
  handlers <- list(
    "!warn" = function(x) {
      events <<- c(events, sprintf("handler:%s", x))
      warning(sprintf("handler warning: %s", x), call. = FALSE)
      sprintf("ok:%s", x)
    }
  )

  value <- withCallingHandlers(
    parse_yaml("values: [!warn a, !warn b]", handlers = handlers),
    warning = function(err) {
      events <<- c(events, sprintf("warning:%s", conditionMessage(err)))
      invokeRestart("muffleWarning")
    }
  )

  expect_identical(value, list(values = list("ok:a", "ok:b")))
  expect_identical(
    events,
    c(
      "handler:a",
      "warning:handler warning: a",
      "handler:b",
      "warning:handler warning: b"
    )
  )
})

test_that("calling error handlers run before nested handler frames unwind", {
  out <- run_nested_handler_calling_error()

  expect_s3_class(out$error, "error")
  expect_match(conditionMessage(out$error), "deep failure at level 4", fixed = TRUE)
  expect_identical(
    out$events,
    c(
      "enter:1",
      "enter:2",
      "enter:3",
      "enter:4",
      "calling:deep failure at level 4",
      "exit:4",
      "exit:3",
      "exit:2",
      "exit:1"
    )
  )

  expect_identical(parse_yaml("value: ok"), list(value = "ok"))
})

test_that("calling handlers can recover handler errors through restarts", {
  events <- character()
  recoverable_error <- function(message) {
    structure(
      list(message = message, call = NULL),
      class = c("yaml12_test_recover", "error", "condition")
    )
  }
  handlers <- list(
    "!recover" = function(x) {
      events <<- c(events, "handler:start")
      on.exit(events <<- c(events, "handler:exit"), add = TRUE)
      withRestarts(
        {
          events <<- c(events, "handler:error")
          stop(recoverable_error(sprintf("need replacement for %s", x)))
        },
        yaml12_test_use_value = function(value) {
          events <<- c(events, "restart")
          value
        }
      )
    }
  )

  value <- withCallingHandlers(
    parse_yaml("value: !recover bad", handlers = handlers),
    yaml12_test_recover = function(err) {
      events <<- c(events, sprintf("calling:%s", conditionMessage(err)))
      invokeRestart("yaml12_test_use_value", "recovered")
    }
  )

  expect_identical(value, list(value = "recovered"))
  expect_identical(
    events,
    c(
      "handler:start",
      "handler:error",
      "calling:need replacement for bad",
      "restart",
      "handler:exit"
    )
  )
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

test_that("nested handler restarts unwind through multiple Rust entrypoints", {
  out <- run_nested_handler_restart_escape()

  expect_identical(out$value, list(fallback = "restart:4"))
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

  for (i in seq_len(10)) {
    out <- run_nested_handler_restart_escape()
    expect_identical(out$value, list(fallback = "restart:4"))
  }

  expect_identical(parse_yaml("value: ok"), list(value = "ok"))
  expect_null(run_nested_handler_parse()$error)
})
