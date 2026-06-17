# Changelog

## yaml12 (development version)

- Fixed source installs on Windows ARM64 by selecting the
  `aarch64-pc-windows-gnullvm` Rust target. Windows source installs now
  also fail early with instructions if the required Rust target is not
  installed. The README documents that Windows ARM64 source installs
  also require Microsoft C++ Build Tools with ARM64 components.

- Added a benchmarks article comparing read/write performance against
  the `yaml` package
  ([\#2](https://github.com/posit-dev/r-yaml12/issues/2)).

## yaml12 0.1.0

CRAN release: 2025-12-11

- Initial CRAN submission.
