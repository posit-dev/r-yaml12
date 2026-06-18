# format_yaml multi-doc output stays stable

    Code
      format_yaml(docs, multi = TRUE)
    Output
      [1] "---\nfoo: 1\n---\nbar:\n  - 2\n  - ~\n"

# write_yaml snapshot aids emitter regressions

    Code
      readChar(path, file.info(path)$size)
    Output
      [1] "---\ntail: |\n  line1\n  line2\n...\n"

# read_yaml errors clearly on non-UTF-8 input

    Code
      read_yaml("latin1.yaml")
    Condition
      Error in `read_yaml()`:
      ! Failed to read `latin1.yaml`: stream did not contain valid UTF-8

---

    Code
      read_yaml("latin1.yaml", multi = TRUE)
    Condition
      Error in `read_yaml()`:
      ! Failed to read `latin1.yaml`: stream did not contain valid UTF-8

