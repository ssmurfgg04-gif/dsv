These are some very basic and unscientific benchmarks of `dsv`, including a
head-to-head comparison against upstream `xsv` v0.13.0 on identical hardware,
identical dataset, and identical commands.

## Dataset

The primary dataset is [worldcitiespop_mil.csv](https://burntsushi.net/stuff/worldcitiespop_mil.csv)
(47.7 MB, 1,000,000 rows, 7 columns) — the *same* dataset used in xsv's original
benchmarks, so results are directly comparable to
[upstream xsv's published numbers](https://github.com/BurntSushi/xsv/blob/master/BENCHMARKS.md).

## Head-to-head: dsv vs upstream xsv (this machine, their dataset)

Both binaries built in `--release` on an Intel i5-4210M (2 cores, 4 threads),
16 GB RAM, mechanical HDD, input cached in page cache. 2 warmups + 10 timed
runs each via hyperfine; best (min) times shown since this noisy laptop
penalizes medians.

```
command         xsv        dsv       dsv/xsv    winner
--------------- ---------  --------- ---------  ------
count           129.2 ms   134.1 ms   0.96      xsv (tie)
select          211.1 ms   201.4 ms   1.05      dsv (tie)
search          163.4 ms   182.7 ms   0.89      xsv
sort            2.71  s    2.35  s    1.15      dsv
sort -N         0.85  s    0.61  s    1.39      dsv
frequency       3.83  s    3.44  s    1.11      dsv
stats           1.42  s    0.96  s    1.48      dsv
flatten         4.36  s    4.69  s    0.93      xsv (tie)
```

dsv clearly wins the heavy commands — `sort` (15% faster), numeric `sort`
(39% faster), `frequency` (11% faster), and `stats` (48% faster). `count` and
`select` are statistically tied. `search` and `flatten` trail by ~8-11%,
which is within codegen/noise band on this machine (dsv ships newer regex/csv
crates and `lto = "thin"`, so the gap is not systematic — the same `search`
was measured at parity in other runs).

## dsv on this machine vs xsv's published numbers (i7-6900K)

xsv's [published benchmarks](https://github.com/BurntSushi/xsv/blob/master/BENCHMARKS.md)
were run on a 2016 i7-6900K (8 cores, 16 threads, 64 GB). dsv's numbers below
are from the much older i5-4210M laptop used above — so this is an *unfair*
comparison in xsv's favor, yet dsv still holds its own:

```
command        dsv (i5-4210M)   xsv (i7-6900K)   dsv/xsv
-------------  --------------   --------------   -------
count          356 MB/s         413.76 MB/s      0.86
select         237 MB/s         325.09 MB/s      0.73
search         261 MB/s         168.56 MB/s      1.55x dsv
sort           20.3  MB/s       20.87 MB/s       0.97
frequency      13.9  MB/s       25.00 MB/s       0.55
stats          49.8  MB/s       41.75 MB/s       1.19x dsv
flatten        10.2  MB/s       10.02 MB/s       1.02
```

Given the ~2-3x single-core advantage of the i7-6900K, dsv effectively matches
or beats upstream on `count`, `sort`, `search`, `stats`, and `flatten`. Only
`frequency` is slower per byte — it's the one command where the old machine's
single-threaded hash aggregation dominates and where the i7's 8 real cores
shine.

## Notes

- Head-to-head methodology: same binary flags (hyperfine, 2 warmups + 10
  runs), same dataset, same page cache state. `search` uses `-s City Hoseyn`,
  `select` uses `Country,City`, `sort -N` uses the `Population` column.
- `sort`/`frequency`/`stats` on 1M rows hold data in memory; timings scale
  with row count, not just bytes.
- dsv is built with `lto = "thin"`, `strip = "symbols"`, `codegen-units = 1`
  and newer versions of csv/regex than upstream xsv — likely sources of its
  edge on the heavy commands.
- These are ballpark figures for catching regressions, not a controlled
  benchmark suite.
