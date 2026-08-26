vendor_archive_pax_headers_contain <- function(path, patterns) {
  connection <- xzfile(path, open = "rb")
  on.exit(close(connection))

  repeat {
    header <- readBin(connection, what = "raw", n = 512L)
    if (length(header) != 512L) {
      stop("vendored archive has a truncated tar header", call. = FALSE)
    }
    if (all(header == as.raw(0))) {
      return(FALSE)
    }

    size_digits <- rawToChar(header[125:136], multiple = TRUE)
    size_digits <- size_digits[size_digits %in% as.character(0:7)]
    size <- if (length(size_digits)) {
      strtoi(paste(size_digits, collapse = ""), base = 8L)
    } else {
      0L
    }
    if (is.na(size)) {
      stop("vendored archive has an invalid tar size", call. = FALSE)
    }

    data_size <- ceiling(size / 512) * 512
    data <- readBin(connection, what = "raw", n = data_size)
    if (length(data) != data_size) {
      stop("vendored archive has truncated tar data", call. = FALSE)
    }

    type <- rawToChar(header[[157L]], multiple = TRUE)
    is_pax_header <- type %in% c("x", "g")
    has_pattern <- any(vapply(
      patterns,
      function(pattern) {
        length(grepRaw(pattern, data[seq_len(size)], fixed = TRUE)) > 0L
      },
      logical(1)
    ))
    if (is_pax_header && has_pattern) {
      return(TRUE)
    }
  }
}

test_that("vendored Rust archive is portable", {
  archive <- source_file("src", "rust", "vendor.tar.xz")

  members <- utils::untar(archive, list = TRUE, tar = "internal")
  expect_gt(length(members), 0L)
  expect_true(all(grepl("^vendor(/|$)", members)))
  expect_false(any(grepl(
    "(^|/)\\.\\.(/|$)|^/|^[A-Za-z]:[/\\\\]",
    members
  )))
  expect_false(any(grepl(
    "(^|/)\\._|(^|/)\\.DS_Store$|(^|/)__MACOSX(/|$)",
    members
  )))
  expect_false(vendor_archive_pax_headers_contain(
    archive,
    c("LIBARCHIVE.xattr", "SCHILY.xattr")
  ))
})
