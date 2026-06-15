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

if (sys.nframe() == 0L) {
  cat(windows_rust_target())
}
