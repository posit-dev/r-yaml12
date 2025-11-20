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

