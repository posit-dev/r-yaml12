windows_rust_target <- function(
  platform = R.version$platform,
  arch = R.version$arch,
  compiled_by = Sys.getenv("R_COMPILED_BY")
) {
  stopifnot(
    is.character(platform),
    length(platform) == 1L,
    !is.na(platform),
    is.character(arch),
    length(arch) == 1L,
    !is.na(arch),
    is.character(compiled_by),
    length(compiled_by) == 1L,
    !is.na(compiled_by)
  )

  platform <- tolower(platform)
  arch <- tolower(arch)

  if (!grepl("mingw|windows", platform)) {
    stop(
      "Expected a Windows platform, not `",
      platform,
      "`",
      call. = FALSE
    )
  }

  if (startsWith(platform, "aarch64-") || identical(arch, "aarch64")) {
    return("aarch64-pc-windows-gnullvm")
  }

  if (startsWith(platform, "i386-") || identical(arch, "i386")) {
    return("i686-pc-windows-gnu")
  }

  if (startsWith(platform, "x86_64-") || identical(arch, "x86_64")) {
    if (grepl("clang", compiled_by, ignore.case = TRUE)) {
      return("x86_64-pc-windows-gnullvm")
    }
    return("x86_64-pc-windows-gnu")
  }

  stop(
    "Unknown Windows architecture: `",
    arch,
    "` for platform `",
    platform,
    "`",
    call. = FALSE
  )
}

installed_rust_targets <- function(rustup = Sys.which("rustup")) {
  stopifnot(
    is.character(rustup),
    length(rustup) == 1L,
    !is.na(rustup)
  )

  rustup <- unname(rustup)
  if (identical(rustup, "")) {
    stop(
      "`rustup` is required to verify the Windows Rust target.",
      call. = FALSE
    )
  }

  targets <- system2(
    rustup,
    c("target", "list", "--installed"),
    stdout = TRUE,
    stderr = TRUE
  )
  status <- attr(targets, "status", exact = TRUE)

  if (!is.null(status) && status != 0L) {
    stop(
      "Failed to list installed Rust targets with `rustup target list --installed`.",
      call. = FALSE
    )
  }

  targets[nzchar(targets)]
}

check_windows_rust_target <- function(
  target = windows_rust_target(),
  installed_targets = installed_rust_targets()
) {
  stopifnot(
    is.character(target),
    length(target) == 1L,
    !is.na(target),
    is.character(installed_targets),
    !anyNA(installed_targets)
  )

  if (target %in% installed_targets) {
    return(invisible(target))
  }

  stop(
    "Rust target `",
    target,
    "` is required to build yaml12 on this Windows platform.\n",
    "Run: rustup target add ",
    target,
    call. = FALSE
  )
}

if (sys.nframe() == 0L) {
  args <- commandArgs(trailingOnly = TRUE)
  if (identical(args, "--check")) {
    check_windows_rust_target()
  } else {
    cat(windows_rust_target())
  }
}
