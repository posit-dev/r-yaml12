# Agent Notes

## Site map

- `R/`: package API; `R/extendr-wrappers.R` is generated.
- `src/rust/`: nested Rust staticlib crate, including `src/`, `Cargo.toml`,
  `Cargo.lock`, and the vendored dependency snapshot.
- `configure*`, `tools/config.R`, `src/Makevars*.in`: build glue that
  generates `src/Makevars` or `src/Makevars.win` and invokes Cargo from R.
- `tests/testthat/`: public API tests plus YAML suite, Windows target, and
  vendored-author checks.
- `vignettes/` and `README.Rmd`: user docs.
- `man/`, `NAMESPACE`, `R/extendr-wrappers.R`, `inst/AUTHORS`,
  `src/Makevars`, and `src/Makevars.win`: generated outputs.

## Generated outputs

- Do not hand-edit generated outputs.
- If roxygen, Rust doc comments, exports, or other Rd-facing surface changes,
  regenerate wrappers/docs from the package root with
  `rextendr::document(); devtools::document()`.

## Rust/Cargo pitfalls

- Run direct Cargo commands from `src/rust`, not the R package root. Cargo
  discovery changes with the working directory.
- Treat `src/rust/Cargo.lock`, `src/rust/vendor.tar.xz`,
  `src/rust/vendor-config.toml`, and `inst/AUTHORS` as one dependency
  snapshot.
- CRAN-style package builds are offline when `vendor.tar.xz` exists and
  `NOT_CRAN` is unset; `tools/config.R` adds `-j 2 --offline`.
- `src/Makevars.in` unpacks `vendor.tar.xz` to `src/vendor`, writes temporary
  Cargo config to `src/.cargo`, sets `CARGO_HOME`, then cleans those dirs.
  Offline failures often mean stale vendor contents, missing crates, or a
  lockfile/vendor mismatch.
- `saphyr` uses `t-kalinowski/saphyr` branch `r-patched`; keep
  `vendor-config.toml` aligned.
- Rust MSRV comes from `DESCRIPTION` and is checked by `tools/msrv.R`; Windows
  target support is checked by `tools/windows-rust-target.R`.

## Validation

- Rust-only work: switch to `src/rust` and run `cargo check`; before finishing,
  run `cargo clippy --all-targets -- -D warnings`, `cargo fmt`, and
  `cargo build`.
- R package work: test through the public R API with
  `Rscript -e 'devtools::test()'` from the package root. This path invokes Rust
  through `configure*`, generated Makevars, and Cargo.
- Formatting: run `cargo fmt` in `src/rust` for Rust and `air format .` from
  the package root for R/docs.

## Rust style

- For Rust code, prefer borrowed `&str` slices from the input buffer. Allocate
  `String` only when owned data is needed.
- Inline format string expressions, for example `"{foo}"` or `"{foo:?}"`.
