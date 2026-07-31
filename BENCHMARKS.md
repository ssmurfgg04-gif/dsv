These are some very basic and unscientific benchmarks of various commands
provided by `dsv`. Please see below for more information.

The dataset is a synthetic 1,000,000-row CSV (23.78 MB) with 3 columns
(`country`, `city`, `population`), generated for benchmarking. Results were
measured with `dsv 0.13.0` compiled in `--release` mode (thin LTO, stripped).

Benchmarks were run on an Intel i5-4210M (2 cores, 4 threads) with 16GB of
RAM, a mechanical HDD, and the input file cached in page cache.

```
count                   0.072  secs   330.3  MB/sec
search                  0.139  secs   171.1  MB/sec
select                  0.193  secs   123.2  MB/sec
transpose               0.453  secs    52.5  MB/sec
stats --median          0.807  secs    29.5  MB/sec
convert -> parquet      1.988  secs    12.0  MB/sec
dedup                   2.055  secs    11.6  MB/sec
frequency               4.645  secs     5.1  MB/sec
sort                    4.956  secs     4.8  MB/sec
convert -> jsonl        8.687  secs     2.7  MB/sec
```

Parquet round-trip: `convert bench.csv bench.parquet` produces a 26.5 MB
file; converting it back to CSV yields identical rows (verified with
`cmp` after normalizing line endings).

### Notes

- `count`, `select`, and `search` are the streaming baselines: near-linear
  scans with constant memory. `count` is the fastest possible command that
  parses every record.
- `sort`, `dedup`, and `frequency` must hold state (or spill) and are
  therefore slower per byte; this matches xsv's original behavior.
- `convert` to Parquet benefits from Arrow's columnar encoding; JSONL output
  is write-bound (one JSON object per line).
- These are ballpark figures for catching regressions, not a controlled
  benchmark. For xsv-era comparisons, see the upstream
  [xsv benchmark](https://github.com/BurntSushi/xsv/blob/master/BENCHMARKS.md).
