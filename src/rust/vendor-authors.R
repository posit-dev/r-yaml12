if (!require('jsonlite')) {
  install.packages('jsonlite', repos = 'https://cloud.r-project.org')
}
metadata <- jsonlite::fromJSON(pipe("cargo metadata --format-version 1"))
packages <- metadata$packages
stopifnot(is.data.frame(packages))
root_id <- metadata$resolve$root
if (!is.null(root_id) && length(root_id) == 1) {
  packages <- subset(packages, id != root_id)
}

authors <- lapply(seq_len(nrow(packages)), function(i) {
  package_authors <- packages$authors[[i]]
  if (length(package_authors) > 0) {
    return(package_authors)
  }

  repository <- packages$repository[[i]]
  if (length(repository) == 1 && !is.na(repository) && nzchar(repository)) {
    paste("see", repository)
  } else {
    "see crate source"
  }
})
keep <- packages$name != 'myrustlib'
packages <- packages[keep, ]
authors <- authors[keep]
author_lines <- vapply(
  authors,
  function(x) paste(sub(" <.*>", "", x), collapse = ', '),
  character(1)
)
lines <- sprintf(" - %s %s: %s", packages$name, packages$version, author_lines)
dir.create('../../inst', showWarnings = FALSE)
footer <- sprintf(
  "\n(This file was auto-generated from 'cargo metadata' on %s)",
  Sys.Date()
)
writeLines(
  c('Authors of vendored cargo crates', lines, footer),
  '../../inst/AUTHORS'
)
