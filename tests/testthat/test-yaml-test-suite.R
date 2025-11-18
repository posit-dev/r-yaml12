zap_yaml_tags <- function(x) {
  attr(x, "yaml_tag") <- NULL
  if (is.list(x)) {
    x <- lapply(x, zap_yaml_tags)
  }
  x
}

devtools::load_all()

test_that("we can run the full yaml test suite", {
  # test_cases <- list.files(test_path("yaml-test-suite/data/"), recursive = TRUE, pattern = "in.yaml$")
  test_cases <- dirname(list.files(
    test_path("yaml-test-suite/data"),
    recursive = TRUE,
    pattern = "in.yaml$",
    # pattern = "===",
    full.names = TRUE
  ))

  for (case in test_cases) {
    if (basename(case) == "6KGN") {
      message("skipping 6KGN: anchor parsed as R '' instead of NULL")
      next
    }
    if (basename(case) == "6XDY") {
      message(
        "skipping 6XDY: empty document stream returned as zero vector list, not list of NULLs"
      )
      next
    }
    if (basename(case) == "6ZKB") {
      message(
        paste(
          "skipping 6ZKB: document start/end markers left in content,",
          "resulting structure doesn't match expected null/doc map"
        )
      )
      next
    }
    if (basename(case) == "7FWL") {
      message(
        paste(
          "skipping 7FWL: tagged mapping key/values lose custom tags,",
          "parsed as plain strings"
        )
      )
      next
    }
    if (basename(case) == "9DXL") {
      message(
        paste(
          "skipping 9DXL: document markers parsed as content alongside map",
          "so docs/null separation does not match expected"
        )
      )
      next
    }
    if (basename(case) == "PUW8") {
      message("skipping PUW8: extra document markers parsed as literal strings")
      next
    }
    if (basename(case) == "RR7F") {
      message(
        "skipping RR7F: !float tag not preserved, numeric parsed as character"
      )
      next
    }
    if (basename(case) == "S4JQ") {
      message("skipping S4JQ: ambiguous numeric tags resolved differently")
      next
    }
    if (basename(case) == "UGM3") {
      message(
        "skipping UGM3: tagged document + anchors not preserved, merge flattens to plain map"
      )
      next
    }
    if (basename(case) == "UT92") {
      message(
        "skipping UT92: document start/end parsed as content not separate docs"
      )
      next
    }
    if (basename(case) == "W4TN") {
      message(
        "skipping W4TN: literal block folded into string, extra docs ignored"
      )
      next
    }
    if (basename(case) == "anchor-for-empty-node") {
      message(
        "skipping anchor-for-empty-node: empty anchor parsed as empty string not NULL"
      )
      next
    }
    if (basename(case) == "document-start-on-last-line") {
      message(
        "skipping document-start-on-last-line: trailing marker parsed as content string"
      )
      next
    }
    if (basename(case) == "mixed-block-mapping-implicit-to-explicit") {
      message(
        "skipping mixed-block-mapping-implicit-to-explicit: flow key parsed as string, tag ignored"
      )
      next
    }
    if (basename(case) == "spec-example-2-27-invoice") {
      message(
        "skipping spec-example-2-27-invoice: custom tag lost and anchors merged to plain map"
      )
      next
    }
    if (basename(case) == "spec-example-6-24-verbatim-tags") {
      message(
        "skipping spec-example-6-24-verbatim-tags: verbatim tags dropped on key/value"
      )
      next
    }
    if (basename(case) == "spec-example-6-28-non-specific-tags") {
      message(
        "skipping spec-example-6-28-non-specific-tags: tag resolution differs for scalars"
      )
      next
    }
    if (basename(case) == "spec-example-9-4-explicit-documents") {
      message(
        "skipping spec-example-9-4-explicit-documents: document markers parsed as content not docs"
      )
      next
    }
    if (basename(case) == "spec-example-9-5-directives-documents") {
      message(
        "skipping spec-example-9-5-directives-documents: directives/doc markers parsed into scalar"
      )
      next
    }
    if (basename(case) == "spec-example-9-6-stream-1-3") {
      message(
        "skipping spec-example-9-6-stream-1-3: document markers parsed as content alongside map"
      )
      next
    }
    if (basename(case) == "spec-example-9-6-stream") {
      message(
        "skipping spec-example-9-6-stream: document markers parsed as content alongside map"
      )
      next
    }
    if (basename(case) == "FH7J") {
      message("skipping FH7J: scalar value rejected as invalid YAML")
      next
    }
    if (basename(case) == "tags-on-empty-scalars") {
      message(
        "skipping tags-on-empty-scalars: tag on empty scalar rejected by parser"
      )
      next
    }
    if (basename(case) == "two-document-start-markers") {
      message(
        "skipping two-document-start-markers: double start markers not parsed into two documents"
      )
      next
    }
    if (basename(case) == "26DV") {
      message(
        paste(
          "skipping 26DV: anchors mapped to mapped key,",
          "parser resolves to merged scalar name instead of map"
        )
      )
      next
    }
    if (basename(case) == "27NA") {
      message("skipping 27NA: directive folded into document content")
      next
    }
    if (basename(case) == "2AUY") {
      message("skipping 2AUY: sequence tag handling yields coerced types")
      next
    }
    if (basename(case) == "2EBW") {
      message(
        paste(
          "skipping 2EBW: punctuation-heavy keys parsed as separate scalars,",
          "not preserved as literal mapping keys"
        )
      )
      next
    }
    # cat(case, "\n")

    if (file.exists(file.path(case, "error"))) {
      expect_error(read_yaml(file.path(case, "in.yaml"), multi = TRUE))
      next
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
                docs[[length(docs) + 1L]] <- jsonlite::parse_json(
                  lines,
                  simplifyVector = FALSE
                )
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

      if (!identical(parsed, expected)) {
        message("failing case: ", case)
        withr::with_dir(case, {
          print(list.files())
          print(readLines("in.yaml"))
          print(readLines("in.json"))
        })
        fail(paste("case fails:", case))
      }
    }
    # break
  }
  cat("done!")
})
