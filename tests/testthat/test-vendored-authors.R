test_that("vendored Rust authors point to repositories when authors are missing", {
  authors <- readLines(system.file("AUTHORS", package = "yaml12"), warn = FALSE)

  for (crate in c("extendr-api", "extendr-ffi", "extendr-macros")) {
    expect_true(any(grepl(
      paste0(
        "^ - ",
        crate,
        " 0[.]9[.]0: see https://github[.]com/extendr/extendr$"
      ),
      authors
    )))
  }
})
