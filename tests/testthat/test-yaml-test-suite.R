
test_cases <- dirname(list.files(
  test_path("yaml-test-suite/data"),
  recursive = TRUE,
  pattern = "in.yaml$",
  full.names = TRUE
))

for (case in test_cases) {
  case_id <- basename(case)
  title_path <- file.path(case, "===")
  case_title <- case_id
  if (file.exists(title_path)) {
    title_text <- trimws(paste(
      readLines(title_path, warn = FALSE),
      collapse = " "
    ))
    if (nzchar(title_text)) {
      case_title <- paste(case_id, title_text, sep = ": ")
    }
  }

  test_that(title_text, {

    if (file.exists(file.path(case, "error"))) {
      expect_error(read_yaml(file.path(case, "in.yaml"), multi = TRUE))
      return()
    }

    parsed <- expect_no_error(read_yaml(
      file.path(case, "in.yaml"),
      multi = TRUE,
      simplify = FALSE
    ))

    if (file.exists(file.path(case, "in.json"))) {
      expected <- tryCatch(
        list(jsonlite::read_json(
          file.path(case, "in.json"),
          simplifyVector = FALSE
        )),
        error = function(e) {
          docs <- list()
          lines <- character()
          con <- file(file.path(case, "in.json"), open = "r")
          on.exit(close(con))
          while (length(next_line <- readLines(con, n = 1))) {
            lines <- c(lines, next_line)
            tryCatch(
              {
                doc <- jsonlite::parse_json(
                  lines,
                  simplifyVector = FALSE
                )
                docs[length(docs) + 1L] <- list(doc)
                lines <- character()
              },
              error = function(e) NULL
            )
          }
          docs
        }
      )

      # TODO: some of these don't make a whole lot of sense...
      # attr(,"yaml_tag")
      # [1] "!!"
      parsed <- zap_yaml_tags(parsed)

      expect_identical(parsed, expected)

      # if (!identical(parsed, expected)) {
      #   # message("failing case: ", case)
      #   # withr::with_dir(case, {
      #   #   print(list.files())
      #   #   print(readLines("in.yaml"))
      #   #   print(readLines("in.json"))
      #   # })
      #   fail(paste("case fails:", case))
      # }
    }
  })
}
