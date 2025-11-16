# Agent Instructions

- Don’t hand-edit generated artifacts: `man/`, `NAMESPACE`, or `R/extendr-wrappers.R`.
- When roxygen or Rust doc comments change, regenerate docs/wrappers from the package root with `rextendr::document(); devtools::document()`.
- Before wrapping up, run formatters: `cargo fmt` and `air format .`
- Run R tests with `Rscript -e 'devtools::test()'`; run a full check with `devtools::check()`. Prefer to run these through the mcp tool if possible; running them via shell will require asking the user for escalated permissions. (If you see errors about missing build tools, thats due to the sandbox -- ask for elevated permissions).
- Run Rust lints with `cargo check` and `cargo clippy` from the `src/rust/src` directory.
- In format strings, always inline expressions (e.g., `"{foo}"` or `"{foo:?}"`).
