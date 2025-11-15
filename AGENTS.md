# Agent Instructions

- Don’t hand-edit generated artifacts: `man/`, `NAMESPACE`, or `R/extendr-wrappers.R`.
- When roxygen or Rust doc comments change, regenerate docs/wrappers from the package root with `rextendr::document(); devtools::document()`.
- Before wrapping up, run formatters: `cargo fmt` and `air format .`
- Run R tests with `Rscript -e 'devtools::test()'`; run a full check with `devtools::check()`.
- Run Rust lints with `cargo clippy`.
