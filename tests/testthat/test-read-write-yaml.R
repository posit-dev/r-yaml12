test_that("write_yaml writes and read_yaml reads single documents", {
  path <- tempfile("yaml12-", fileext = ".yaml")
  on.exit(unlink(path), add = TRUE)

  value <- list(alpha = 1L, nested = c(TRUE, NA))
  out <- write_yaml(value, path)

  expect_null(out)
  expect_true(file.exists(path))
  expect_identical(read_yaml(path), value)
})

test_that("write_yaml and read_yaml handle multi-document streams", {
  path <- tempfile("yaml12-", fileext = ".yaml")
  on.exit(unlink(path), add = TRUE)

  docs <- list(list(foo = 1L), list(bar = list(2L, NULL)))
  write_yaml(docs, path, multi = TRUE)

  docs[[2]]$bar <- c(2L, NA)
  expect_identical(read_yaml(path, multi = TRUE), docs)
})

test_that("read_yaml errors clearly when the file cannot be read", {
  path <- tempfile("yaml12-missing-", fileext = ".yaml")
  expect_error(read_yaml(path), "Failed to read")
})

test_that("read_yaml does not simplify mixed-type sequences", {
  path <- tempfile("yaml12-", fileext = ".yaml")
  on.exit(unlink(path), add = TRUE)

  writeLines(c("- true", "- 1"), path)
  result <- read_yaml(path)

  expect_type(result, "list")
  expect_identical(result, list(TRUE, 1L))
})

test_that("read_yaml keeps tagged sequence elements as list values", {
  path <- tempfile("yaml12-", fileext = ".yaml")
  on.exit(unlink(path), add = TRUE)

  writeLines(c("- !foo 1", "- 2"), path)
  result <- read_yaml(path)
  first_tag <- attr(result[[1]], "yaml_tag")

  expect_type(result, "list")
  expect_type(first_tag, "character")
  expect_length(first_tag, 1L)
  expect_false(identical(first_tag, ""))
})
