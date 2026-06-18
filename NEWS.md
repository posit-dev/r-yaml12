# yaml12 (development version)

* Replaced the Rust-side `extendr` dependency with `savvy`/`savvy-ffi` bindings.
  Source installs now require rustc 1.71 or newer. Vendored Rust crate
  attribution now points to crate repository metadata when Cargo does not
  provide crate authors.

* Fixed source installs on Windows ARM64 by selecting the
  `aarch64-pc-windows-gnullvm` Rust target. Windows source installs now also
  fail early with instructions if the required Rust target is not installed.
  The README documents that Windows ARM64 source installs also require
  Microsoft C++ Build Tools with ARM64 components.

* Added a benchmarks article comparing read/write performance against the
  `yaml` package (#2).

# yaml12 0.1.0

* Initial CRAN submission.
