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
[`yaml12::write_yaml()`](https://posit-dev.github.io/r-yaml12/reference/format_yaml.md).

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
    #> 1 yaml12::read_…       840  33.2µs  34.4µs    27932.    2.75KB     0   
    #> 2 yaml::read_ya…       840 102.9µs 117.3µs     8156.   34.78KB     4.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-2.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…      1676  57.6µs  59.9µs    16180.        0B     0   
    #> 2 yaml::read_ya…      1676 146.7µs 152.9µs     6213.    10.8KB     4.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-3.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…      3348 110µs  113µs     8540.        0B     2.00  4269
    #> 2 yaml::read…      3348 237µs  245µs     3932.    13.7KB     4.00  1966
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-4.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…      6692 216µs  223µs     4333.        0B     2.00  2167
    #> 2 yaml::read…      6692 418µs  441µs     2248.    19.3KB     2.00  1124
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-5.png)

    #> # A tibble: 2 × 14
    #>   expression  file_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>      <dbl> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::re…     13380 432µs  460µs     2163.      304B     0     1082
    #> 2 yaml::read…     13380 785µs  816µs     1196.    30.9KB     4.00   598
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-6.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…     26756 876.28µs 900.57µs     1098.      560B     2.00
    #> 2 yaml::read_…     26756   1.55ms   1.59ms      614.    69.4KB     3.99
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-7.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…     53508 1.77ms  1.8ms      553.    1.05KB     0      277
    #> 2 yaml::rea…     53508 3.14ms  3.2ms      306.  146.29KB     3.98   154
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-8.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    107012 3.46ms 3.51ms      282.    2.05KB     1.99   142
    #> 2 yaml::rea…    107012 6.72ms 6.82ms      145.  300.09KB     1.99    73
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-9.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…    214020  6.65ms  6.81ms     147.     4.05KB     0   
    #> 2 yaml::read_ya…    214020 15.91ms 16.16ms      60.8  607.63KB     3.92
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-10.png)

    #> # A tibble: 2 × 14
    #>   expression file_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>     <dbl> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::r…    428036   13ms 13.1ms      74.9    8.05KB     1.97    38
    #> 2 yaml::rea…    428036 42.2ms 42.8ms      23.1    1.19MB     1.93    12
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-11.png)

    #> # A tibble: 2 × 14
    #>   expression      file_size     min median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>          <dbl> <bch:t> <bch:>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_y…    856068  25.9ms   27ms     36.8       16KB     1.94
    #> 2 yaml::read_yam…    856068 140.5ms  151ms      6.68     2.4MB     1.67
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-12.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…   1712132  53.8ms  54.6ms     18.0       32KB     2.00
    #> 2 yaml::read_ya…   1712132 860.6ms 860.6ms      1.16     4.8MB     0   
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-13.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…   3424260 107.86ms 110.36ms     8.56       64KB    1.71 
    #> 2 yaml::read_…   3424260    4.25s    4.25s     0.235     9.6MB    0.235
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-14.png)

    #> # A tibble: 2 × 14
    #>   expression     file_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>         <dbl> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::read_…   6848516 220.7ms 224.6ms    4.47       128KB    2.98 
    #> 2 yaml::read_ya…   6848516   18.1s   18.1s    0.0553    19.2MB    0.111
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-8-15.png)

    #> # A tibble: 2 × 14
    #>   expression   file_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>       <dbl> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::rea…  13697028 429.64ms 460.93ms    2.17       256KB   3.25  
    #> 2 yaml::read_…  13697028    1.07m    1.07m    0.0155    38.4MB   0.0776
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
    #> 1 yaml12::wr… 3360 by… 17.9µs 18.4µs    51859.    3.02KB     5.19 10000
    #> 2 yaml::writ… 3360 by… 68.1µs 72.4µs    12992.   33.68KB     4.00  6493
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-2.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6672 by… 29.8µs 30.5µs    31478.        0B     3.15 10000
    #> 2 yaml::writ… 6672 by… 90.6µs 95.4µs     9538.    1.56KB     4.00  4767
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-3.png)

    #> # A tibble: 2 × 14
    #>   expression      obj_size     min  median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>      <objct_> <bch:t> <bch:t>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::write_… 13296 b…  53.6µs  54.6µs    17794.        0B     0   
    #> 2 yaml::write_ya… 13296 b… 134.9µs 143.6µs     6384.    3.07KB     6.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-4.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 26544 b… 100µs  102µs     9551.        0B     0     4774
    #> 2 yaml::write… 26544 b… 223µs  236µs     3937.    6.09KB     6.00  1968
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-5.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 53040 b… 197µs  202µs     4864.        0B     0     2432
    #> 2 yaml::write… 53040 b… 409µs  437µs     2211.    12.1KB     4.00  1106
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-6.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 106032 … 392µs  406µs     2450.        0B     0     1226
    #> 2 yaml::write… 106032 … 779µs  815µs     1151.    24.2KB     5.99   576
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-7.png)

    #> # A tibble: 2 × 14
    #>   expression    obj_size      min   median `itr/sec` mem_alloc `gc/sec`
    #>   <bch:expr>    <objct_> <bch:tm> <bch:tm>     <dbl> <bch:byt>    <dbl>
    #> 1 yaml12::writ… 212016 … 781.09µs 803.64µs     1241.        0B     0   
    #> 2 yaml::write_… 212016 …   1.53ms   1.57ms      594.    48.3KB     6.00
    #> # ℹ 7 more variables: n_itr <int>, n_gc <dbl>, total_time <bch:tm>,
    #> #   result <list>, memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-8.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 423984 … 1.54ms 1.56ms      639.        0B     0      320
    #> 2 yaml::writ… 423984 … 3.02ms 3.08ms      310.    96.6KB     3.98   156
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-9.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 847920 … 3.02ms 3.06ms      325.        0B     0      163
    #> 2 yaml::writ… 847920 … 5.97ms 6.09ms      155.     193KB     3.98    78
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-10.png)

    #> # A tibble: 2 × 14
    #>   expression obj_size    min  median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr> <objct_> <bch:> <bch:t>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::w… 1695792…    6ms  6.04ms     165.         0B     0       83
    #> 2 yaml::wri… 1695792… 11.8ms 12.08ms      51.3     386KB     3.95    26
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-11.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 3391536… 12.1ms 12.2ms      81.5        0B     0       41
    #> 2 yaml::writ… 3391536… 23.4ms 23.8ms      41.4     772KB     3.94    21
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-12.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 6783024…   24ms 24.5ms      40.4        0B     0       21
    #> 2 yaml::writ… 6783024… 47.3ms 47.7ms      20.6    1.51MB     3.75    11
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-13.png)

    #> # A tibble: 2 × 14
    #>   expression  obj_size    min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>  <objct_> <bch:> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wr… 1356600… 51.3ms 53.5ms      18.7        0B     0       10
    #> 2 yaml::writ… 1356600… 94.7ms 95.5ms      10.4    3.02MB     3.46     6
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-14.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 2713195… 106ms  107ms      9.36        0B     0        5
    #> 2 yaml::write… 2713195… 194ms  194ms      5.15    6.03MB     5.15     3
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

![](benchmarks_files/figure-html/unnamed-chunk-12-15.png)

    #> # A tibble: 2 × 14
    #>   expression   obj_size   min median `itr/sec` mem_alloc `gc/sec` n_itr
    #>   <bch:expr>   <objct_> <bch> <bch:>     <dbl> <bch:byt>    <dbl> <int>
    #> 1 yaml12::wri… 5426385… 210ms  210ms      4.77        0B     0        3
    #> 2 yaml::write… 5426385… 389ms  469ms      2.13    12.1MB     3.20     2
    #> # ℹ 6 more variables: n_gc <dbl>, total_time <bch:tm>, result <list>,
    #> #   memory <list>, time <list>, gc <list>

## Conclusion

Across all tested file sizes and for this workload, `yaml12` is faster
than `yaml` at reading and writing YAML.
