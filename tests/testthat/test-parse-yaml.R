test_that("parse_yaml handles scalars", {
  expect_identical(parse_yaml("null"), NULL)
  expect_identical(parse_yaml("123"), 123L)
  expect_identical(parse_yaml("true"), TRUE)
  expect_identical(parse_yaml("hello"), "hello")
})

test_that("parse_yaml ignores YAML comments", {
  yaml <- "
# whole-line comment
title: example # inline comment
items: [a, b] # trailing comment
"

  expect_identical(
    parse_yaml(yaml),
    list(title = "example", items = c("a", "b"))
  )
})

test_that("parse_yaml normalizes literal core-schema tags", {
  inputs <- c(
    "!!str true",
    "!<tag:yaml.org,2002:str> true",
    "%TAG ! tag:yaml.org,2002:\n---\n!str true",
    "%TAG !yaml! tag:yaml.org,2002:\n---\n!yaml!str true",
    "%TAG !! tag:yaml.org,2002:\n---\n!!str true"
  )

  parsed <- lapply(inputs, parse_yaml)
  expect_true(all(vapply(parsed, identical, logical(1), "true")))
  expect_true(all(vapply(
    parsed,
    function(x) is.null(attr(x, "yaml_tag", exact = TRUE)),
    logical(1)
  )))
})

test_that("parse_yaml handles simple sequences and mappings", {
  simple_seq <- r"--(
- a
- b
- c
)--"

  expect_identical(
    parse_yaml(simple_seq),
    c("a", "b", "c")
  )

  expect_identical(
    parse_yaml(simple_seq, simplify = FALSE),
    list("a", "b", "c")
  )

  expect_identical(
    parse_yaml(simple_seq, simplify = TRUE),
    c("a", "b", "c")
  )

  expect_identical(
    parse_yaml(
      r"--(
foo: 1
bar: baz
)--"
    ),
    list(foo = 1L, bar = "baz")
  )

  expect_identical(
    parse_yaml(c("foo: 1", "bar: 2")),
    list(foo = 1L, bar = 2L)
  )

  expect_error(parse_yaml(c("foo: 1", NA_character_)), "must not contain NA")
})

test_that("parse_yaml keeps sequences as lists when simplify = FALSE", {
  yaml <- "
- true
- 3
- null
"
  parsed <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(parsed, list(TRUE, 3L, NULL))
})

test_that("parse_yaml omits yaml_keys for plain string keys", {
  yaml <- "
alpha: 1
beta: true
"
  parsed <- parse_yaml(yaml, simplify = FALSE)
  expect_null(attr(parsed, "yaml_keys", exact = TRUE))
  expect_identical(names(parsed), c("alpha", "beta"))
  expect_identical(parsed$alpha, 1L)
  expect_identical(parsed$beta, TRUE)
})

test_that("parse_yaml preserves unsupported timestamp tags", {
  yaml <- "
- !!timestamp 2025-01-01
- !!timestamp 2025-01-01 21:59:43.10 -5
"
  parsed <- parse_yaml(yaml)
  expect_length(parsed, 2)

  expect_type(parsed[[1]], "character")
  expect_identical(as.character(parsed[[1]]), "2025-01-01")
  expect_identical(
    attr(parsed[[1]], "yaml_tag", exact = TRUE),
    "tag:yaml.org,2002:timestamp"
  )

  expect_type(parsed[[2]], "character")
  expect_identical(
    as.character(parsed[[2]]),
    "2025-01-01 21:59:43.10 -5"
  )
  expect_identical(
    attr(parsed[[2]], "yaml_tag", exact = TRUE),
    "tag:yaml.org,2002:timestamp"
  )
})

test_that("parse_yaml handles multiple documents when requested", {
  yaml <- r"--(
---
foo: 1
---
bar: 2
)--"
  expect_identical(parse_yaml(yaml), list(foo = 1L))
  expect_identical(
    parse_yaml(yaml, multi = TRUE),
    list(list(foo = 1L), list(bar = 2L))
  )
})

test_that("parse_yaml ignores errors in later documents when multi = FALSE", {
  yaml <- r"--(
---
foo: 1
...
---
not: [valid
)--"
  expect_identical(parse_yaml(yaml), list(foo = 1L))
  expect_error(parse_yaml(yaml, multi = TRUE), "YAML parse error")
})

test_that("parse_yaml errors on NA strings regardless of position or length", {
  expect_snapshot(error = TRUE, parse_yaml(NA_character_))
  expect_snapshot(error = TRUE, parse_yaml(c(NA_character_, "foo: 1")))
  expect_snapshot(error = TRUE, parse_yaml(c("foo: 1", NA_character_)))
  expect_snapshot(error = TRUE, parse_yaml(NA))
  expect_snapshot(error = TRUE, parse_yaml(NA_integer_))
  expect_snapshot(error = TRUE, parse_yaml(NA_real_))
  expect_snapshot(error = TRUE, parse_yaml(NA_complex_))
  expect_identical(parse_yaml(character()), NULL)
  expect_snapshot(
    error = TRUE,
    parse_yaml(c(NA_character_, NA_character_, "foo: 1"))
  )
  expect_snapshot(
    error = TRUE,
    parse_yaml(c("foo: 1", "bar: 2", NA_character_))
  )
})

test_that("parse_yaml accepts latin1 encoded input strings", {
  latin1 <- rawToChar(as.raw(0xe9))
  Encoding(latin1) <- "latin1"

  expect_identical(parse_yaml(latin1), "\u00e9")
})

test_that("parse_yaml honors latin1 marks on valid UTF-8 bytes", {
  latin1 <- rawToChar(as.raw(c(0xc3, 0xa9)))
  Encoding(latin1) <- "latin1"

  expect_identical(parse_yaml(latin1), "\u00c3\u00a9")

  latin1_line <- rawToChar(as.raw(c(0x2d, 0x20, 0xc3, 0xa9)))
  Encoding(latin1_line) <- "latin1"
  expect_identical(
    parse_yaml(c(latin1_line, latin1_line)),
    c("\u00c3\u00a9", "\u00c3\u00a9")
  )
})

test_that("parse_yaml rejects malformed strings marked as UTF-8", {
  invalid <- rawToChar(as.raw(0xff))
  Encoding(invalid) <- "UTF-8"
  expect_identical(Encoding(invalid), "UTF-8")
  expect_false(validUTF8(invalid))

  expect_error(
    parse_yaml(invalid),
    "R UTF-8 string contains invalid UTF-8",
    fixed = TRUE
  )
})

test_that("parse_yaml simplifies mixed numeric sequences", {
  yaml <- "[1, 2.5, 0x10, .inf, null]"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_type(simplified, "double")
  expect_identical(simplified, c(1, 2.5, 16, Inf, NA_real_))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(1L, 2.5, 16L, Inf, NULL))
})

test_that("parse_yaml simplifies signed infinities", {
  yaml <- "[-.inf, +.inf, .INF]"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_type(simplified, "double")
  expect_identical(simplified, c(-Inf, Inf, Inf))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(-Inf, Inf, Inf))
})

test_that("parse_yaml simplifies NaN values", {
  yaml <- "[.nan, .NaN]"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_type(simplified, "double")
  expect_identical(simplified, c(NaN, NaN))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(NaN, NaN))
})

test_that("parse_yaml promotes signed integers with floats", {
  yaml <- "[-1, +2, 3.0]"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_type(simplified, "double")
  expect_identical(simplified, c(-1, 2, 3))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(-1L, 2L, 3.0))
})

test_that("parse_yaml handles trailing newlines", {
  expect_identical(parse_yaml("foo: 1\n"), list(foo = 1L))
})

test_that("parse_yaml preserves YAML tags", {
  expect_identical(
    parse_yaml(r"--(!custom 3)--"),
    structure("3", yaml_tag = "!custom")
  )

  tagged <- parse_yaml(r"--(values: !seq [1, 2])--")
  expect_identical(tagged$values, structure(c(1L, 2L), yaml_tag = "!seq"))
})

test_that("parse_yaml preserves YAML tags under GC pressure", {
  expected <- structure("value", yaml_tag = "!custom")
  ok <- TRUE

  gctorture(TRUE)
  on.exit(gctorture(FALSE), add = TRUE)

  for (i in seq_len(50)) {
    ok <- identical(parse_yaml(r"--(!custom value)--"), expected)
    if (!ok) {
      break
    }
  }

  gctorture(FALSE)
  expect_true(ok)
})

test_that("parse_yaml preserves mixed unsimplified containers under GC pressure", {
  yaml <- r"--(
integer: 1
logical: true
nothing: null
text: value
sequence: [false, 2, 3.5, null, item]
nested:
  tagged: !custom tagged
  handled: !wrap handled
)--"
  handlers <- list(
    "!wrap" = function(value) {
      structure(list(value = value), class = "wrapped")
    }
  )
  expected <- list(
    integer = 1L,
    logical = TRUE,
    nothing = NULL,
    text = "value",
    sequence = list(FALSE, 2L, 3.5, NULL, "item"),
    nested = list(
      tagged = structure("tagged", yaml_tag = "!custom"),
      handled = structure(list(value = "handled"), class = "wrapped")
    )
  )

  gctorture(TRUE)
  on.exit(gctorture(FALSE), add = TRUE)

  parsed <- parse_yaml(yaml, simplify = FALSE, handlers = handlers)
  strings <- parse_yaml("[alpha, beta, null]")

  gctorture(FALSE)
  expect_identical(parsed, expected)
  expect_identical(strings, c("alpha", "beta", NA_character_))
})

test_that("parse_yaml applies handlers to tagged nodes", {
  handlers <- list(
    "!expr" = function(x) eval(str2lang(x), baseenv()),
    "!wrap" = function(x) list(value = x)
  )

  expect_identical(
    parse_yaml("foo: !expr 1+1", handlers = handlers),
    list(foo = 2)
  )

  expect_identical(
    parse_yaml("items: !wrap [a, b]", handlers = handlers),
    list(items = list(value = c("a", "b")))
  )
})

test_that("parse_yaml applies handlers to tagged mapping keys", {
  handlers <- list(
    "!upper" = function(x) toupper(x)
  )

  result <- parse_yaml("!upper key: value", handlers = handlers)
  expect_identical(result, list(KEY = "value"))
})

test_that("parse_yaml applies mapping key handlers once", {
  calls <- 0L
  handlers <- list(
    "!suffix" = function(x) {
      calls <<- calls + 1L
      paste0(x, "!")
    }
  )

  result <- parse_yaml("!suffix key: value", handlers = handlers)
  expect_identical(result, list("key!" = "value"))
  expect_identical(calls, 1L)
})

test_that("parse_yaml keeps handled string keys in yaml_keys when needed", {
  handlers <- list(
    "!upper" = function(x) toupper(x)
  )
  yaml <- r"--(
!upper key: value
1: one
)--"

  result <- parse_yaml(yaml, handlers = handlers)

  expected <- structure(
    list("value", "one"),
    names = c("KEY", ""),
    yaml_keys = list("KEY", 1L)
  )
  expect_identical(result, expected)
})

test_that("parse_yaml leaves names empty when key handler returns non-string", {
  handlers <- list(
    "!meta" = function(x) list(label = toupper(x))
  )

  result <- parse_yaml("!meta key: value", handlers = handlers)
  expect_identical(names(result), "")
  expect_identical(result[[1]], "value")
  expect_true(!is.null(attr(result, "yaml_keys")))
  expect_identical(attr(result, "yaml_keys")[[1]], list(label = "KEY"))
})

test_that("parse_yaml leaves names empty when key handler returns attributed string", {
  handlers <- list(
    "!decorated" = function(x) {
      structure(
        toupper(x),
        names = "ignored",
        class = "decorated"
      )
    }
  )

  result <- parse_yaml("!decorated key: value", handlers = handlers)
  expect_identical(names(result), "")
  expect_identical(result[[1]], "value")
  yaml_keys <- attr(result, "yaml_keys", exact = TRUE)
  expect_true(is.list(yaml_keys))
  expect_identical(attr(yaml_keys[[1]], "names", exact = TRUE), "ignored")
  expect_s3_class(yaml_keys[[1]], "decorated")
  expect_identical(as.character(yaml_keys[[1]]), "KEY")
})

test_that("parse_yaml applies handlers inside sequences before returning", {
  handlers <- list(
    "!double" = function(x) as.integer(x) * 2L
  )

  result <- parse_yaml("items: [!double 2, 3, !double 5]", handlers = handlers)
  expect_identical(result, list(items = list(4L, 3L, 10L)))
})

test_that("parse_yaml handler errors propagate", {
  expect_error(
    parse_yaml(
      "foo: !boom bar",
      handlers = list("!boom" = function(x) stop("boom"))
    ),
    "boom"
  )
})

test_that("parse_yaml handles large handler sets (hash map backend)", {
  tags <- sprintf("!h%d", seq_len(10))
  called <- new.env(parent = emptyenv())
  handlers <- stats::setNames(
    lapply(tags, function(tag) {
      force(tag)
      function(x) {
        called[[tag]] <<- x
        paste0(tag, ":", x)
      }
    }),
    tags
  )

  result <- parse_yaml("value: !h9 bar", handlers = handlers)
  expect_identical(result, list(value = "!h9:bar"))
  expect_identical(as.list(called, all.names = TRUE), list("!h9" = "bar"))
})

test_that("parse_yaml errors on duplicate handler names", {
  dup_handlers <- list("!dup" = identity, "!dup" = as.integer)
  expect_error(
    parse_yaml("value: !dup 1", handlers = dup_handlers),
    "Duplicate handler `!dup`"
  )
})

test_that("parse_yaml validates handlers argument", {
  expect_error(parse_yaml("foo: !expr 1", handlers = 12), "named list")
  expect_error(
    parse_yaml("foo: !expr 1", handlers = list("!expr" = "not a function")),
    "must be a function"
  )
})

test_that("parse_yaml leaves names empty for tagged string keys without handlers", {
  yaml <- "!tagged foo: 1\n"

  parsed <- parse_yaml(yaml, simplify = TRUE)
  expect_identical(
    parsed,
    structure(
      list(1L),
      names = "",
      yaml_keys = list(structure("foo", yaml_tag = "!tagged"))
    )
  )
  expect_identical(names(parsed), "")
  yaml_keys <- attr(parsed, "yaml_keys", exact = TRUE)
  expect_true(is.list(yaml_keys))
  expect_identical(attr(yaml_keys[[1]], "yaml_tag", exact = TRUE), "!tagged")
  expect_identical(as.character(yaml_keys[[1]]), "foo")
})

test_that("parse_yaml() warnings are catchable and respect options(warn)", {
  expect_no_warning(parse_yaml("!custom null"))
  expect_identical(
    parse_yaml("!custom null"),
    structure("null", yaml_tag = "!custom")
  )
  expect_no_warning(suppressWarnings(parse_yaml("!custom null")))
  expect_no_error(withr::with_options(
    list(warn = 2L),
    parse_yaml("!custom null")
  ))
  expect_no_warning(parse_yaml("!!null null"))
  expect_no_warning(parse_yaml("!<tag:yaml.org,2002:null> null"))
})

test_that("parse_yaml resolves all canonical null tag spellings", {
  canonical_cases <- c(
    "!!null ~",
    "!<tag:yaml.org,2002:null> ~"
  )
  for (yaml in canonical_cases) {
    parsed <- parse_yaml(yaml, simplify = FALSE)
    expect_identical(parsed, NULL)
    expect_null(attr(parsed, "yaml_tag", exact = TRUE))
  }

  informative_cases <- list(
    "!<!!null> ~" = "!!null",
    "!<!null> ~" = "!null",
    "!null ~" = "!null"
  )
  for (yaml in names(informative_cases)) {
    parsed <- parse_yaml(yaml, simplify = FALSE)
    expect_identical(
      parsed,
      structure("~", yaml_tag = informative_cases[[yaml]])
    )
  }
})

test_that("parse_yaml errors clearly on invalid canonical tags", {
  expect_snapshot(error = TRUE, parse_yaml("!!int foo"))
  expect_snapshot(error = TRUE, parse_yaml("!!null foo"))
})

test_that("parse_yaml preserves unknown core tags", {
  collection <- parse_yaml(
    "!!python/object:__main__.DangerousPayload {payload: true}"
  )
  expect_identical(
    collection,
    structure(
      list(payload = TRUE),
      yaml_tag = "tag:yaml.org,2002:python/object:__main__.DangerousPayload"
    )
  )
  collection_yaml <- format_yaml(collection)
  expect_true(startsWith(
    collection_yaml,
    "!!python/object:__main__.DangerousPayload"
  ))
  expect_identical(parse_yaml(collection_yaml), collection)

  scalar <- parse_yaml('!!unknown "true"')
  expect_identical(
    scalar,
    structure("true", yaml_tag = "tag:yaml.org,2002:unknown")
  )
  scalar_yaml <- format_yaml(scalar)
  expect_true(startsWith(scalar_yaml, "!!unknown"))
  expect_identical(parse_yaml(scalar_yaml), scalar)
})

test_that("parse_yaml renders non-string mapping keys", {
  yaml <- r"--(
1: a
true: b
null: c
3.5: d
)--"
  result <- parse_yaml(yaml)

  expected <- structure(
    list("a", "b", "c", "d"),
    names = c("", "", "", ""),
    yaml_keys = list(1L, TRUE, NULL, 3.5)
  )

  expect_identical(result, expected)
})

test_that("parse_yaml stores non-string mapping keys in yaml_key attribute", {
  yaml <- r"--(
1: a
true: b
3.5: c
string: d
)--"
  result <- parse_yaml(yaml)

  expected <- structure(
    list("a", "b", "c", "d"),
    names = c("", "", "", "string"),
    yaml_keys = list(1L, TRUE, 3.5, "string")
  )

  expect_identical(result, expected)
})

test_that("parse_yaml mapping key tags respect simplify flag", {
  yaml <- "!<tag:yaml.org,2002:str> foo: 1\n"

  expected <- list(foo = 1L)

  expect_identical(parse_yaml(yaml, simplify = TRUE), expected)
  expect_identical(parse_yaml(yaml, simplify = FALSE), expected)

  expect_snapshot(str(parse_yaml(
    "!<tag:yaml.org,2002:str> foo: 1\n",
    simplify = TRUE
  )))
  expect_snapshot(str(parse_yaml(
    "!<tag:yaml.org,2002:str> foo: 1\n",
    simplify = FALSE
  )))
})

test_that("parse_yaml omits yaml_keys for core string tagged mapping keys", {
  yaml <- "!!str foo: 1\n"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_identical(simplified, list(foo = 1L))
  expect_null(attr(simplified, "yaml_keys", exact = TRUE))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(foo = 1L))
  expect_null(attr(unsimplified, "yaml_keys", exact = TRUE))
})

test_that("parse_yaml preserves non-core tags on mapping keys via yaml_keys", {
  yaml <- "!custom foo: 1\n"

  expected <- structure(
    list(1L),
    names = "",
    yaml_keys = list(structure("foo", yaml_tag = "!custom"))
  )

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_identical(simplified, expected)

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, expected)

  encoded <- format_yaml(unsimplified)
  roundtrip <- parse_yaml(encoded, simplify = FALSE)
  expect_identical(roundtrip, expected)

  expect_snapshot(str(parse_yaml("!custom foo: 1\n", simplify = TRUE)))
  expect_snapshot(str(parse_yaml("!custom foo: 1\n", simplify = FALSE)))
})

test_that("parse_yaml round-trips verbatim tags on mapping keys", {
  parsed <- parse_yaml("!<foo> key: value", simplify = FALSE)
  expected <- structure(
    list("value"),
    names = "",
    yaml_keys = list(structure("key", yaml_tag = "foo"))
  )

  expect_identical(parsed, expected)
  expect_identical(parse_yaml(format_yaml(parsed), simplify = FALSE), parsed)
})

test_that("parse_yaml does not set yaml_keys when all mapping keys are strings", {
  yaml <- r"--(
foo: 1
bar: 2
)--"

  result <- parse_yaml(yaml)
  expect_null(attr(result, "yaml_keys", exact = TRUE))
  expect_named(result, c("foo", "bar"))
})

test_that("parse_yaml yaml_keys align with positions when names are empty", {
  yaml <- r"--(
1: a
2: b
)--"
  result <- parse_yaml(yaml)

  expected <- structure(
    list("a", "b"),
    names = c("", ""),
    yaml_keys = list(1L, 2L)
  )

  expect_identical(result, expected)
})

test_that("parse_yaml returns visibly", {
  expect_visible(parse_yaml("answer: 42"))
  expect_identical(parse_yaml("answer: 42"), list(answer = 42L))
})

test_that("parse_yaml keeps sequences/mappings of length 1 as collections", {
  single_seq <- parse_yaml("- 1")
  expect_type(single_seq, "integer")
  expect_identical(single_seq, 1L)

  single_map <- parse_yaml("key: 1")
  expect_type(single_map, "list")
  expect_identical(single_map, list(key = 1L))
})

test_that("roundtrip newline in short string scalar", {
  og <- list(foo = "bar!\nbar!", baz = 42L)
  rt <- parse_yaml(format_yaml(og))
  expect_identical(og, rt)
})

test_that("parse_yaml resolves simple anchors and aliases", {
  yaml <- "a1: &DEFAULT\n  b1: 4\na2: *DEFAULT\n"

  parsed <- parse_yaml(yaml, simplify = FALSE)

  expect_identical(parsed$a1$b1, 4L)
  expect_identical(parsed$a2$b1, 4L)
})

test_that("parse_yaml resolves anchors and aliases inside sequences", {
  yaml <- r"--(
- &A 1
- 2
- *A
)--"

  simplified <- parse_yaml(yaml, simplify = TRUE)
  expect_identical(simplified, c(1L, 2L, 1L))

  unsimplified <- parse_yaml(yaml, simplify = FALSE)
  expect_identical(unsimplified, list(1L, 2L, 1L))
})
