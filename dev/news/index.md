# Changelog

## yaml12 (development version)

- Replaced the Rust-side R API layer with `savvy`/`savvy-ffi` bindings.
  Source installs now require rustc 1.71 or newer. Vendored Rust crate
  attribution now points to crate repository metadata when Cargo does
  not provide crate authors.

- [`format_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  and
  [`write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  now emit whole-valued doubles with a decimal suffix (for example,
  `100.0` instead of `100`), so parsing the result preserves their R
  double type.

- [`format_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  and
  [`write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  now automatically wrap long single-line strings and multiline strings
  containing unindented, single-line paragraphs separated by exactly one
  blank line. Strings with a safe word boundary are emitted as folded
  block scalars: `>-` preserves no final newline, while `>` preserves
  exactly one. Paragraph breaks and all other string content round-trip
  through
  [`parse_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/parse_yaml.md)
  unchanged. The new `width` argument controls the target line width;
  `width = Inf` disables folded wrapping, mapping keys never use block
  scalars, and unsafe values use a lossless fallback. Mapping keys
  longer than YAML’s implicit-key limit use explicit mapping syntax.

- Multiline literal blocks now use explicit indentation indicators to
  preserve leading spaces or tabs. They keep physical blank lines empty
  and preserve later-indented lines. Root literal content is indented so
  document markers inside a string cannot end the scalar.

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

- [`read_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/parse_yaml.md)
  and
  [`write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md)
  now expand tilde prefixes (`~`) in `path`, as by
  [`path.expand()`](https://rdrr.io/r/base/path.expand.html)
  ([\#7](https://github.com/posit-dev/r-yaml12/issues/7)).

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
