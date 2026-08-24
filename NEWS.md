# yaml12 0.2.0 (2026-08-24)

* The parser now accepts and ignores reserved directives such as `%***` and
  treats indented `---` text as part of a multiline plain scalar.

* `parse_yaml()` and `read_yaml()` now preserve tags in the
  `tag:yaml.org,2002:` namespace that are not converted to built-in R values as
  a `yaml_tag` attribute.

* `parse_yaml()` now invokes mapping-key handlers once per key.

* `write_yaml()` no longer adds optional document end (`...`) markers. Written
  documents still begin with `---` and end with a newline.

* `format_yaml()` and `write_yaml()` now emit `Inf`, `-Inf`, and `NaN` as
  `.Inf`, `-.Inf`, and `.NaN` so they round-trip as doubles (#9).

* `write_yaml()` gains an `append` argument for adding YAML documents to
  existing files (#4).

* Replaced the Rust-side R API layer with `savvy`/`savvy-ffi` bindings.
  Source installs now require rustc 1.71 or newer. Vendored Rust crate
  attribution now points to crate repository metadata when Cargo does not
  provide crate authors.

* `format_yaml()` and `write_yaml()` now emit whole-valued doubles with a
  decimal suffix (for example, `100.0` instead of `100`), so parsing the result
  preserves their R double type.

* `format_yaml()` and `write_yaml()` now automatically wrap long single-line
  strings and multiline strings containing unindented, single-line paragraphs
  separated by exactly one blank line. Strings with a safe word boundary are
  emitted as folded block scalars: `>-` preserves no final newline, while `>`
  preserves exactly one. Paragraph breaks and all other string content
  round-trip through `parse_yaml()` unchanged. The new `width` argument
  controls the target line width; `width = NULL` and `width = Inf` disable
  folded wrapping, mapping keys never use block scalars, and unsafe values use
  a lossless fallback. Mapping keys longer than YAML's implicit-key limit use
  explicit mapping syntax.

* Multiline literal blocks now use explicit indentation indicators to preserve
  leading spaces or tabs. They keep physical blank lines empty and preserve
  later-indented lines. Root literal content is indented so document markers
  inside a string cannot end the scalar.

* `format_yaml()` and `write_yaml()` now quote strings only when YAML 1.2
  requires it. Strings such as `"yes"`, `"don't"`, `"a,b"`, `"f[0]"`, or
  `".gitignore"` are emitted as plain scalars (YAML 1.2 has no legacy
  `yes`/`no`/`on`/`off` booleans, and indicator characters only need quoting
  in positions where they are ambiguous). Strings the YAML 1.2 core schema
  would read back as null, boolean, or a number (e.g. `"true"`, `"0x1F"`,
  `"0o17"`, `".inf"`) are still quoted, as are structurally unsafe ones
  (leading indicators, `": "`, `" #"`, leading/trailing white space, or
  document-marker prefixes).

* `read_yaml()` and `write_yaml()` now expand tilde prefixes (`~`) in `path`,
  as by `path.expand()` (#7).

* Fixed source installs on Windows ARM64 by selecting the
  `aarch64-pc-windows-gnullvm` Rust target. Windows source installs now also
  fail early with instructions if the required Rust target is not installed.
  The README documents that Windows ARM64 source installs also require
  Microsoft C++ Build Tools with ARM64 components.

* Added a benchmarks article comparing read/write performance against the
  `yaml` package (#2).

# yaml12 0.1.0

* Initial CRAN submission.
