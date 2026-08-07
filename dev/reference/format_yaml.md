# Format or write R objects as YAML 1.2.

`format_yaml()` returns YAML as a character string. `write_yaml()`
writes a YAML stream to a file or stdout and always emits document start
(`---`) markers and a final end (`...`) marker. Both functions honor a
`yaml_tag` attribute on values (see examples).

Long single-line strings and multiline strings containing unindented,
single-line paragraphs separated by exactly one blank line are
automatically wrapped when a line would be wider than `width` columns
and has a safe word boundary. Wrapped strings use a YAML folded block
scalar: `>-` preserves no final newline, while `>` preserves exactly
one. Paragraph breaks and all other string content round-trip through
[`parse_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/parse_yaml.md)
unchanged. Strings that cannot be folded losslessly use a literal or
quoted representation, and mapping keys are never wrapped.

Literal blocks use explicit indentation indicators when needed to
preserve leading spaces or tabs. They keep physical blank lines empty
and preserve later-indented lines.

## Usage

``` r
format_yaml(value, multi = FALSE, width = 80L)

write_yaml(value, path = NULL, multi = FALSE, append = FALSE, width = 80L)
```

## Arguments

- value:

  Any R object composed of lists, atomic vectors, and scalars.

- multi:

  When `TRUE`, treat `value` as a list of YAML documents and encode a
  stream.

- width:

  Target maximum line width in columns. Long single-line strings and
  unindented paragraphs with safe word boundaries are wrapped.
  Individual lines may still exceed `width` when there is no safe break
  point (e.g. a single long word) or under deep indentation. Use `Inf`
  to disable wrapping.

- path:

  Scalar string file path to write YAML to when using `write_yaml()`.
  Tilde prefixes (`~`) are expanded as by
  [`base::path.expand()`](https://rdrr.io/r/base/path.expand.html). When
  `NULL` (the default), write to R's standard output connection.

- append:

  When `TRUE`, append to `path` instead of replacing it. Defaults to
  `FALSE`.

## Value

`format_yaml()` returns a scalar character string containing YAML.
`write_yaml()` invisibly returns `value`.

## Examples

``` r
cat(format_yaml(list(foo = 1, bar = list(TRUE, NA))))
#> foo: 1.0
#> bar:
#>   - true
#>   - ~

docs <- list("first", "second")
cat(format_yaml(docs, multi = TRUE))
#> ---
#> first
#> ---
#> second

tagged <- structure("1 + 1", yaml_tag = "!expr")
cat(tagged_yaml <- format_yaml(tagged), "\n")
#> !expr 1 + 1 

dput(parse_yaml(tagged_yaml))
#> structure("1 + 1", yaml_tag = "!expr")


write_yaml(list(foo = 1, bar = list(2, "baz")))
#> ---
#> foo: 1.0
#> bar:
#>   - 2.0
#>   - baz
#> ...

write_yaml(list("foo", "bar"), multi = TRUE)
#> ---
#> foo
#> ---
#> bar
#> ...

tagged <- structure("1 + 1", yaml_tag = "!expr")
write_yaml(tagged)
#> ---
#> !expr 1 + 1
#> ...
```
