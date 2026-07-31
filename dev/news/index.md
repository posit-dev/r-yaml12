# Changelog

## yaml12 (development version)

- [`format_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  and
  [`write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  now automatically wrap long strings: strings that would produce a line
  wider than 80 columns are emitted as folded block scalars (`>-`)
  broken at word boundaries. Folding restores a single space at each
  break, so wrapped strings round-trip through
  [`parse_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/parse_yaml.md)
  unchanged. The new `width` argument controls the target line width;
  use `width = Inf` to disable wrapping.

- [`format_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  and
  [`write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  now quote strings only when YAML 1.2 requires it. Strings such as
  `"yes"`, `"don't"`, `"a,b"`, `"f[0]"`, or `".gitignore"` are emitted
  as plain scalars (YAML 1.2 has no legacy `yes`/`no`/`on`/`off`
  booleans, and indicator characters only need quoting in positions
  where they are ambiguous). Strings the YAML 1.2 core schema would read
  back as null, boolean, or a number (e.g. `"true"`, `"0x1F"`, `"0o17"`,
  `".inf"`) are still quoted, as are structurally unsafe ones (leading
  indicators, `": "`, `" #"`, leading/trailing white space, or
  document-marker prefixes).

- Updated the Rust integration to `rextendr` 0.5.0 and `extendr` 0.9.0,
  resolving `extendr` crates from crates.io. Source installs now require
  rustc 1.71 or newer. Vendored Rust crate attribution now points to
  crate repository metadata when Cargo does not provide crate authors.

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
