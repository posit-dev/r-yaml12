test_that("native errors report the public wrapper call", {
  err <- tryCatch(
    parse_yaml(NA),
    error = identity
  )

  expect_s3_class(err, "error")
  expect_identical(conditionCall(err), quote(parse_yaml(NA)))
})

test_that("native registration rejects string lookup", {
  expect_error(
    .Call("savvy_parse_yaml_native__impl", "", FALSE, TRUE, NULL),
    "C symbol name",
    fixed = TRUE
  )
})
