
<!-- README.md is generated from README.Rmd. Please edit that file -->

# yaml12

<!-- badges: start -->

[![R-CMD-check](https://github.com/posit-dev/r-yaml12/actions/workflows/R-CMD-check.yaml/badge.svg)](https://github.com/posit-dev/r-yaml12/actions/workflows/R-CMD-check.yaml)

<!-- badges: end -->

A YAML 1.2 parser/formatter for R, implemented in Rust for speed and
correctness. Built on the excellent
[`saphyr`](https://github.com/saphyr-rs/saphyr) crate.

## Installation

You can install yaml12 from CRAN with:

``` r
install.packages("yaml12")
```

You can install the development version of yaml12 from
[GitHub](https://github.com/) with:

``` r
# install.packages("pak")
pak::pak("posit-dev/r-yaml12")
```

## Quick start

``` r
library(yaml12)

yaml <- "
title: A modern YAML parser and emitter written in Rust
properties: [fast, correct, safe, simple]
sequences:
  simplify: true
"

doc <- parse_yaml(yaml)
str(doc)
#> List of 3
#>  $ title     : chr "A modern YAML parser and emitter written in Rust"
#>  $ properties: chr [1:4] "fast" "correct" "safe" "simple"
#>  $ sequences :List of 1
#>   ..$ simplify: logi TRUE
```

### Reading and writing files

``` r
value_out <- list(alpha = 1L, nested = c(TRUE, NA))

write_yaml(value_out, "my.yaml")
value_in <- read_yaml("my.yaml")

stopifnot(identical(value_out, value_in))

# Multi-document streams
docs_out <- list(list(foo = 1L), list(bar = c(2L, NA)))

write_yaml(docs_out, "my-multi.yaml", multi = TRUE)
docs_in <- read_yaml("my-multi.yaml", multi = TRUE)

stopifnot(identical(docs_in, docs_out))
```

### Tag handlers

Handlers let you opt into custom behavior for tagged nodes while keeping
the default parser strict and safe.

``` r
yaml <- "
- !upper [rust, r]
- !expr 6 * 7
"

handlers <- list(
  "!expr"  = function(x) eval(str2lang(x), baseenv()),
  "!upper" = toupper
)

parse_yaml(yaml, handlers = handlers)
#> [[1]]
#> [1] "RUST" "R"   
#> 
#> [[2]]
#> [1] 42
```

### Non-string mapping keys

YAML mappings can use keys that R cannot store directly as names (for
example, booleans, numbers, or tagged strings). When that happens,
`parse_yaml()` still returns a named list but also attaches a
`yaml_keys` attribute containing the original YAML keys:

``` r
yaml <- "
true: a
null: b
!custom foo: c
"

parsed <- parse_yaml(yaml)

stopifnot(identical(
  parsed,
  structure(
    list("a", "b", "c"),
    names = c("", "", ""),
    yaml_keys = list(TRUE, NULL, structure("foo", yaml_tag = "!custom"))
  )
))
```

### Formatting and round-tripping

The `yaml_tag` and `yaml_keys` attributes are also hooks for customizing
output: tags on values round-trip, and `yaml_keys` allows you to emit
mappings with non-string or tagged keys that can’t be represented as an
R name.

``` r
obj <- list(
  seq = 1:2,
  map = list(key = "value"),
  tagged = structure("1 + 1", yaml_tag = "!expr"),
  keys = structure(
    list("a", "b", "c"),
    names = c("plain", "", ""),
    yaml_keys = list("plain", TRUE, structure("foo", yaml_tag = "!custom"))
  )
)

yaml <- format_yaml(obj)
cat(yaml)
#> seq:
#>   - 1
#>   - 2
#> map:
#>   key: value
#> tagged: !expr 1 + 1
#> keys:
#>   plain: a
#>   true: b
#>   !custom foo: c

roundtripped <- parse_yaml(yaml)
identical(obj, roundtripped)
#> [1] TRUE
```

## Documentation

- YAML quick primer:
  <https://posit-dev.github.io/r-yaml12/articles/yaml-2-minute-intro.html>.
- Tags, handlers, anchors, and advanced YAML features:
  <https://posit-dev.github.io/r-yaml12/articles/yaml-tags-and-advanced-features.html>.
