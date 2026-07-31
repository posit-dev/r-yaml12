# Format or write R objects as YAML 1.2.

`format_yaml()` returns YAML as a character string. `write_yaml()`
writes a YAML stream to a file or stdout and always emits document start
(`---`) markers and a final end (`...`) marker. Both functions honor a
`yaml_tag` attribute on values (see examples).

Long strings are automatically wrapped: when a string would produce a
line wider than `width` columns, it is emitted as a YAML folded block
scalar (`>-`) broken at word boundaries. Folding turns each line break
back into a single space, so wrapped strings round-trip through
[`parse_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/parse_yaml.md)
unchanged. Strings without a safe break point (e.g. no spaces) are left
on one line, and mapping keys are never wrapped.

## Usage

``` r
format_yaml(value, multi = FALSE, width = 80)

write_yaml(value, path = NULL, multi = FALSE, width = 80)
```

## Arguments

- value:

  Any R object composed of lists, atomic vectors, and scalars.

- multi:

  When `TRUE`, treat `value` as a list of YAML documents and encode a
  stream.

- width:

  Target maximum line width in columns; strings that would produce wider
  lines are wrapped. Individual lines may still exceed `width` when
  there is no safe break point (e.g. a single long word) or under deep
  indentation. Use `Inf` to disable wrapping.

- path:

  Scalar string file path to write YAML to when using `write_yaml()`.
  When `NULL` (the default), write to R's standard output connection.

## Value

`format_yaml()` returns a scalar character string containing YAML.
`write_yaml()` invisibly returns `value`.

## Examples

``` r
cat(format_yaml(list(foo = 1, bar = list(TRUE, NA))))
#> foo: 1
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
#> foo: 1
#> bar:
#>   - 2
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
