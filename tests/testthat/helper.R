zap_yaml_tags <- function(x) {
  attr(x, "yaml_tag") <- NULL
  if (is.list(x)) {
    x <- lapply(x, zap_yaml_tags)
  }
  x
}

source_file <- function(...) {
  paths <- list(...)
  candidates <- c(
    do.call(test_path, c(list("..", ".."), paths)),
    do.call(
      file.path,
      c(list(getwd(), "..", "..", "00_pkg_src", "yaml12"), paths)
    )
  )

  for (candidate in candidates) {
    if (file.exists(candidate)) {
      return(candidate)
    }
  }

  skip("Source file is not available in this test layout")
}
