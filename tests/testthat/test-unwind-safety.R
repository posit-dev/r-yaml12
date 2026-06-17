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
