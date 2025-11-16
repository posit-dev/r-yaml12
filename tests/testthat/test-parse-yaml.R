test_that("parse_yaml handles scalars", {
  expect_identical(parse_yaml("null"), NULL)
  expect_identical(parse_yaml("123"), 123L)
  expect_identical(parse_yaml("true"), TRUE)
  expect_identical(parse_yaml("hello"), "hello")
})

test_that("parse_yaml handles simple sequences and mappings", {
  expect_identical(
    parse_yaml(
      r"--(
- a
- b
- c
)--"
    ),
    list("a", "b", "c")
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
  expect_identical(
    parse_yaml(r"--(!custom 3)--"),
    structure(3L, yaml_tag = "!custom")
  )

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

  expect_named(result, c("", "", "", ""))
  expect_identical(result[[1]], "a")
  expect_identical(result[[2]], "b")
  expect_identical(result[[3]], "c")
  expect_identical(result[[4]], "d")
  yaml_keys <- attr(result, "yaml_keys", exact = TRUE)
  expect_null(names(yaml_keys))
  expect_identical(yaml_keys[[1]], 1L)
  expect_identical(yaml_keys[[2]], TRUE)
  expect_identical(yaml_keys[[3]], NULL)
  expect_identical(yaml_keys[[4]], 3.5)
})

test_that("parse_yaml stores non-string mapping keys in yaml_key attribute", {
  yaml <- r"--(
1: a
true: b
3.5: c
string: d
)--"
  result <- parse_yaml(yaml)

  expect_named(result, c("", "", "", "string"))
  yaml_keys <- attr(result, "yaml_keys", exact = TRUE)
  expect_null(names(yaml_keys))
  expect_identical(yaml_keys[[1]], 1L)
  expect_identical(yaml_keys[[2]], TRUE)
  expect_identical(yaml_keys[[3]], 3.5)
  expect_identical(yaml_keys[[4]], "string")
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

  expect_named(result, c("", ""))
  expect_identical(result[[1]], "a")
  expect_identical(result[[2]], "b")

  yaml_keys <- attr(result, "yaml_keys", exact = TRUE)
  expect_null(names(yaml_keys))
  expect_identical(yaml_keys[[1]], 1L)
  expect_identical(yaml_keys[[2]], 2L)
})
