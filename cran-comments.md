## R CMD check results

0 errors | 0 warnings | 0 notes

## Reverse dependencies

Rapp is the only reverse dependency. Its existing exact-output snapshots fail
because of intentional formatting changes in yaml12 0.2.0. A pending
compatibility PR keeps explicit snapshots for both CRAN yaml12 0.1.0 and
yaml12 0.2.0:

https://github.com/r-lib/Rapp/pull/41

With that pending change, both yaml12 variants pass Rapp's full test suite and
`R CMD check --as-cran --no-manual` with 0 errors, 0 warnings, and 1 expected
note about Rapp's development version number.
