#!/usr/bin/env Rscript

# Refresh the complete, locked Rust dependency snapshot used by source builds.
# Run from the package root:
#   Rscript --vanilla tools/vendor-rust.R

package_name <- tryCatch(
  read.dcf("DESCRIPTION", "Package")[[1L]],
  error = function(e) NULL
)
if (!identical(package_name, "yaml12")) {
  stop(
    "must be run from the yaml12 package root",
    call. = FALSE
  )
}

if (!requireNamespace("jsonlite", quietly = TRUE)) {
  stop(
    "package 'jsonlite' is required; install it before refreshing dependencies",
    call. = FALSE
  )
}

find_tool <- function(name) {
  path <- Sys.which(name)
  if (!nzchar(path)) {
    stop(name, " is required but was not found on PATH", call. = FALSE)
  }
  path
}

run <- function(command, args, wd, stdout = "", env = character()) {
  old_wd <- setwd(wd)
  on.exit(setwd(old_wd))

  if (length(env)) {
    if (is.null(names(env)) || any(!nzchar(names(env)))) {
      stop("environment variables must be named", call. = FALSE)
    }
    old_env <- Sys.getenv(names(env), unset = NA_character_)
    names(old_env) <- names(env)
    on.exit(
      {
        restore <- !is.na(old_env)
        if (any(restore)) {
          do.call(Sys.setenv, as.list(old_env[restore]))
        }
        if (any(!restore)) {
          Sys.unsetenv(names(old_env)[!restore])
        }
      },
      add = TRUE
    )
    do.call(Sys.setenv, as.list(env))
  }

  status <- system2(
    command,
    args,
    stdout = stdout,
    stderr = ""
  )
  if (is.null(status)) {
    status <- 0L
  }
  if (!identical(as.integer(status), 0L)) {
    stop(
      "command failed: ",
      command,
      " ",
      paste(args, collapse = " "),
      call. = FALSE
    )
  }
}

scalar_or <- function(x, fallback) {
  if (is.null(x) || length(x) != 1L || is.na(x) || !nzchar(x)) {
    fallback
  } else {
    x
  }
}

crate_authors <- function(package) {
  authors <- unlist(package$authors, use.names = FALSE)
  if (length(authors)) {
    authors <- sub(" <[^>]+>$", "", authors)
    return(paste(authors, collapse = ", "))
  }

  repository <- scalar_or(package$repository, "")
  if (nzchar(repository)) {
    paste("see", repository)
  } else {
    "see crate source"
  }
}

dependency_packages <- function(metadata) {
  root_id <- metadata$resolve$root
  packages <- Filter(
    function(package) !identical(package$id, root_id),
    metadata$packages
  )
  names <- vapply(packages, `[[`, character(1), "name")
  versions <- vapply(packages, `[[`, character(1), "version")
  packages[order(names, versions)]
}

write_authors <- function(packages, path) {
  lines <- vapply(
    packages,
    function(package) {
      sprintf(
        " - %s %s: %s",
        package$name,
        package$version,
        crate_authors(package)
      )
    },
    character(1)
  )
  footer <- sprintf(
    "\n(This file was auto-generated from 'cargo metadata' on %s)",
    Sys.Date()
  )
  writeLines(c("Authors of vendored cargo crates", lines, footer), path)
}

write_license_note <- function(packages, path) {
  separator <- paste(rep("-", 61L), collapse = "")
  blocks <- unlist(lapply(packages, function(package) {
    c(
      separator,
      "",
      sprintf("Name:        %s", package$name),
      sprintf(
        "Repository:  %s",
        scalar_or(package$repository, "see crate source")
      ),
      sprintf("Authors:     %s", crate_authors(package)),
      sprintf(
        "License:     %s",
        scalar_or(package$license, "see crate source")
      ),
      ""
    )
  }))
  header <- c(
    paste(
      "The binary compiled from the source code of this package contains",
      "the following Rust crates:"
    ),
    "",
    ""
  )
  writeLines(c(header, blocks, separator), path)
}

pax_headers_contain <- function(path, patterns) {
  connection <- xzfile(path, open = "rb")
  on.exit(close(connection))

  repeat {
    header <- readBin(connection, what = "raw", n = 512L)
    if (length(header) != 512L) {
      stop("generated vendor archive has a truncated tar header", call. = FALSE)
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
      stop("generated vendor archive has an invalid tar size", call. = FALSE)
    }

    data_size <- ceiling(size / 512) * 512
    data <- readBin(connection, what = "raw", n = data_size)
    if (length(data) != data_size) {
      stop("generated vendor archive has truncated tar data", call. = FALSE)
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

validate_archive <- function(path) {
  members <- utils::untar(path, list = TRUE, tar = "internal")
  if (!length(members)) {
    stop("generated vendor archive is empty", call. = FALSE)
  }
  if (any(!grepl("^vendor(/|$)", members))) {
    stop(
      "generated vendor archive contains entries outside vendor/",
      call. = FALSE
    )
  }
  if (any(grepl("(^|/)\\.\\.(/|$)|^/|^[A-Za-z]:[/\\\\]", members))) {
    stop("generated vendor archive contains an unsafe path", call. = FALSE)
  }
  if (any(grepl("(^|/)\\._|(^|/)\\.DS_Store$|(^|/)__MACOSX(/|$)", members))) {
    stop("generated vendor archive contains macOS metadata", call. = FALSE)
  }
  if (pax_headers_contain(path, c("LIBARCHIVE.xattr", "SCHILY.xattr"))) {
    stop("generated vendor archive contains extended attributes", call. = FALSE)
  }
  invisible(members)
}

publish <- function(from, to) {
  if (!file.copy(from, to, overwrite = TRUE, copy.mode = TRUE)) {
    stop("failed to update ", to, call. = FALSE)
  }
}

main <- function() {
  repo_root <- normalizePath(getwd(), winslash = "/", mustWork = TRUE)
  rust_dir <- file.path(repo_root, "src", "rust")
  manifest <- file.path(rust_dir, "Cargo.toml")
  lockfile <- file.path(rust_dir, "Cargo.lock")
  if (!file.exists(manifest) || !file.exists(lockfile)) {
    stop("src/rust/Cargo.toml or Cargo.lock is missing", call. = FALSE)
  }

  cargo <- find_tool("cargo")
  vendor_dir <- file.path(rust_dir, "vendor")
  config_stage <- file.path(rust_dir, "vendor-config.toml.new")
  archive_stage <- file.path(rust_dir, "vendor.tar.xz.new")
  metadata_stage <- file.path(rust_dir, "cargo-metadata.json.new")
  authors_stage <- file.path(repo_root, "inst", "AUTHORS.new")
  license_stage <- file.path(repo_root, "LICENSE.note.new")
  staged_files <- c(
    config_stage,
    archive_stage,
    metadata_stage,
    authors_stage,
    license_stage
  )

  unlink(c(vendor_dir, staged_files), recursive = TRUE, force = TRUE)
  on.exit(
    unlink(c(vendor_dir, staged_files), recursive = TRUE, force = TRUE),
    add = TRUE
  )

  message("Vendoring locked Rust dependencies...")
  run(
    cargo,
    c("vendor", "--locked", "vendor"),
    rust_dir,
    stdout = config_stage
  )
  if (!file.exists(config_stage) || !file.info(config_stage)$size) {
    stop("cargo vendor did not emit a source configuration", call. = FALSE)
  }

  message("Reading locked Cargo metadata...")
  run(
    cargo,
    c("metadata", "--locked", "--format-version", "1"),
    rust_dir,
    stdout = metadata_stage
  )
  metadata <- jsonlite::read_json(metadata_stage, simplifyVector = FALSE)
  packages <- dependency_packages(metadata)
  if (!length(packages)) {
    stop("cargo metadata returned no dependency packages", call. = FALSE)
  }
  write_authors(packages, authors_stage)
  write_license_note(packages, license_stage)

  message("Creating metadata-clean xz archive...")
  old_wd <- setwd(rust_dir)
  on.exit(setwd(old_wd), add = TRUE)
  utils::tar(
    archive_stage,
    files = "vendor",
    compression = "xz",
    compression_level = 9,
    tar = "internal"
  )
  setwd(old_wd)
  validate_archive(archive_stage)

  message("Checking the staged snapshot offline...")
  verification_dir <- tempfile("yaml12-vendor-check-")
  dir.create(file.path(verification_dir, ".cargo"), recursive = TRUE)
  on.exit(unlink(verification_dir, recursive = TRUE, force = TRUE), add = TRUE)
  utils::untar(
    archive_stage,
    exdir = verification_dir,
    tar = "internal"
  )
  config_copied <- file.copy(
    config_stage,
    file.path(verification_dir, ".cargo", "config.toml"),
    overwrite = TRUE
  )
  if (!config_copied) {
    stop("failed to stage the Cargo source configuration", call. = FALSE)
  }
  cargo_home <- file.path(verification_dir, "cargo-home")
  dir.create(cargo_home)
  verification_metadata <- file.path(verification_dir, "metadata.json")
  run(
    cargo,
    c(
      "metadata",
      "--locked",
      "--offline",
      "--format-version",
      "1",
      "--manifest-path",
      shQuote(manifest)
    ),
    verification_dir,
    stdout = verification_metadata,
    env = c(CARGO_HOME = cargo_home)
  )
  if (
    !file.exists(verification_metadata) ||
      !file.info(verification_metadata)$size
  ) {
    stop("offline Cargo metadata check returned no output", call. = FALSE)
  }

  publish(config_stage, file.path(rust_dir, "vendor-config.toml"))
  publish(authors_stage, file.path(repo_root, "inst", "AUTHORS"))
  publish(license_stage, file.path(repo_root, "LICENSE.note"))
  publish(archive_stage, file.path(rust_dir, "vendor.tar.xz"))

  message("Rust dependency snapshot refreshed and verified.")
}

main()
