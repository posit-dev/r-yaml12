# Benchmarks

## Goal

Compare `yaml12` and `yaml` for reading and writing YAML over a range of
document sizes.

We intentionally avoid testing semantic differences between YAML 1.1 and
1.2, as well as less-used YAML features like tags and streams. Instead,
we focus on the simple default usage of both.

Operations:

- **read**: YAML file → R object
- **write**: R object → YAML (written to
  [`nullfile()`](https://rdrr.io/r/base/showConnections.html))

## Setup

``` r

library(dplyr, warn.conflicts = FALSE)
library(ggplot2)
```

### Generate inputs

To benchmark, we create YAML files consisting of a repeating sequence of
a small, fixed “mixed” node, repeated `2^(1:15)` times. This node is
designed to exercise the full YAML 1.2 core schema (every core type is
represented). We generate these files with
[`yaml12::write_yaml()`](https://posit-dev.github.io/r-yaml12/dev/reference/format_yaml.md).

Each file is written to a temporary file and then we measure read time.
Note that on macOS and Linux,
[`tempfile()`](https://rdrr.io/r/base/tempfile.html) paths often live on
a `tmpfs` in RAM, so this typically won’t trigger a write to disk under
most circumstances.

``` r

# A YAML node that exercises every core schema type:
#   seq, map, str, bool, int, float, null
mixed_node <- list(
  str = c(
    "Lorem ipsum dolor sit amet, vel accumsan vitae faucibus ultrices leo",
    "neque? Et cursus lacinia, ut, sit donec facilisi eu interdum. Dui",
    "ipsum, vitae ligula commodo convallis ac sed nunc. Ipsum at nec lacus",
    "eros suscipit vitae."
  ),
  block_str = "lorem \n ipsum \n dolor\n",
  bools = c(TRUE, FALSE),
  ints = c(123L, -123L),
  floats = c(123.456, -123.456),
  null = NULL
)
```

``` r

make_yaml_doc <- function(size = 1) {
  path <- tempfile(fileext = ".yaml")
  yaml12::write_yaml(rep(list(mixed_node), size), path)
  path
}

docs <- sapply(2^(1:15), make_yaml_doc)
```

## Read performance

``` r

read_results <- lapply(docs, function(doc) {
  result <- bench::mark(
    yaml12::read_yaml(doc),
    yaml::read_yaml(doc),
    check = FALSE
  )
  result$file_size <- file.info(doc)$size
  result
})
#> Warning: Some expressions had a GC in every iteration; so filtering is
#> disabled.
#> Warning: Some expressions had a GC in every iteration; so filtering is
#> disabled.
#> Warning: Some expressions had a GC in every iteration; so filtering is
#> disabled.
```

### Read results summary

``` r

read_results_df <- bind_rows(read_results) |>
  mutate(expression = as.factor(sapply(expression, deparse1)))
```

``` r

read_results_df |>
  ggplot(aes(x = file_size, y = median, color = expression)) +
  labs(
    y = "Median Run Time",
    x = "File Size",
    title = "Read Performance"
  ) +
  geom_point() +
  geom_smooth(linewidth = .5) +
  scale_x_log10(labels = scales::label_bytes()) +
  scale_y_continuous(trans = bench::bench_time_trans())
#> `geom_smooth()` using method = 'loess' and formula = 'y ~ x'
```

![](benchmarks_files/figure-html/unnamed-chunk-7-1.png)

### Read results (detailed)

``` r

invisible(lapply(read_results, \(result) {
  print(plot(result) + ggplot2::ggtitle(scales::label_bytes()(result$file_size[1])))
  print(summary(result, filter_gc = FALSE))
}))
```

![](benchmarks_files/figure-html/unnamed-chunk-8-1.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…       856  33.1µs  34.6µs    28003.     2.6KB     0   
    #> 2 yaml::read_ya…       856 102.8µs 117.5µs     8330.    34.8KB     2.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-2.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…      1704  59.9µs  62.5µs    15663.        0B     0   
    #> 2 yaml::read_ya…      1704 146.7µs 152.2µs     6316.    10.9KB     4.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-3.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…      3400 114µs  119µs     8143.        0B     0     4071
    #> 2 yaml::read…      3400 235µs  244µs     3974.    13.7KB     4.00  1987
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-4.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…      6792 223µs  234µs     4214.        0B     0     2107
    #> 2 yaml::read…      6792 416µs  436µs     2217.    19.4KB     4.00  1109
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-5.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…     13576 451µs  471µs     2125.      304B     0     1063
    #> 2 yaml::read…     13576 776µs  809µs     1201.    31.1KB     4.00   601
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-6.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…     27144  911.3µs 936.74µs     1054.      560B     2.00
    #> 2 yaml::read_…     27144   1.53ms   1.57ms      630.    69.8KB     2.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-7.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…     54280 1.83ms 1.86ms      531.    1.05KB     2.00   266
    #> 2 yaml::rea…     54280  3.1ms 3.15ms      308.  147.06KB     4.00   154
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-8.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    108552 3.59ms  3.7ms      270.    2.05KB     0      136
    #> 2 yaml::rea…    108552 6.67ms 6.77ms      144.   301.6KB     3.95    73
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-9.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…    217096  7.06ms  7.16ms     139.     4.05KB     0   
    #> 2 yaml::read_ya…    217096 15.88ms 16.05ms      61.2  610.65KB     1.97
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-10.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    434184 13.9ms 14.1ms      69.9    8.05KB     2.00    35
    #> 2 yaml::rea…    434184 42.1ms 42.5ms      23.3     1.2MB     1.94    12
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-11.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…    868360  27.3ms  27.9ms     35.5    16.05KB     0   
    #> 2 yaml::read_ya…    868360   141ms   147ms      6.82    2.41MB     1.71
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-12.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…   1736712  56.7ms  57.7ms     17.2    32.05KB        0
    #> 2 yaml::read_ya…   1736712 878.2ms 878.2ms      1.14    4.82MB        0
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-13.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…   3473416 115.02ms 117.28ms     8.36    64.05KB    1.67 
    #> 2 yaml::read_…   3473416    4.16s    4.16s     0.240    9.65MB    0.240
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-14.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…   6946824 235.6ms 269.1ms    3.72       128KB    1.86 
    #> 2 yaml::read_ya…   6946824   17.8s   17.8s    0.0561    19.3MB    0.112
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-15.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…  13893640 456.26ms 465.33ms    2.15       256KB    1.07 
    #> 2 yaml::read_…  13893640    1.14m    1.14m    0.0146    38.6MB    0.102
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

## Write performance

``` r

objs <- lapply(2^(1:15), function(n) rep(list(mixed_node), n))

write_results <- lapply(objs, function(obj) {
  result <- bench::mark(
    yaml12::write_yaml(obj, nullfile()),
    yaml::write_yaml(obj, nullfile()),
    check = FALSE
  )
  result$obj_size <- object.size(obj)
  result
})
#> Warning: Some expressions had a GC in every iteration; so filtering is
#> disabled.
```

### Write results summary:

``` r

write_results_df <- bind_rows(write_results) |>
  mutate(expression = as.factor(sapply(expression, deparse1)))
```

``` r

write_results_df |>
  ggplot(aes(x = obj_size, y = median, color = expression)) +
  labs(
    y = "Median Run Time",
    x = "Object Size",
    title = "Write Performance"
  ) +
  geom_point() +
  geom_smooth(linewidth = .5) +
  scale_x_log10(labels = scales::label_bytes()) +
  scale_y_continuous(trans = bench::bench_time_trans())
#> `geom_smooth()` using method = 'loess' and formula = 'y ~ x'
```

![](benchmarks_files/figure-html/unnamed-chunk-11-1.png)

### Write results (detailed)

``` r

invisible(lapply(write_results, \(result) {
  print(plot(result) + ggplot2::ggtitle(scales::label_bytes()(as.numeric(result$obj_size[1]))))
  print(summary(result, filter_gc = FALSE))
}))
```

![](benchmarks_files/figure-html/unnamed-chunk-12-1.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 3360 by… 14.4µs 14.8µs    65894.    3.02KB     0    10000
    #> 2 yaml::writ… 3360 by… 67.8µs 71.1µs    13313.   33.68KB     2.00  6653
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-2.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6672 by… 22.5µs 23.1µs    42268.        0B     0    10000
    #> 2 yaml::writ… 6672 by… 88.9µs   93µs     9558.    1.56KB     4.00  4777
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-3.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size   min  median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch> <bch:t>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 13296 b…  40µs  40.9µs    23924.        0B     0    10000
    #> 2 yaml::writ… 13296 b… 133µs 138.3µs     6871.    3.07KB     2.00  3435
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-4.png)

    #> # A tibble: 2 × 14
    #>   expression obj_size     min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr> <objct_> <bch:t> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::w… 26544 b…  74.4µs   76µs    12786.        0B     0     6390
    #> 2 yaml::wri… 26544 b… 218.3µs  227µs     4218.    6.09KB     2.00  2109
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-5.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 53040 b… 145µs  153µs     6463.        0B     0     3231
    #> 2 yaml::write… 53040 b… 399µs  425µs     2189.    12.1KB     4.00  1095
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-6.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 106032 … 291µs  299µs     3295.        0B     0     1648
    #> 2 yaml::write… 106032 … 762µs  792µs     1232.    24.2KB     2.00   617
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-7.png)

    #> # A tibble: 2 × 14
    #>   expression     obj_size     min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>     <objct_> <bch:t> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::write… 212016 … 582.4µs 600.79µs     1666.        0B     0   
    #> 2 yaml::write_y… 212016 …   1.5ms   1.53ms      637.    48.3KB     2.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-8.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 423984 … 1.14ms 1.16ms      862.        0B     0      431
    #> 2 yaml::writ… 423984 … 2.96ms 3.01ms      318.    96.6KB     3.98   160
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-9.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 847920 … 2.22ms 2.25ms      442.        0B     0      222
    #> 2 yaml::writ… 847920 … 5.87ms 5.96ms      163.     193KB     1.99    82
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-10.png)

    #> # A tibble: 2 × 14
    #>   expression      obj_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>      <objct_> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::write_… 1695792…  4.35ms  4.38ms     227.         0B     0   
    #> 2 yaml::write_ya… 1695792… 11.71ms 11.84ms      82.6     386KB     1.97
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-11.png)

    #> # A tibble: 2 × 14
    #>   expression      obj_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>      <objct_> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::write_… 3391536…  8.57ms  8.62ms     116.         0B     0   
    #> 2 yaml::write_ya… 3391536… 23.44ms 23.67ms      40.5     772KB     3.86
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-12.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6783024…   17ms 17.3ms      57.3        0B     0       29
    #> 2 yaml::writ… 6783024… 47.1ms 47.7ms      20.6    1.51MB     1.87    11
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-13.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 1356600… 35.7ms 36.3ms      27.5        0B     0       14
    #> 2 yaml::writ… 1356600… 94.2ms 95.6ms      10.3    3.02MB     1.71     6
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-14.png)

    #> # A tibble: 2 × 14
    #>   expression      obj_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>      <objct_> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::write_… 2713195…  75.9ms  77.3ms     12.9         0B     0   
    #> 2 yaml::write_ya… 2713195… 190.3ms 190.7ms      5.17    6.03MB     1.72
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-15.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 5426385… 152ms  152ms      6.50        0B     0        4
    #> 2 yaml::write… 5426385… 389ms  485ms      2.06    12.1MB     2.06     2
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

## Conclusion

Across all tested file sizes and for this workload, `yaml12` is faster
than `yaml` at reading and writing YAML.
