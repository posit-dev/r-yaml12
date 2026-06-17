test_that("vendored Rust authors include crates without Cargo author metadata", {
  authors <- readLines(system.file("AUTHORS", package = "yaml12"), warn = FALSE)

  for (crate in c("extendr-api", "extendr-ffi", "extendr-macros")) {
    expect_true(any(grepl(
      paste0(
        "^ - ",
        crate,
        " 0[.]9[.]0: authors not provided in Cargo metadata$"
      ),
      authors
    )))
  }
})
