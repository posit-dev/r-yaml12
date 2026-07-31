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
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…       828 22.4µs 25.5µs    38018.     2.6KB     0    10000
    #> 2 yaml::rea…       828 60.7µs 72.9µs    13086.    34.7KB     5.97  6580
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-2.png)

    #> # A tibble: 2 × 14
    #>   expression      file_size    min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>          <dbl> <bch:> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_y…      1648 43.1µs  48.8µs    20225.        0B     0   
    #> 2 yaml::read_yam…      1648 93.9µs 108.2µs     8934.    10.6KB     6.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-3.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…      3288  84.9µs  91.5µs    10581.        0B     0   
    #> 2 yaml::read_ya…      3288 163.1µs 179.9µs     5207.    13.2KB     6.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-4.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…      6568 169µs  186µs     5295.        0B     0     2647
    #> 2 yaml::read…      6568 307µs  337µs     2849.    18.4KB     4.00  1425
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-5.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…     13128 338µs  364µs     2678.      304B     2.00  1339
    #> 2 yaml::read…     13128 592µs  641µs     1497.    29.1KB     4.00   749
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-6.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…     26248 689.54µs  731.3µs     1345.      560B     0   
    #> 2 yaml::read_…     26248   1.19ms   1.28ms      738.    65.9KB     6.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-7.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…     52488 1.45ms 1.56ms      628.    1.05KB     0      314
    #> 2 yaml::rea…     52488 2.47ms 2.67ms      363.  139.31KB     3.99   182
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-8.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    104968 3.28ms 3.47ms      286.    2.05KB     0      143
    #> 2 yaml::rea…    104968 5.41ms 5.74ms      169.   286.1KB     3.99    85
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-9.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…    209928  6.65ms  6.89ms     142.     4.05KB     2.00
    #> 2 yaml::read_ya…    209928 12.95ms 13.79ms      71.5  579.65KB     1.99
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-10.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    419848 12.6ms 13.5ms      73.4    8.05KB     1.98    37
    #> 2 yaml::rea…    419848 37.2ms 39.2ms      25.2    1.14MB     1.94    13
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-11.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…    839688  24.2ms  25.7ms     38.1    16.05KB     1.90
    #> 2 yaml::read_ya…    839688 129.3ms 137.1ms      7.27    2.29MB     0   
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-12.png)

    #> # A tibble: 2 × 14
    #>   expression      file_size     min median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>          <dbl> <bch:t> <bch:>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_y…   1679368  48.1ms   52ms     19.0    32.05KB     0   
    #> 2 yaml::read_yam…   1679368 658.9ms  659ms      1.52    4.58MB     1.52
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-13.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…   3358728 107.77ms 111.77ms     8.58    64.05KB    1.72 
    #> 2 yaml::read_…   3358728    2.78s    2.78s     0.360    9.16MB    0.360
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-14.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…   6717448 234.2ms 241.1ms    4.01       128KB    2.67 
    #> 2 yaml::read_ya…   6717448   12.6s   12.6s    0.0796    18.3MB    0.159
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-15.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…  13434888 422.25ms 429.85ms    2.33       256KB   1.16  
    #> 2 yaml::read_…  13434888    1.25m    1.25m    0.0133    36.7MB   0.0800
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
    #> 1 yaml12::wr… 3360 by… 16.1µs 17.2µs    49638.    3.02KB     4.96 10000
    #> 2 yaml::writ… 3360 by… 34.1µs   39µs    24544.   33.68KB     2.45 10000
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-2.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6672 by… 28.8µs 32.9µs    30051.        0B     0    10000
    #> 2 yaml::writ… 6672 by… 51.1µs 57.9µs    15908.    1.56KB     4.00  7951
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-3.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 13296 b… 55.2µs 57.6µs    16677.        0B     2.00  8335
    #> 2 yaml::writ… 13296 b… 85.2µs 95.8µs    10079.    3.07KB     2.00  5039
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-4.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 26544 b… 107µs  112µs     8619.        0B     2.00  4309
    #> 2 yaml::write… 26544 b… 155µs  168µs     5773.    6.09KB     2.00  2887
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-5.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 53040 b… 211µs  223µs     4341.        0B     0     2171
    #> 2 yaml::write… 53040 b… 296µs  338µs     2891.    12.1KB     4.00  1446
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-6.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 106032 … 422µs  477µs     2144.        0B     0     1073
    #> 2 yaml::write… 106032 … 587µs  675µs     1464.    24.2KB     4.00   733
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-7.png)

    #> # A tibble: 2 × 14
    #>   expression    obj_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>    <objct_> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::writ… 212016 … 843.99µs 885.62µs     1101.        0B     0   
    #> 2 yaml::write_… 212016 …   1.16ms   1.25ms      776.    48.3KB     2.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-8.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 423984 …  1.7ms 1.77ms      553.        0B     0      277
    #> 2 yaml::writ… 423984 … 2.33ms 2.52ms      383.    96.6KB     3.99   192
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-9.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 847920 … 3.57ms 3.78ms      262.        0B     0      131
    #> 2 yaml::writ… 847920 …  4.7ms 5.14ms      191.     193KB     3.98    96
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-10.png)

    #> # A tibble: 2 × 14
    #>   expression obj_size    min  median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr> <objct_> <bch:> <bch:t>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::w… 1695792… 7.38ms  7.94ms     127.         0B     0       64
    #> 2 yaml::wri… 1695792…  9.5ms 10.53ms      94.8     386KB     1.98    48
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-11.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 3391536… 14.6ms 15.4ms      64.5        0B     0       33
    #> 2 yaml::writ… 3391536… 19.6ms 21.2ms      46.5     772KB     3.87    24
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-12.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6783024… 30.1ms 32.2ms      30.6        0B     0       16
    #> 2 yaml::writ… 6783024… 39.5ms 42.8ms      22.6    1.51MB     3.77    12
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-13.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 1356600… 59.9ms   64ms      15.5        0B     0        8
    #> 2 yaml::writ… 1356600… 84.4ms 84.9ms      11.3    3.02MB     3.76     6
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-14.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 2713195… 127ms  130ms      7.67        0B     0        4
    #> 2 yaml::write… 2713195… 170ms  177ms      5.68    6.03MB     3.78     3
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-15.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 5426385… 277ms  278ms      3.59        0B     0        2
    #> 2 yaml::write… 5426385… 345ms  346ms      2.89    12.1MB     2.89     2
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

## Conclusion

Across all tested file sizes and for this workload, `yaml12` is faster
than `yaml` at reading and writing YAML.
