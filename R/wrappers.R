#' @usage NULL
#' @useDynLib yaml12, .registration = TRUE
NULL

#' Parse YAML 1.2 document(s) into base R structures.
#'
#' `parse_yaml()` takes strings of YAML; `read_yaml()` reads from a file path.
#'
#' YAML tags without a corresponding `handler` are preserved in a `yaml_tag`
#' attribute. Mappings with keys that are not all simple scalar strings are
#' returned as a named list with a `yaml_keys` attribute.
#'
#' @section Duplicate mapping keys:
#' YAML 1.2 requires mapping keys to be unique, but `parse_yaml()` and
#' `read_yaml()` do not currently reject duplicates. When a key is repeated
#' with the same spelling and quoting style, only its last value is retained.
#' Keys written differently, such as `key` and `"key"`, may remain as separate
#' entries even when they produce the same R name. A future release may warn or
#' error for these cases, so this behavior should not be relied upon.
#'
#' @param text Character vector; elements are concatenated with `"\n"`.
#' @param path Scalar string path to a YAML file. Tilde prefixes (`~`) are
#'   expanded as by [base::path.expand()].
#' @param multi When `TRUE`, return a list containing all documents in the
#'   stream.
#' @param simplify When `FALSE`, keep YAML sequences as R lists instead of
#'   simplifying to atomic vectors.
#' @param handlers Named list of R functions with names corresponding to YAML
#'   tags; matching handlers transform tagged values.
#' @return When `multi = FALSE`, returns a parsed R object for the first
#'   document. When `multi = TRUE`, returns a list of parsed documents.
#' @rdname parse_yaml
#' @examples
#' dput(parse_yaml("foo: [1, 2, 3]"))
#'
#' # homogeneous sequences simplify by default.
#' # YAML null maps to NA in otherwise homogeneous sequences.
#' dput(parse_yaml("foo: [1, 2, 3, null]"))
#'
#' # mixed type sequence never simplify
#' dput(parse_yaml("[1, true, cat]"))
#'
#' # use `simplify=FALSE` to always return sequences as lists.
#' str(parse_yaml("foo: [1, 2, 3, null]", simplify = FALSE))
#'
#' # Parse multiple documents when requested.
#' stream <- "
#' ---
#' first: 1
#' ---
#' second: 2
#' "
#' str(parse_yaml(stream, multi = TRUE))
#'
#' # Read from a file; keep sequences as lists.
#' path <- tempfile(fileext = ".yaml")
#' writeLines("alpha: [true, null]\nbeta: 3.5", path)
#' str(read_yaml(path, simplify = FALSE))
#' @export
parse_yaml <- function(text, multi = FALSE, simplify = TRUE, handlers = NULL) {
  .Call(savvy_parse_yaml_native__impl, text, multi, simplify, handlers)
}

#' Debug helper: print saphyr `Yaml` nodes without converting to R objects.
#'
#' @noRd
dbg_yaml <- function(text) {
  invisible(.Call(savvy_dbg_yaml_native__impl, text))
}

#' Format or write R objects as YAML 1.2.
#'
#' @description
#' `format_yaml()` returns YAML as a character string. `write_yaml()` writes a
#' YAML stream to a file or stdout and starts every document with a document
#' start (`---`) marker. Both functions honor a `yaml_tag` attribute on values
#' (see examples).
#'
#' Long single-line strings and multiline strings containing unindented,
#' single-line paragraphs separated by exactly one blank line are
#' automatically wrapped when a line would be wider than `width` columns and
#' has a safe word boundary. Wrapped strings use a YAML folded block scalar:
#' `>-` preserves no final newline, while `>` preserves exactly one. Paragraph
#' breaks and all other string content round-trip through `parse_yaml()`
#' unchanged. Strings that cannot be folded losslessly use a literal or quoted
#' representation, and mapping keys are never wrapped.
#'
#' Literal blocks use explicit indentation indicators when needed to preserve
#' leading spaces or tabs. They keep physical blank lines empty and preserve
#' later-indented lines.
#'
#' @param value Any R object composed of lists, atomic vectors, and scalars.
#' @param path Scalar string file path to write YAML to when using
#'   `write_yaml()`. Tilde prefixes (`~`) are expanded as by
#'   [base::path.expand()]. When `NULL` (the default), write to R's standard
#'   output connection.
#' @param multi When `TRUE`, treat `value` as a list of YAML documents and
#'   encode a stream.
#' @param append When `TRUE`, append to `path` instead of replacing it. Defaults
#'   to `FALSE`.
#' @param width Target maximum line width in columns. Long single-line strings
#'   and unindented paragraphs with safe word boundaries are wrapped.
#'   Individual lines may still exceed `width` when there is no safe break
#'   point (e.g. a single long word) or under deep indentation. Use `NULL` or
#'   `Inf` to disable wrapping.
#' @return `format_yaml()` returns a scalar character string containing YAML.
#'   `write_yaml()` invisibly returns `value`.
#' @rdname format_yaml
#' @export
#' @examples
#' cat(format_yaml(list(foo = 1, bar = list(TRUE, NA))))
#'
#' docs <- list("first", "second")
#' cat(format_yaml(docs, multi = TRUE))
#'
#' tagged <- structure("1 + 1", yaml_tag = "!expr")
#' cat(tagged_yaml <- format_yaml(tagged), "\n")
#'
#' dput(parse_yaml(tagged_yaml))
format_yaml <- function(value, multi = FALSE, width = 80L) {
  .Call(savvy_format_yaml_native__impl, value, multi, width)
}

#' Read YAML 1.2 document(s) from a file path.
#'
#' @rdname parse_yaml
#' @export
read_yaml <- function(path, multi = FALSE, simplify = TRUE, handlers = NULL) {
  .Call(savvy_read_yaml_native__impl, path, multi, simplify, handlers)
}

#' Write an R object as YAML 1.2 to a file.
#'
#' @rdname format_yaml
#' @examples
#'
#'
#' write_yaml(list(foo = 1, bar = list(2, "baz")))
#'
#' write_yaml(list("foo", "bar"), multi = TRUE)
#'
#' tagged <- structure("1 + 1", yaml_tag = "!expr")
#' write_yaml(tagged)
#' @export
write_yaml <- function(
  value,
  path = NULL,
  multi = FALSE,
  append = FALSE,
  width = 80L
) {
  invisible(
    .Call(savvy_write_yaml_native__impl, value, path, multi, width, append)
  )
}
