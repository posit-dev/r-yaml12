# Agent Notes

## Map

- `R/`: R API; keep `R/extendr-wrappers.R` synchronized with the Rust
  API.
- `src/rust/`: nested Rust staticlib crate; run Cargo commands here.
- `configure*`, `tools/config.R`, `src/Makevars*.in`: generate Makevars
  and invoke Cargo during R package builds.
- `tests/testthat/`: public API tests, YAML suite, Windows target
  checks, and vendored-author checks.
- `vignettes/`, `README.Rmd`: user docs.

## Notes

- Do not hand-edit generated files: `man/`, `NAMESPACE`, `inst/AUTHORS`,
  `src/Makevars`, and `src/Makevars.win`.
- The custom Rust/C entrypoint layer does not expose rextendr’s
  `wrap__make_yaml12_wrappers` generator. Do not use
  `rextendr::document()`; it cannot update the checked-in wrappers.
- If Rust doc comments, exported signatures, or the Rd-facing surface
  change, update `R/extendr-wrappers.R` and `src/entrypoint.c` as
  applicable, then run `Rscript -e 'devtools::document()'` from the
  package root.
- Direct Cargo work happens in `src/rust`; Cargo discovery depends on
  the working directory.
- R package builds happen from the package root and invoke Rust through
  `configure*`, generated Makevars, and Cargo.
- Treat `src/rust/Cargo.lock`, `src/rust/vendor.tar.xz`,
  `src/rust/vendor-config.toml`, and `inst/AUTHORS` as one dependency
  snapshot.
- CRAN-style package builds are offline when `vendor.tar.xz` exists and
  `NOT_CRAN` is unset. Offline failures often mean stale vendor
  contents, missing crates, or a lockfile/vendor mismatch.
- `saphyr` uses `t-kalinowski/saphyr` branch `r-patched`; keep
  `vendor-config.toml` aligned.
- Rust: prefer borrowed `&str` slices from the input buffer, allocate
  `String` only when needed, and inline format expressions like
  `"{foo}"`.

## Validate

- Rust-only, from `src/rust`: `cargo check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt`,
  `cargo build`.
- R package, from root: `Rscript -e 'devtools::test()'`.
- R/docs format, from root: `air format .`.
