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

test_that("format_yaml wraps long strings as folded block scalars", {
  long <- paste(rep("word", 30), collapse = " ")
  encoded <- format_yaml(list(key = long))
  expect_true(grepl("key: >-\n", encoded, fixed = TRUE))
  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 80))
  expect_identical(parse_yaml(encoded), list(key = long))
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
})

test_that("format_yaml wraps root strings within `width`", {
  value <- paste(rep("word", 30), collapse = " ")
  encoded <- format_yaml(value, width = 20)

  expect_true(startsWith(encoded, ">-\n"))
  expect_true(all(nchar(strsplit(encoded, "\n")[[1]]) <= 20))
  expect_identical(parse_yaml(encoded), value)
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

  write_yaml(list(key = long), path, width = Inf)
  expect_false(any(grepl(">-", readLines(path), fixed = TRUE)))
  expect_identical(read_yaml(path), list(key = long))
})

test_that("format_yaml validates `width`", {
  for (width in list(0, -1, -Inf, NA, NaN, "80", c(40, 80))) {
    expect_error(
      format_yaml(list(key = "value"), width = width),
      "`width` must be a single number >= 1, or Inf",
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
  actual <- tryCatch(
    parse_yaml(format_yaml(object, width = width), simplify = FALSE),
    error = identity
  )

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
}

expect_scalar_yaml_roundtrip <- function(value, width = 80, info = NULL) {
  expect_yaml_roundtrip(list(value = value), width, info)
}

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
  expect_identical(encoded, "! 1")

  reparsed <- parse_yaml(encoded)
  expect_identical(reparsed, structure("1", yaml_tag = "!"))
})

if (FALSE) {
  test_that("format_yaml tags Date and POSIXct objects as timestamps", {
    posix_val <- as.POSIXct("2024-01-02 03:04:05", tz = "UTC")
    posix_yaml <- format_yaml(posix_val)
    expect_true(grepl("timestamp", posix_yaml, fixed = TRUE))
    parsed_posix <- parse_yaml(posix_yaml)
    expect_s3_class(parsed_posix, "POSIXct")
    expect_identical(attr(parsed_posix, "tzone"), "UTC")
    expect_equal(as.numeric(parsed_posix), as.numeric(posix_val))

    date_val <- as.Date("2024-01-02")
    date_yaml <- format_yaml(date_val)
    expect_true(grepl("timestamp", date_yaml, fixed = TRUE))
    parsed_date <- parse_yaml(date_yaml)
    expect_s3_class(parsed_date, "Date")
    expect_identical(parsed_date, date_val)
  })

  test_that("POSIXct values round-trip with format_yaml/parse_yaml", {
    utc_time <- as.POSIXct("2024-02-03 01:02:03", tz = "UTC")
    expect_identical(utc_time, parse_yaml(format_yaml(utc_time)))

    local_time <- as.POSIXct("2024-02-03 01:02:03", tz = "")
    round_tripped <- parse_yaml(format_yaml(local_time))
    expect_s3_class(round_tripped, "POSIXct")
    expect_identical(as.numeric(round_tripped), as.numeric(local_time))
    expect_null(attr(round_tripped, "tzone", exact = TRUE))

    now_time <- Sys.time()
    expect_identical(now_time, parse_yaml(format_yaml(now_time)))

    naive_time <- structure(1763834102.3786, class = c("POSIXct", "POSIXt"))
    expect_identical(naive_time, parse_yaml(format_yaml(naive_time)))

    offset_time <- as.POSIXct("2024-02-03 01:02:03", tz = "Etc/GMT-3")
    offset_round_tripped <- parse_yaml(format_yaml(offset_time))
    expect_s3_class(offset_round_tripped, "POSIXct")
    expect_identical(as.numeric(offset_round_tripped), as.numeric(offset_time))
    expect_identical(
      attr(offset_round_tripped, "tzone", exact = TRUE),
      "Etc/GMT-3"
    )
  })

  test_that("Date sequences round-trip with format_yaml/parse_yaml", {
    dates <- seq.Date(as.Date("1000-01-01"), as.Date("3000-01-01"), by = "day")
    expect_equal(parse_yaml(format_yaml(dates)), dates)
  })

  test_that("POSIXct round-trips with format_yaml/parse_yaml", {
    x <- .POSIXct(runif(10000, ISOdate(1000, 1, 1), ISOdate(3000, 1, 1)))
    expect_equal(x, parse_yaml(format_yaml(x)))

    x <- .POSIXct(runif(10000, ISOdate(1900, 1, 1), ISOdate(2100, 1, 1)))
    expect_equal(x, parse_yaml(format_yaml(x)))

    x <- .POSIXct(runif(10000, ISOdate(1960, 1, 1), ISOdate(2025, 1, 1)))
    expect_equal(x, parse_yaml(format_yaml(x)))
  })
}
