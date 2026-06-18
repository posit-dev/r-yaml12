# Agent Notes

## Map

- `R/`: R API; `R/wrappers.R` contains public wrappers and roxygen docs.
- `src/rust/`: nested Rust staticlib crate; run Cargo commands here.
- `configure*`, `tools/config.R`, `src/Makevars*.in`: generate Makevars and
  invoke Cargo during R package builds.
- `tests/testthat/`: public API tests, YAML suite, Windows target checks, and
  vendored-author checks.
- `vignettes/`, `README.Rmd`: user docs.

## Notes

- Do not hand-edit generated files: `man/`, `NAMESPACE`, `src/init.c`,
  `src/rust/api.h`, `inst/AUTHORS`, `src/Makevars`, `src/Makevars.win`.
- If Rust `#[savvy]` entrypoints change, regenerate savvy glue from the package
  root with `savvy-cli update .`. Keep the generated `src/init.c` and
  `src/rust/api.h`; do not commit `R/000-wrappers.R` unless the public wrapper
  strategy changes.
- If roxygen, exports, or Rd-facing surface change, regenerate from the package
  root with `devtools::document()`.
- Direct Cargo work happens in `src/rust`; Cargo discovery depends on the
  working directory.
- R package builds happen from the package root and invoke Rust through
  `configure*`, generated Makevars, and Cargo.
- Treat `src/rust/Cargo.lock`, `src/rust/vendor.tar.xz`,
  `src/rust/vendor-config.toml`, and `inst/AUTHORS` as one dependency snapshot.
- CRAN-style package builds are offline when `vendor.tar.xz` exists and
  `NOT_CRAN` is unset. Offline failures often mean stale vendor contents,
  missing crates, or a lockfile/vendor mismatch.
- `saphyr` uses `t-kalinowski/saphyr` branch `r-patched`; keep
  `vendor-config.toml` aligned.
- Rust: prefer borrowed `&str` slices from the input buffer, allocate `String`
  only when needed, and inline format expressions like `"{foo}"`.

## Validate

- Rust-only, from `src/rust`: `cargo check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt`, `cargo build`.
- R package, from root: `Rscript -e 'devtools::test()'`.
- R/docs format, from root: `air format .`.
