test_that("parse_yaml handles scalars", {
  expect_identical(parse_yaml("null"), NULL)
  expect_identical(parse_yaml("123"), 123L)
  expect_identical(parse_yaml("true"), TRUE)
  expect_identical(parse_yaml("hello"), "hello")
})

test_that("parse_yaml handles simple sequences and mappings", {
  expect_identical(
    parse_yaml(r"--(
- a
- b
- c
)--"),
    list("a", "b", "c")
  )

  expect_identical(
    parse_yaml(r"--(
foo: 1
bar: baz
)--"),
    list(foo = 1L, bar = "baz")
  )

  expect_identical(
    parse_yaml(c("foo: 1", "bar: 2")),
    list(foo = 1L, bar = 2L)
  )

  expect_error(parse_yaml(c("foo: 1", NA_character_)), "must not contain NA")
})

test_that("parse_yaml ignores additional documents", {
  yaml <- r"--(
---
foo: 1
---
bar: 2
)--"
  expect_identical(parse_yaml(yaml), list(foo = 1L))
})

test_that("parse_yaml handles trailing newlines", {
  expect_identical(parse_yaml("foo: 1\n"), list(foo = 1L))
})

test_that("parse_yaml preserves YAML tags", {
  expect_identical(parse_yaml(r"--(!custom 3)--"), structure(3L, yaml_tag = "!custom"))

  tagged <- parse_yaml(r"--(values: !seq [1, 2])--")
  expect_identical(tagged$values, structure(list(1L, 2L), yaml_tag = "!seq"))
})

test_that("parse_yaml renders non-string mapping keys", {
  yaml <- r"--(
1: a
true: b
null: c
3.5: d
)--"
  result <- parse_yaml(yaml)

  expect_named(result, c("1", "true", "null", "3.5"))
  expect_identical(result$`1`, "a")
  expect_identical(result$`true`, "b")
  expect_identical(result$`null`, "c")
  expect_identical(result$`3.5`, "d")
})
