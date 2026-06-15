load_windows_rust_target <- function() {
  env <- new.env(parent = baseenv())
  source(test_path("../../tools/windows-rust-target.R"), local = env)
  env$windows_rust_target
}

test_that("windows Rust target selection supports ARM Windows", {
  windows_rust_target <- load_windows_rust_target()

  expect_identical(
    windows_rust_target(
      platform = "aarch64-w64-mingw32",
      arch = "aarch64",
      compiled_by = "clang"
    ),
    "aarch64-pc-windows-gnullvm"
  )

  expect_identical(
    windows_rust_target(
      platform = "x86_64-w64-mingw32",
      arch = "x86_64",
      compiled_by = "gcc"
    ),
    "x86_64-pc-windows-gnu"
  )

  expect_identical(
    windows_rust_target(
      platform = "i386-w64-mingw32",
      arch = "i386",
      compiled_by = "gcc"
    ),
    "i686-pc-windows-gnu"
  )
})

test_that("windows Makevars uses the Rust target helper", {
  makevars_win <- readLines(test_path("../../src/Makevars.win.in"))

  expect_true(any(grepl("windows-rust-target.R", makevars_win, fixed = TRUE)))
  expect_false(any(grepl(
    "$(WIN)))-pc-windows-gnu",
    makevars_win,
    fixed = TRUE
  )))
})
