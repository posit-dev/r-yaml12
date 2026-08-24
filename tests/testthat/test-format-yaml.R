test_that("format_yaml round-trips basic R lists", {
  obj <- list(
    foo = "bar",
    baz = list(TRUE, 123L),
    qux = list(sub = list("nested", NULL))
  )

  encoded <- format_yaml(obj)
  expect_type(encoded, "character")

  expected <- list(
    foo = "bar",
    baz = list(TRUE, 123L),
    qux = list(sub = c("nested", NA))
  )
  reparsed <- parse_yaml(encoded)
  expect_identical(reparsed, expected)

  expect_identical(parse_yaml(encoded, simplify = FALSE), obj)
})

test_that("format_yaml round-trips non-finite doubles", {
  values <- c(Inf, -Inf, NaN)
  expected <- c(".Inf", "-.Inf", ".NaN")

  for (i in seq_along(values)) {
    encoded <- format_yaml(values[[i]])
    expect_identical(encoded, expected[[i]])
    expect_identical(parse_yaml(encoded), values[[i]])
  }

  values <- c(values, NA_real_)
  expect_identical(parse_yaml(format_yaml(values)), values)
})

test_that("format_yaml preserves whole-valued doubles", {
  expect_identical(format_yaml(100, width = Inf), "100.0")
  expect_identical(parse_yaml(format_yaml(100)), 100)

  values <- c(100, -2, 1.5)
  expect_identical(parse_yaml(format_yaml(values)), values)
})

test_that("format_yaml accepts latin1 encoded strings", {
  latin1 <- rawToChar(as.raw(0xe9))
  Encoding(latin1) <- "latin1"

  expect_identical(parse_yaml(format_yaml(latin1)), "\u00e9")
})

test_that("format_yaml translates every character vector element", {
  latin1 <- rawToChar(as.raw(0xe9))
  Encoding(latin1) <- "latin1"

  expect_identical(
    parse_yaml(format_yaml(c(latin1, latin1))),
    c("\u00e9", "\u00e9")
  )
})

test_that("format_yaml honors latin1 marks on valid UTF-8 bytes", {
  latin1 <- rawToChar(as.raw(c(0xc3, 0xa9)))
  Encoding(latin1) <- "latin1"
  expected <- "\u00c3\u00a9"

  expect_identical(parse_yaml(format_yaml(latin1)), expected)
  expect_identical(
    parse_yaml(format_yaml(c(latin1, latin1))),
    c(expected, expected)
  )

  object <- setNames(list("value"), latin1)
  reparsed <- parse_yaml(format_yaml(object), simplify = FALSE)
  expect_identical(names(reparsed), expected)

  tag <- rawToChar(as.raw(c(0x21, 0xc3, 0xa9)))
  Encoding(tag) <- "latin1"
  tagged <- structure("value", yaml_tag = tag)
  expect_identical(format_yaml(tagged, width = Inf), "!\u00c3\u00a9 value")
})

test_that("format_yaml rejects malformed strings marked as UTF-8", {
  invalid <- rawToChar(as.raw(0xff))
  Encoding(invalid) <- "UTF-8"
  expect_identical(Encoding(invalid), "UTF-8")
  expect_false(validUTF8(invalid))

  expect_error(
    format_yaml(invalid),
    "R UTF-8 string contains invalid UTF-8",
    fixed = TRUE
  )
})

test_that("format_yaml rejects bytes-encoded strings", {
  bytes <- rawToChar(as.raw(0xff))
  Encoding(bytes) <- "bytes"

  expect_error(
    format_yaml(bytes),
    'translating strings with "bytes" encoding is not allowed',
    fixed = TRUE
  )

  tagged <- structure("value", yaml_tag = bytes)
  expect_error(
    format_yaml(tagged),
    'translating strings with "bytes" encoding is not allowed',
    fixed = TRUE
  )

  expect_identical(format_yaml("ok"), "ok")
})

test_that("format_yaml rejects bytes encoding even when bytes are valid UTF-8", {
  bytes <- rawToChar(as.raw(c(0xc3, 0xa9)))
  Encoding(bytes) <- "bytes"

  values <- list(
    bytes,
    c(bytes, bytes),
    setNames(list("value"), bytes),
    structure("value", yaml_tag = bytes)
  )

  for (value in values) {
    expect_error(
      format_yaml(value),
      'translating strings with "bytes" encoding is not allowed',
      fixed = TRUE
    )
  }
})

test_that("format_yaml quotes arbitrary-sized core integer strings", {
  values <- c(
    "0x8000000000000000",
    "0xFFFFFFFFFFFFFFFF",
    "0o1000000000000000000000"
  )

  for (value in values) {
    encoded <- format_yaml(value, width = Inf)

    expect_identical(encoded, paste0("\"", value, "\""), info = value)
    expect_identical(parse_yaml(encoded), value, info = value)
  }

  expect_identical(format_yaml("0xg", width = Inf), "0xg")
})

expect_scalar_serialization <- function(
  value,
  scalar_yaml,
  key_yaml = paste0(scalar_yaml, ": 1")
) {
  info <- encodeString(value, quote = "\"")

  encoded <- format_yaml(value, width = Inf)
  expect_identical(encoded, scalar_yaml, info = info)
  expect_identical(parse_yaml(encoded), value, info = info)

  object <- setNames(list(1L), value)
  encoded <- format_yaml(object, width = Inf)
  expect_identical(encoded, key_yaml, info = info)
  expect_identical(parse_yaml(encoded, simplify = FALSE), object, info = info)
}

expect_yaml_emission <- function(object, expected, width = 80, info = NULL) {
  encoded <- format_yaml(object, width = width)
  expect_identical(encoded, expected, info = info)
  expect_identical(
    parse_yaml(encoded, simplify = FALSE),
    object,
    info = info
  )
  invisible(encoded)
}

test_that("format_yaml emits YAML 1.2 plain strings through its public API", {
  values <- c(
    "yes",
    "No",
    "on",
    "OFF",
    "y",
    "n",
    "don't",
    "say \"hi\"",
    "a\\b",
    "a,b",
    "f[0]",
    "x{1}",
    "foo#bar",
    "foo:bar",
    "a ? b",
    "-x",
    "?x",
    ":x",
    "--x",
    ".gitignore",
    "=",
    "<y>",
    "0xg",
    "0o9",
    "1_000",
    "1.2.3",
    "e5",
    "inf",
    "nan",
    "a * b",
    "5% off",
    "a  b"
  )

  for (value in values) {
    expect_scalar_serialization(value, value)
  }
})

test_that("format_yaml quotes core-schema strings through its public API", {
  values <- c(
    "~",
    "null",
    "NULL",
    "true",
    "False",
    "12",
    "+7",
    "-3",
    "0x1F",
    "0o17",
    "3.5",
    "-2e10",
    ".5",
    ".inf",
    "-.Inf",
    ".NaN"
  )

  for (value in values) {
    expect_scalar_serialization(value, paste0("\"", value, "\""))
  }
})

test_that("format_yaml quotes structurally unsafe plain scalars", {
  values <- c(
    "",
    " x",
    "x ",
    "- x",
    "-",
    "?",
    ":",
    ": x",
    "foo: bar",
    "foo:",
    "a #b",
    "[x",
    "]x",
    ",x",
    "#x",
    "&x",
    "*x",
    "!x",
    "|x",
    ">x",
    "'x",
    "%x",
    "@x",
    "`x",
    "---",
    "--- x",
    "... x",
    "\ufeffx"
  )

  for (value in values) {
    expect_scalar_serialization(value, paste0("\"", value, "\""))
  }

  expect_scalar_serialization("a\tb", "\"a\\tb\"")
  expect_scalar_serialization("\"x", "\"\\\"x\"")
  expect_scalar_serialization("a\nb", "|-\n  a\n  b", "\"a\\nb\": 1")
})

test_that("format_yaml wraps long strings as folded block scalars", {
  long <- paste(rep("word", 30), collapse = " ")
  encoded <- format_yaml(list(key = long))
  expect_true(grepl("key: >-\n", encoded, fixed = TRUE))
  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 80))
  expect_identical(parse_yaml(encoded), list(key = long))
})

test_that("format_yaml chooses folded chomping from the value", {
  paragraph <- "alpha beta gamma delta epsilon"
  values <- c(strip = paragraph, clip = paste0(paragraph, "\n"))
  headers <- c(strip = ">-", clip = ">")

  for (value_name in names(values)) {
    expect_yaml_emission(
      list(body = values[[value_name]]),
      paste0(
        "body: ",
        headers[[value_name]],
        "\n",
        "  alpha beta gamma\n",
        "  delta epsilon"
      ),
      width = 20,
      info = value_name
    )
  }
})

test_that("format_yaml folds Markdown paragraph breaks losslessly", {
  paragraph <- "alpha beta gamma delta epsilon"
  body <- paste(paragraph, paragraph, sep = "\n\n")
  values <- c(strip = body, clip = paste0(body, "\n"))
  headers <- c(strip = ">-", clip = ">")

  for (value_name in names(values)) {
    encoded <- expect_yaml_emission(
      list(body = values[[value_name]]),
      paste0(
        "body: ",
        headers[[value_name]],
        "\n",
        "  alpha beta gamma\n",
        "  delta epsilon\n\n\n",
        "  alpha beta gamma\n",
        "  delta epsilon"
      ),
      width = 20,
      info = value_name
    )

    expect_true(
      grepl("delta epsilon\n\n\n  alpha", encoded, fixed = TRUE),
      info = value_name
    )
    expect_false(
      any(grepl("[[:blank:]]+$", strsplit(encoded, "\n")[[1]])),
      info = value_name
    )
  }
})

test_that("format_yaml folds paragraphs in every scalar context", {
  paragraph <- "alpha beta gamma delta epsilon"
  value <- paste(paragraph, paragraph, sep = "\n\n")
  cases <- list(
    root = list(
      object = value,
      expected = paste0(
        ">-\n",
        "  alpha beta gamma\n",
        "  delta epsilon\n\n\n",
        "  alpha beta gamma\n",
        "  delta epsilon"
      )
    ),
    sequence = list(
      object = list(value),
      expected = paste0(
        "- >-\n",
        "  alpha beta gamma\n",
        "  delta epsilon\n\n\n",
        "  alpha beta gamma\n",
        "  delta epsilon"
      )
    ),
    mapping = list(
      object = list(body = value),
      expected = paste0(
        "body: >-\n",
        "  alpha beta gamma\n",
        "  delta epsilon\n\n\n",
        "  alpha beta gamma\n",
        "  delta epsilon"
      )
    ),
    nested = list(
      object = list(outer = list(body = value)),
      expected = paste0(
        "outer:\n",
        "  body: >-\n",
        "    alpha beta gamma\n",
        "    delta epsilon\n\n\n",
        "    alpha beta gamma\n",
        "    delta epsilon"
      )
    )
  )

  for (context in names(cases)) {
    encoded <- expect_yaml_emission(
      cases[[context]]$object,
      cases[[context]]$expected,
      width = 20,
      info = context
    )
    expect_true(
      all(nchar(strsplit(encoded, "\n")[[1]]) <= 20),
      info = context
    )
  }
})

test_that("format_yaml keeps line-oriented multiline strings literal", {
  values <- c(
    nested_list = "- outer\n  - nested",
    blockquote = "> first quoted line\n> second quoted line",
    code = "```r\nprint('hello')\n```"
  )
  for (value_name in names(values)) {
    value <- values[[value_name]]
    expect_yaml_emission(
      list(body = value),
      paste0("body: |-\n  ", gsub("\n", "\n  ", value, fixed = TRUE)),
      width = 20,
      info = value_name
    )
  }

  paragraph <- "alpha beta gamma delta epsilon"
  value <- paste(paragraph, paragraph, sep = "\n")
  expect_yaml_emission(
    list(body = value),
    paste0(
      "body: |-\n",
      "  alpha beta gamma delta epsilon\n",
      "  alpha beta gamma delta epsilon"
    ),
    width = 20
  )
})

test_that("format_yaml emits empty literal lines without indentation", {
  encoded <- expect_yaml_emission(
    list(body = "alpha\n\nomega"),
    "body: |-\n  alpha\n\n  omega"
  )
  expect_false(any(grepl("[[:blank:]]+$", strsplit(encoded, "\n")[[1]])))

  expect_yaml_emission(
    list(body = "\n- outer\n  - nested"),
    "body: |-\n\n  - outer\n    - nested"
  )
})

test_that("format_yaml indents root literal content", {
  values <- c(
    document_start = "foo\n---\nbar",
    document_end = "foo\n...\nbar"
  )

  for (value_name in names(values)) {
    value <- values[[value_name]]
    expect_yaml_emission(
      value,
      paste0("|-\n  ", gsub("\n", "\n  ", value, fixed = TRUE)),
      width = Inf,
      info = value_name
    )
  }
})

test_that("format_yaml explicitly indents literal blocks with leading whitespace", {
  value <- "  indented\nnext"
  cases <- list(
    root = list(
      object = value,
      expected = "|2-\n    indented\n  next"
    ),
    sequence = list(
      object = list(value),
      expected = "- |2-\n    indented\n  next"
    ),
    mapping = list(
      object = list(body = value),
      expected = "body: |2-\n    indented\n  next"
    ),
    nested = list(
      object = list(outer = list(body = value)),
      expected = "outer:\n  body: |2-\n      indented\n    next"
    )
  )

  for (context in names(cases)) {
    expect_yaml_emission(
      cases[[context]]$object,
      cases[[context]]$expected,
      info = context
    )
  }

  values <- list(
    leading_tab = list(
      value = "\tindented\nnext",
      expected = "body: |2-\n  \tindented\n  next"
    ),
    leading_empty_line = list(
      value = "\n  indented\nnext",
      expected = "body: |2-\n\n    indented\n  next"
    ),
    clip = list(
      value = "  indented\nnext\n",
      expected = "body: |2\n    indented\n  next"
    )
  )

  for (case_name in names(values)) {
    expect_yaml_emission(
      list(body = values[[case_name]]$value),
      values[[case_name]]$expected,
      info = case_name
    )
  }
})

test_that("format_yaml retains conservative multiline fallbacks", {
  quoted <- list(
    trailing_newlines = c(
      value = "alpha\nbeta\n\n",
      expected = "body: \"alpha\\nbeta\\n\\n\""
    )
  )

  for (case_name in names(quoted)) {
    expect_yaml_emission(
      list(body = quoted[[case_name]][["value"]]),
      quoted[[case_name]][["expected"]],
      width = 20,
      info = case_name
    )
  }

  unsafe_gaps <- c(
    repeated_spaces = paste0(strrep("a", 25), "  ", strrep("b", 25)),
    tab = paste0(strrep("a", 25), "\t", strrep("b", 25))
  )
  for (case_name in names(unsafe_gaps)) {
    value <- paste(unsafe_gaps[[case_name]], "tail", sep = "\n\n")
    encoded <- format_yaml(list(body = value), width = 20)
    expect_false(grepl("body: >", encoded, fixed = TRUE), info = case_name)
    expect_identical(
      parse_yaml(encoded, simplify = FALSE),
      list(body = value),
      info = case_name
    )
  }
})

test_that("format_yaml keeps paragraph-shaped mapping keys inline", {
  paragraph <- "alpha beta gamma delta epsilon"
  key <- paste(paragraph, paragraph, sep = "\n\n")
  object <- setNames(list("payload"), key)
  encoded <- format_yaml(object, width = 20)

  expect_false(grepl("|", encoded, fixed = TRUE))
  expect_false(grepl(">", encoded, fixed = TRUE))
  expect_identical(parse_yaml(encoded, simplify = FALSE), object)
})

test_that("format_yaml leaves multiline wrapping disabled at non-finite widths", {
  paragraph <- "alpha beta gamma delta epsilon"
  value <- paste(paragraph, paragraph, sep = "\n\n")
  widths <- list(`Inf` = Inf, `-Inf` = -Inf, `NaN` = NaN)

  for (width_name in names(widths)) {
    expect_yaml_emission(
      list(body = value),
      paste0(
        "body: |-\n",
        "  alpha beta gamma delta epsilon\n\n",
        "  alpha beta gamma delta epsilon"
      ),
      width = widths[[width_name]],
      info = width_name
    )
  }
})

test_that("format_yaml wrapping round-trips nested structures", {
  long <- paste(rep("word", 30), collapse = " ")
  obj <- list(a = list(b = list(long, long)), c = long)
  encoded <- format_yaml(obj)
  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 80))
  expect_identical(parse_yaml(encoded, simplify = FALSE), obj)
})

test_that("format_yaml `width` argument controls wrapping", {
  long <- paste(rep("word", 30), collapse = " ")

  narrow <- format_yaml(list(key = long), width = 40)
  expect_true(all(nchar(strsplit(narrow, "\n")[[1]]) <= 40))
  expect_identical(parse_yaml(narrow), list(key = long))

  unwrapped <- format_yaml(list(key = long), width = Inf)
  expect_false(grepl(">-", unwrapped, fixed = TRUE))
  expect_identical(parse_yaml(unwrapped), list(key = long))
  expect_identical(format_yaml(list(key = long), width = NULL), unwrapped)
})

test_that("format_yaml wraps root strings within `width`", {
  value <- paste(rep("word", 30), collapse = " ")
  encoded <- format_yaml(value, width = 20)

  expect_true(startsWith(encoded, ">-\n"))
  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 20))
  expect_identical(parse_yaml(encoded), value)
})

test_that("format_yaml counts quoting when deciding to wrap", {
  values <- c(
    quotes = paste0("# ", strrep("a ", 38), "aa"),
    escapes = paste0("# ", strrep("a ", 37), "\\ aa")
  )

  for (value_name in names(values)) {
    value <- values[[value_name]]
    encoded <- format_yaml(value, width = 80)

    expect_true(startsWith(encoded, ">-\n"), info = value_name)
    expect_true(
      all(nchar(strsplit(encoded, "\n", fixed = TRUE)[[1]]) <= 80),
      info = value_name
    )
    expect_identical(parse_yaml(encoded), value, info = value_name)
  }
})

test_that("format_yaml respects narrow widths", {
  value <- paste(rep("aa", 12), collapse = " ")
  encoded <- format_yaml(list(key = value), width = 10)

  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 10))
  expect_identical(parse_yaml(encoded), list(key = value))
})

test_that("format_yaml includes inline prefixes when wrapping", {
  objects <- list(
    mapping = list(abcdefghijkl = "aa bb cc"),
    sequence = list("aa bb cc")
  )
  widths <- c(mapping = 20, sequence = 9)

  for (context in names(objects)) {
    encoded <- format_yaml(objects[[context]], width = widths[[context]])
    expect_true(
      all(nchar(strsplit(encoded, "\n")[[1]]) <= widths[[context]]),
      info = context
    )
    expect_identical(
      parse_yaml(encoded, simplify = FALSE),
      objects[[context]],
      info = context
    )
  }
})

test_that("write_yaml `width` argument controls wrapping", {
  long <- paste(rep("word", 30), collapse = " ")
  path <- withr::local_tempfile(fileext = ".yaml")

  write_yaml(list(key = long), path, width = 40)
  lines <- readLines(path)
  expect_true(all(nchar(lines) <= 40))
  expect_identical(read_yaml(path), list(key = long))

  write_yaml(list(key = long), path, FALSE, FALSE, Inf)
  unwrapped <- readLines(path)
  expect_false(any(grepl(">-", unwrapped, fixed = TRUE)))
  expect_identical(read_yaml(path), list(key = long))

  write_yaml(list(key = long), path, width = NULL)
  expect_identical(readLines(path), unwrapped)
})

test_that("YAML formatting defaults to an integer width", {
  expect_identical(formals(format_yaml)$width, 80L)
  expect_identical(formals(write_yaml)$width, 80L)
})

test_that("format_yaml validates `width`", {
  for (width in list(0, -1, NA_real_, "80", c(40, 80))) {
    expect_error(
      format_yaml(list(key = "value"), width = width),
      "`width` must be NULL, Inf, or a single number >= 1",
      fixed = TRUE
    )
  }
})

test_that("format_yaml leaves unbreakable long strings on one line", {
  long <- strrep("x", 120)
  encoded <- format_yaml(list(key = long))
  expect_false(grepl(">-", encoded, fixed = TRUE))
  expect_identical(parse_yaml(encoded), list(key = long))
})

test_that("format_yaml never wraps long mapping keys", {
  key <- paste(rep("word", 30), collapse = " ")
  obj <- setNames(list(1L), key)
  encoded <- format_yaml(obj)
  expect_false(grepl(">-", encoded, fixed = TRUE))
  expect_identical(parse_yaml(encoded), obj)
})

test_that("format_yaml quotes long strings with unsafe whitespace", {
  long <- paste0(" ", paste(rep("word", 25), collapse = " "))
  encoded <- format_yaml(list(key = long))
  expect_false(grepl(">-", encoded, fixed = TRUE))
  expect_identical(parse_yaml(encoded), list(key = long))
})

expect_yaml_roundtrip <- function(object, width = 80, info = NULL) {
  encoded <- tryCatch(format_yaml(object, width = width), error = identity)

  if (inherits(encoded, "error")) {
    fail(
      paste("YAML emission failed:", conditionMessage(encoded)),
      info = info
    )
    return(invisible())
  }

  emitted_info <- paste0(
    "emitted YAML: ",
    encodeString(encoded, quote = "\"")
  )
  info <- paste(c(info, emitted_info), collapse = "\n")
  actual <- tryCatch(parse_yaml(encoded, simplify = FALSE), error = identity)

  if (inherits(actual, "error")) {
    fail(
      paste("YAML round trip failed:", conditionMessage(actual)),
      info = info
    )
    return(invisible())
  }

  expect_identical(
    actual,
    object,
    info = info
  )

  invisible(encoded)
}

expect_scalar_yaml_roundtrip <- function(value, width = 80, info = NULL) {
  expect_yaml_roundtrip(list(value = value), width, info)
}

unicode_whitespace_code_points <- c(
  0x0085L,
  0x00a0L,
  0x1680L,
  0x2000L:0x200aL,
  0x2028L,
  0x2029L,
  0x202fL,
  0x205fL,
  0x3000L
)

test_that("format_yaml uses explicit syntax for overlong mapping keys", {
  keys <- c(
    plain = strrep("x", 1025),
    unicode = strrep("\u6f22", 1025),
    escaped = strrep("\n", 512)
  )

  for (key_name in names(keys)) {
    object <- setNames(list("payload"), keys[[key_name]])
    encoded <- expect_yaml_roundtrip(object, width = 20, info = key_name)

    expect_true(startsWith(encoded, "? "), info = key_name)
    expect_false(grepl("? |", encoded, fixed = TRUE), info = key_name)
    expect_false(grepl("? >", encoded, fixed = TRUE), info = key_name)
  }
})

test_that("format_yaml keeps mapping keys at the simple-key limit implicit", {
  keys <- c(
    plain = strrep("x", 1024),
    unicode = strrep("\u6f22", 1024),
    escaped = strrep("\n", 511)
  )

  for (key_name in names(keys)) {
    object <- setNames(list("payload"), keys[[key_name]])
    encoded <- expect_yaml_roundtrip(object, width = 20, info = key_name)

    expect_false(startsWith(encoded, "? "), info = key_name)
  }
})

test_that("format_yaml uses explicit syntax for tagged collection keys", {
  keys <- list(
    sequence = structure(list(1L, 2L), yaml_tag = "!generated"),
    mapping = structure(list(value = 1L), yaml_tag = "!generated")
  )

  for (key_name in names(keys)) {
    object <- structure(
      list("payload"),
      names = "",
      yaml_keys = list(keys[[key_name]])
    )
    encoded <- expect_yaml_roundtrip(object, width = 20, info = key_name)

    expect_true(startsWith(encoded, "? !generated"), info = key_name)
  }
})

test_that("format_yaml counts tags toward the simple-key limit", {
  key <- structure(
    "key",
    yaml_tag = paste0("!", strrep("x", 1024))
  )
  object <- structure(
    list("payload"),
    names = "",
    yaml_keys = list(key)
  )
  encoded <- expect_yaml_roundtrip(object, width = 20)

  expect_true(startsWith(encoded, "? !"))
})

test_that("format_yaml counts non-finite spellings in tagged keys", {
  key <- structure(
    Inf,
    yaml_tag = paste0("!", strrep("x", 1019))
  )
  object <- structure(
    list("payload"),
    names = "",
    yaml_keys = list(key)
  )

  encoded <- format_yaml(object, width = 20)
  expect_true(startsWith(encoded, "? !"))
})

lorem_words <- strsplit(
  paste(
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
  ),
  " ",
  fixed = TRUE
)[[1]]

lorem_with_separator <- function(position, separator) {
  paste0(
    paste(lorem_words[seq_len(position)], collapse = " "),
    separator,
    paste(lorem_words[(position + 1):length(lorem_words)], collapse = " ")
  )
}

test_that("format_yaml round-trips compound horizontal whitespace", {
  separators <- c(
    space = " ",
    two_spaces = "  ",
    four_spaces = "    ",
    tab = "\t",
    two_tabs = "\t\t",
    space_tab = " \t",
    tab_space = "\t ",
    space_tab_space = " \t "
  )

  for (position in seq_len(length(lorem_words) - 1)) {
    for (separator_name in names(separators)) {
      value <- lorem_with_separator(position, separators[[separator_name]])

      for (width in c(20, 40, 80, Inf)) {
        expect_scalar_yaml_roundtrip(
          value,
          width,
          info = sprintf(
            "separator %s at gap %d with width %s",
            separator_name,
            position,
            width
          )
        )
      }
    }
  }
})

test_that("format_yaml round-trips embedded line whitespace", {
  separators <- c(
    newline = "\n",
    two_newlines = "\n\n",
    three_newlines = "\n\n\n",
    space_newline = " \n",
    tab_newline = "\t\n",
    newline_space = "\n ",
    newline_two_spaces = "\n  ",
    newline_tab = "\n\t",
    spaced_blank_line = "\n \n",
    tabbed_blank_line = "\n\t\n",
    crlf = "\r\n",
    carriage_return = "\r",
    form_feed = "\f",
    vertical_tab = "\v"
  )

  for (position in seq_len(length(lorem_words) - 1)) {
    for (separator_name in names(separators)) {
      value <- lorem_with_separator(position, separators[[separator_name]])

      for (width in c(20, 80, Inf)) {
        expect_scalar_yaml_roundtrip(
          value,
          width,
          info = sprintf(
            "separator %s at gap %d with width %s",
            separator_name,
            position,
            width
          )
        )
      }
    }
  }
})

test_that("format_yaml round-trips leading and trailing whitespace", {
  edges <- c(
    none = "",
    space = " ",
    two_spaces = "  ",
    tab = "\t",
    space_tab = " \t",
    newline = "\n",
    two_newlines = "\n\n",
    three_newlines = "\n\n\n",
    space_newline = " \n",
    tab_newline = "\t\n",
    newline_space = "\n ",
    newline_tab = "\n\t",
    spaced_blank_line = "\n \n",
    tabbed_blank_line = "\n\t\n"
  )
  lorem <- paste(lorem_words, collapse = " ")

  for (prefix_name in names(edges)) {
    for (suffix_name in names(edges)) {
      value <- paste0(edges[[prefix_name]], lorem, edges[[suffix_name]])

      for (width in c(40, Inf)) {
        expect_scalar_yaml_roundtrip(
          value,
          width,
          info = sprintf(
            "prefix %s and suffix %s with width %s",
            prefix_name,
            suffix_name,
            width
          )
        )
      }
    }
  }
})

test_that("format_yaml round-trips whitespace-only strings", {
  values <- c(
    empty = "",
    space = " ",
    two_spaces = "  ",
    tab = "\t",
    two_tabs = "\t\t",
    mixed_horizontal = " \t ",
    newline = "\n",
    two_newlines = "\n\n",
    three_newlines = "\n\n\n",
    space_newline = " \n",
    tab_newline = "\t\n",
    newline_space = "\n ",
    newline_tab = "\n\t",
    spaced_blank_line = "\n \n",
    tabbed_blank_line = "\n\t\n",
    whitespace_lines = " \n  \n\t"
  )

  for (value_name in names(values)) {
    for (width in c(20, Inf)) {
      expect_scalar_yaml_roundtrip(
        values[[value_name]],
        width,
        info = sprintf("%s with width %s", value_name, width)
      )
    }
  }
})

test_that("format_yaml round-trips newline-only scalars before siblings", {
  objects <- list(
    sequence = list("\n", "after"),
    mapping = list(value = "\n", after = "after"),
    nested = list(outer = list(value = "\n", after = "after"))
  )

  for (context in names(objects)) {
    for (width in c(20, Inf)) {
      expect_yaml_roundtrip(
        objects[[context]],
        width,
        info = sprintf("newline-only %s at width %s", context, width)
      )
    }
  }
})

test_that("format_yaml round-trips Unicode whitespace-only strings", {
  for (code_point in unicode_whitespace_code_points) {
    value <- intToUtf8(code_point)
    objects <- list(
      root = value,
      sequence = list(value),
      mapping = list(value = value),
      nested = list(outer = list(value = value)),
      mapping_key = setNames(list("payload"), value)
    )

    for (context in names(objects)) {
      expect_yaml_roundtrip(
        objects[[context]],
        width = Inf,
        info = sprintf("U+%04X in %s", code_point, context)
      )
    }
  }
})

test_that("format_yaml round-trips compound whitespace patterns", {
  clauses <- c(
    "Lorem ipsum dolor sit amet",
    "consectetur adipiscing elit",
    "sed do eiusmod tempor incididunt ut labore et dolore magna aliqua"
  )
  separators <- c(
    space = " ",
    two_spaces = "  ",
    tab = "\t",
    space_tab_space = " \t ",
    newline = "\n",
    two_newlines = "\n\n",
    space_newline = " \n",
    newline_space = "\n ",
    spaced_blank_line = "\n \n"
  )

  for (first_name in names(separators)) {
    for (second_name in names(separators)) {
      value <- paste0(
        clauses[[1]],
        separators[[first_name]],
        clauses[[2]],
        separators[[second_name]],
        clauses[[3]]
      )

      expect_scalar_yaml_roundtrip(
        value,
        width = 40,
        info = sprintf("separators %s and %s", first_name, second_name)
      )
    }
  }
})

test_that("format_yaml round-trips whitespace in scalar contexts", {
  lorem <- paste(lorem_words, collapse = " ")
  values <- c(
    plain = lorem,
    repeated_spaces = sub(" ipsum ", "  ipsum   ", lorem, fixed = TRUE),
    embedded_tab = sub(" ipsum ", "\t ipsum\t", lorem, fixed = TRUE),
    embedded_newline = sub(" ipsum ", "\nipsum\n", lorem, fixed = TRUE),
    leading_whitespace = paste0(" \t", lorem),
    trailing_whitespace = paste0(lorem, " \t"),
    trailing_newline = paste0(lorem, "\n"),
    trailing_newlines = paste0(lorem, "\n\n")
  )

  for (value_name in names(values)) {
    value <- values[[value_name]]
    objects <- list(
      root = value,
      sequence = list(value),
      mapping = list(value = value),
      nested = list(outer = list(inner = value)),
      mapping_key = setNames(list("payload"), value)
    )

    for (context_name in names(objects)) {
      object <- objects[[context_name]]

      expect_yaml_roundtrip(
        object,
        width = 40,
        info = sprintf("%s in %s context", value_name, context_name)
      )
    }
  }
})

test_that("format_yaml fuzzes exact string round-trips", {
  fuzz_seed <- 20260731L
  withr::local_seed(fuzz_seed)

  widths <- c(1, 2, 5, 10, 20, 40, 80, Inf)
  contexts <- c(
    "root",
    "sequence",
    "mapping",
    "nested",
    "mapping_key",
    "tagged"
  )
  families <- c("paragraphs", "literal", "fallback", "arbitrary")
  paragraph_words <- c(
    "alpha",
    "beta",
    "gamma",
    "punctuation,",
    "colon:inside",
    "quote\"inside",
    "back\\slash",
    "caf\u00e9",
    "e\u0301",
    "\u6f22\u5b57",
    strrep("unbreakable", 12)
  )
  first_lines <- c(
    "first line",
    "- outer",
    "> blockquote",
    "1. ordered item",
    "```r"
  )
  later_lines <- c(
    "second line",
    "  - nested",
    "    indented code",
    "\tindented with a tab",
    "",
    "line with trailing space ",
    "line with trailing tab\t"
  )
  unicode_whitespace <- intToUtf8(
    unicode_whitespace_code_points,
    multiple = TRUE
  )
  fallback_values <- c(
    " leading space",
    "trailing space ",
    "\tleading tab",
    "trailing tab\t",
    "repeated  spaces",
    "embedded\ttab",
    "line\r\nbreak",
    "carriage\rreturn",
    "form\ffeed",
    "vertical\vtab",
    "\n",
    "\n\n",
    "value\n\n",
    "value\n\n\n",
    " \n  \n\t",
    unicode_whitespace,
    paste0(unicode_whitespace, "value"),
    paste0("value", unicode_whitespace)
  )
  arbitrary_tokens <- c(
    "",
    paragraph_words,
    "\U0001f642",
    "-",
    "?",
    ":",
    "#",
    "---",
    "...",
    "[value]",
    "{value}",
    "'single'",
    "\"double\"",
    "\u0001",
    "\u0007",
    "\u007f",
    unicode_whitespace
  )
  separators <- c(
    "",
    " ",
    "  ",
    "\t",
    "\n",
    "\n\n",
    "\n\n\n",
    " \n",
    "\n ",
    "\r\n",
    "\r",
    "\f",
    "\v",
    "\u00a0"
  )
  edges <- c("", " ", "\t", "\n", "\n\n", unicode_whitespace)

  for (case in seq_len(2000L)) {
    combination <- case - 1L
    family <- families[[combination %% length(families) + 1L]]
    context <- contexts[[
      (combination %/% length(families)) %% length(contexts) + 1L
    ]]
    width <- widths[[
      (combination %/% (length(families) * length(contexts))) %%
        length(widths) +
        1L
    ]]

    value <- switch(
      family,
      paragraphs = {
        paragraph_count <- sample.int(3L, 1L)
        paragraphs <- vapply(
          seq_len(paragraph_count),
          function(i) {
            paste(
              sample(
                paragraph_words,
                sample(18L:24L, 1L),
                replace = TRUE
              ),
              collapse = " "
            )
          },
          character(1)
        )
        paste0(
          paste(paragraphs, collapse = "\n\n"),
          if (sample(c(FALSE, TRUE), 1L)) "\n" else ""
        )
      },
      literal = {
        lines <- c(
          sample(first_lines, 1L),
          sample(later_lines, sample.int(6L, 1L), replace = TRUE)
        )
        paste0(
          sample(c("", "\n", "\n\n"), 1L),
          paste(lines, collapse = "\n"),
          if (sample(c(FALSE, TRUE), 1L)) "\n" else ""
        )
      },
      fallback = sample(fallback_values, 1L),
      arbitrary = {
        token_count <- sample.int(8L, 1L)
        tokens <- sample(arbitrary_tokens, token_count, replace = TRUE)
        value <- tokens[[1L]]
        if (token_count > 1L) {
          for (i in 2L:token_count) {
            value <- paste0(
              value,
              sample(separators, 1L),
              tokens[[i]]
            )
          }
        }
        paste0(sample(edges, 1L), value, sample(edges, 1L))
      }
    )

    object <- switch(
      context,
      root = value,
      sequence = list(value, "after"),
      mapping = list(value = value, after = "after"),
      nested = list(outer = list(value = value, after = "after")),
      mapping_key = setNames(list("payload"), value),
      tagged = structure(value, yaml_tag = "!generated")
    )
    info <- sprintf(
      "seed %d, case %d, family %s, context %s, width %s, value %s",
      fuzz_seed,
      case,
      family,
      context,
      width,
      encodeString(value, quote = "\"")
    )

    expect_yaml_roundtrip(object, width, info)
  }
})

test_that("format_yaml errors on duplicate names", {
  expect_error(
    format_yaml(list(a = 1, a = 2)),
    "Duplicate mapping key `a`"
  )

  dup_na <- list(1, 2)
  names(dup_na) <- c(NA, NA)
  expect_error(
    format_yaml(dup_na),
    "Duplicate mapping key `null`",
    fixed = TRUE
  )

  dup_empty <- list(1, 2)
  names(dup_empty) <- c("", "")
  expect_error(
    format_yaml(dup_empty),
    "Duplicate mapping key `(empty string)`",
    fixed = TRUE
  )
})

test_that("format_yaml round-trips mixed NA and empty names", {
  obj <- list(1L, 2L, 3L)
  names(obj) <- c("a", NA, "")

  encoded <- format_yaml(obj)
  reparsed <- parse_yaml(encoded, simplify = TRUE)

  expected <- structure(
    list(1L, 2L, 3L),
    names = c("a", "", ""),
    yaml_keys = list("a", NULL, "")
  )

  expect_identical(reparsed, expected)
})

test_that("format_yaml preserves yaml_tag attribute", {
  obj <- structure(
    list(
      scalar = structure("bar", yaml_tag = "!expr"),
      seq = structure(list(1L, 2L), yaml_tag = "!seq")
    ),
    yaml_tag = "!custom"
  )
  encoded <- format_yaml(obj)
  expect_true(grepl("!custom", encoded, fixed = TRUE))
  expect_true(grepl("!expr", encoded, fixed = TRUE))
  expect_true(grepl("!seq", encoded, fixed = TRUE))

  reparsed <- parse_yaml(encoded)
  expect_identical(attr(reparsed, "yaml_tag"), "!custom")
  expect_identical(attr(reparsed$scalar, "yaml_tag"), "!expr")
  expect_identical(attr(reparsed$seq, "yaml_tag"), "!seq")
})

test_that("format_yaml preserves yaml_tag attributes using core schema handle", {
  obj <- structure(
    list(
      seq = structure(list(1L, 2L), yaml_tag = "!!seq"),
      map = structure(list(foo = "bar"), yaml_tag = "!!map")
    ),
    yaml_tag = "!custom"
  )

  encoded <- format_yaml(obj)
  expect_true(grepl("!!seq", encoded, fixed = TRUE))
  expect_true(grepl("!!map", encoded, fixed = TRUE))
  expect_true(grepl("!custom", encoded, fixed = TRUE))

  reparsed <- parse_yaml(encoded, simplify = FALSE)
  expect_identical(attr(reparsed, "yaml_tag", exact = TRUE), "!custom")
  expect_null(attr(reparsed$seq, "yaml_tag", exact = TRUE))
  expect_null(attr(reparsed$map, "yaml_tag", exact = TRUE))
})

test_that("format_yaml keeps fully-qualified yaml_tag strings intact", {
  obj <- structure("bar", yaml_tag = "!<tag:yaml.org,2002:str>")

  encoded <- format_yaml(obj)
  expect_true(grepl("!<tag:yaml.org,2002:str>", encoded, fixed = TRUE))

  reparsed <- parse_yaml(encoded)
  expect_identical(reparsed, "bar")
  expect_null(attr(reparsed, "yaml_tag", exact = TRUE))
})

test_that("format_yaml round-trips multi-document streams", {
  docs <- list(list(foo = 1L), list(bar = list(2L, NULL)))
  encoded <- format_yaml(docs, multi = TRUE)
  expect_true(startsWith(encoded, "---"))
  expect_true(grepl("\n---\n", encoded, fixed = TRUE))
  expect_true(grepl("\n$", encoded))
  parsed <- parse_yaml(encoded, multi = TRUE)
  docs[[2]]$bar <- c(2L, NA)
  expect_identical(parsed, docs)
})

test_that("format_yaml with multi = TRUE rejects named lists", {
  docs <- list(a = list(foo = 1L), b = list(bar = 2L))
  expect_error(
    format_yaml(docs, multi = TRUE),
    "`value` must be an unnamed list when `multi = TRUE` (names must be NULL)",
    fixed = TRUE
  )
})

test_that("format_yaml single-document output has no header or trailing newline", {
  obj <- list(foo = 1L)
  encoded <- format_yaml(obj)
  expect_false(startsWith(encoded, "---"))
  expect_false(grepl("\n$", encoded))
  expect_identical(parse_yaml(encoded), obj)
})

test_that("format_yaml validates yaml_tag attribute shape", {
  tagged <- structure("value", yaml_tag = c("!a", "!b"))
  expect_error(
    format_yaml(tagged),
    "Invalid `yaml_tag` attribute: expected a single, non-missing string"
  )

  bad_type <- structure("value", yaml_tag = 1L)
  expect_error(
    format_yaml(bad_type),
    "Invalid `yaml_tag` attribute: expected a single, non-missing string"
  )
})

test_that("format_yaml errors clearly when multi = TRUE without a list", {
  expect_error(
    format_yaml(1L, multi = TRUE),
    "`value` must be a list when `multi = TRUE`",
    fixed = TRUE
  )
})

test_that("format_yaml preserves binary tags", {
  # b64::encode("hello world")
  tagged <- structure("aGVsbG8gd29ybGQ=", yaml_tag = "!!binary")
  out <- format_yaml(tagged)
  expect_true(startsWith(out, "!!binary "))
  expect_true(grepl("!!binary", out, fixed = TRUE))

  reparsed <- parse_yaml(out)
  expect_identical(as.character(reparsed), "aGVsbG8gd29ybGQ=")
  expect_identical(
    attr(reparsed, "yaml_tag", exact = TRUE),
    "tag:yaml.org,2002:binary"
  )
})

test_that("format_yaml respects yaml_keys attribute", {
  parsed <- parse_yaml(
    r"--(
1: a
true: b
null: c
3.5: d
)--"
  )

  encoded <- format_yaml(parsed)
  reparsed <- parse_yaml(encoded)
  expect_identical(reparsed, parsed)
})

test_that("format_yaml returns visibly", {
  obj <- list(answer = 42L)

  expect_visible(format_yaml(obj))
  out <- format_yaml(obj)
  expect_true(is.character(out) && length(out) == 1)
  expect_identical(parse_yaml(out), obj)
})

test_that("format_yaml preserves single-length collections", {
  seq_out <- format_yaml(list(list(1L)))
  reparsed_seq <- parse_yaml(seq_out)
  expect_identical(reparsed_seq, list(1L))

  map_out <- format_yaml(list(list(key = 1L)))
  reparsed_map <- parse_yaml(map_out)
  expect_identical(reparsed_map, list(list(key = 1L)))
})

test_that("format_yaml retains partial names as mapping keys", {
  obj <- list(a = 1L, 2L)
  encoded <- format_yaml(obj)
  reparsed <- parse_yaml(encoded)
  expect_named(reparsed, c("a", ""))
  expect_identical(reparsed[[1]], 1L)
  expect_identical(reparsed[[2]], 2L)
})

test_that("format_yaml converts NA names to null YAML keys", {
  obj <- list(a = 1L, b = 2L)
  names(obj)[2] <- NA_character_
  encoded <- format_yaml(obj)
  reparsed <- parse_yaml(encoded)

  expected <- structure(
    list(1L, 2L),
    names = c("a", ""),
    yaml_keys = list("a", NULL)
  )
  expect_identical(reparsed, expected)
})

test_that("format_yaml errors clearly on invalid yaml_tag", {
  missing <- structure("value", yaml_tag = NA_character_)
  expect_error(
    format_yaml(missing),
    "non-missing string.*Must not be NA"
  )

  malformed <- structure("value", yaml_tag = "!!")
  expect_error(
    format_yaml(malformed),
    "Invalid YAML tag `!!`",
    fixed = TRUE
  )
})

test_that("format_yaml round-trips bare local tag handle", {
  tagged <- structure(1, yaml_tag = "!")
  encoded <- format_yaml(tagged)
  expect_identical(encoded, "! 1.0")

  reparsed <- parse_yaml(encoded)
  expect_identical(reparsed, structure("1.0", yaml_tag = "!"))
})
