test_that("vendored Rust authors include savvy crates", {
  authors <- readLines(system.file("AUTHORS", package = "yaml12"), warn = FALSE)

  expect_true(any(grepl(
    "^ - savvy 0[.]8[.]13: Hiroaki Yutani$",
    authors
  )))
  expect_true(any(grepl(
    "^ - savvy-ffi 0[.]8[.]14: Hiroaki Yutani$",
    authors
  )))
})
