# dsv — Data Swiss Knife

[![CI](https://github.com/ssmurfgg04-gif/dsv/actions/workflows/ci.yml/badge.svg)](https://github.com/ssmurfgg04-gif/dsv/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/dsv.svg)](https://crates.io/crates/dsv)
[![docs.rs](https://docs.rs/dsv/badge.svg)](https://docs.rs/dsv)
[![License: MIT/Unlicense](https://img.shields.io/badge/license-MIT%2FUnlicense-blue.svg)](LICENSE-MIT)

A fast CSV, TSV, Parquet, and JSONL command line toolkit written in Rust.
Modern fork of [BurntSushi/xsv](https://github.com/BurntSushi/xsv) (10.8K ★), merging all 33 unmerged PRs with Parquet/JSONL support, new commands, and modern dependencies.

## Install

```bash
cargo install dsv
```

Or download a [pre-built binary](https://github.com/ssmurfgg04-gif/dsv/releases).

## Commands

| Command | Description |
|---------|-------------|
| `cat` | Concatenate CSV files by row or column |
| `convert` | Convert between CSV, TSV, JSONL, and Parquet |
| `count` | Count rows (instantaneous with index) |
| `dedup` | Remove duplicate rows |
| `exclude` | Exclude rows matching a regex pattern |
| `fill` | Forward-fill or back-fill empty values |
| `fixlengths` | Force records to same length |
| `flatten` | Show one field per line |
| `fmt` | Reformat CSV delimiters, quoting, terminators |
| `frequency` | Build frequency tables per column |
| `headers` | Show header names |
| `index` | Create fast access index |
| `input` | Read CSV with exotic quoting/escaping |
| `join` | Inner, outer, left, right joins |
| `partition` | Split data by column value |
| `rename` | Rename column headers |
| `reverse` | Reverse row order |
| `sample` | Randomly sample rows |
| `search` | Regex search across fields |
| `select` | Select or reorder columns |
| `slice` | Slice rows (constant-time with index) |
| `sort` | Sort data |
| `split` | Split into N-chunk files |
| `stats` | Per-column statistics (mean, stddev, median, etc.) |
| `table` | Aligned column output via elastic tabstops |
| `transpose` | Swap rows and columns |

## Quick examples

```bash
# Stats on a CSV
dsv stats data.csv --everything | dsv table

# Convert CSV to Parquet
dsv convert data.csv data.parquet

# Convert Parquet back to JSONL
dsv convert data.parquet data.jsonl

# Run stats directly on Parquet (no intermediate CSV)
dsv stats data.parquet

# Search CSV, pipe through table
dsv search --regex '\d{5}' data.csv | dsv table

# Forward-fill empty cells in a column
dsv fill --select Population data.csv

# Rename columns
dsv rename 'old_name:new_name' data.csv

# Transpose rows and columns
dsv transpose data.csv
```

## Why dsv over xsv, qsv, or xan?

**dsv fills the gap** between the archived xsv and the bloated forks:

| Feature | xsv (archived) | qsv | xan | **dsv** |
|---------|:---:|:---:|:---:|:---:|
| Maintained | ✗ | ✓ | ✓ | **✓** |
| Parquet support | ✗ | ✗ | ✗ | **✓** |
| JSONL support | ✗ | ✗ | ✗ | **✓** |
| 33 community PRs merged | ✗ | partial | partial | **✓** |
| Lean command set (26) | ✓ | ✗ (40+) | ✗ (30+) | **✓** |
| Rust edition 2021 | ✗ | ✓ | ✓ | **✓** |
| clap v4 CLI | ✗ | ✓ | ✗ | **✓** |
| MSRV 1.85 | ✗ | ✓ | ✓ | **✓** |

**xsv** was archived in 2023. **qsv** adds heavy SQL, networking, and pandas-like features — great for ETL but not a drop-in xsv replacement. **xan** focuses on data journalism with Python integrations. **dsv** is the lean, modern evolution of xsv: same philosophy, more formats, community features merged.

## Feature parity with xsv

Every xsv command works exactly as documented upstream — same flags, same output format — so dsv is a drop-in replacement:

| xsv command | dsv | Extras beyond xsv |
|:---|:---:|:---|
| `cat`, `count`, `fixlengths`, `flatten`, `fmt`, `frequency`, `headers`, `index`, `input`, `join`, `partition`, `reverse`, `sample`, `search`, `select`, `slice`, `sort`, `split`, `stats`, `table` | **✓** | |
| `convert` (CSV ⇄ TSV ⇄ JSONL ⇄ Parquet) | **✓** | new |
| `dedup` (remove duplicate rows) | **✓** | new |
| `exclude` (drop rows matching a regex) | **✓** | new |
| `fill` (forward/back-fill empty cells) | **✓** | new |
| `rename` (rename column headers) | **✓** | new |
| `transpose` (swap rows and columns) | **✓** | new |

In addition, all 33 community PRs that sat unmerged in xsv's upstream repository are merged here, and every command also accepts Parquet and JSONL input where it makes sense (e.g. `stats`, `select`, `search`).

## Whirlwind tour

```bash
# Download sample data
curl -LO https://burntsushi.net/stuff/worldcitiespop.csv

# Examine headers
dsv headers worldcitiespop.csv

# Get column statistics
dsv stats worldcitiespop.csv --everything | dsv table

# Create an index for faster access
dsv index worldcitiespop.csv

# Select columns and sample 10 random rows
dsv select Country,AccentCity,Population worldcitiespop.csv \
  | dsv sample 10 \
  | dsv table

# Frequency table
dsv frequency worldcitiespop.csv --limit 5

# Search for rows with population data
dsv search -s Population '[0-9]' worldcitiespop.csv \
  | dsv select Country,AccentCity,Population \
  | dsv table

# Convert to Parquet for analytics
dsv convert worldcitiespop.csv worldcitiespop.parquet

# Same stats, now on Parquet
dsv stats worldcitiespop.parquet --everything | dsv table
```

## JSONL notes

- Only `.jsonl` and `.ndjson` extensions are detected as JSONL. Plain `.json` files are treated as CSV — JSON (non line-delimited) files are **not** supported as input.
- JSON objects are unordered maps. When reading JSONL, columns are derived from the first record's keys; records with extra keys extend the column set. When writing JSONL, keys are serialized in the order the columns were read (CSV header order, or insertion order for JSONL→JSONL).
- To round-trip reliably between CSV and JSONL, keep column names unique and stable. Empty and non-UTF-8 fields are handled as JSON `null`.

## Gzip support

Any `.gz` input file is transparently decompressed, and writing to a `.gz` path transparently compresses. This works for every command (`cat`, `select`, `search`, `stats`, `convert`, …) and across all formats (CSV, TSV, JSONL, Parquet):

```bash
# Read gzipped CSV
dsv stats data.csv.gz --everything | dsv table

# Write gzipped output
dsv select name,age data.csv -o data.csv.gz

# Convert gzipped CSV to JSONL
dsv convert data.csv.gz data.jsonl
```

Gzip is detected purely by the `.gz` extension (e.g. `data.tsv.gz` still uses tab delimiters). Reading from stdin and writing to stdout behave as before.

## Performance

dsv inherits xsv's zero-compromise performance model: constant-time indexing, streaming row processing, and minimal memory overhead. On the exact dataset used in xsv's original benchmarks, dsv beats upstream xsv on the same hardware on `sort`, numeric `sort`, `frequency`, and `stats` (up to 48% faster), and ties on `count`/`select`. Parquet reads leverage Apache Arrow's columnar format for fast predicate pushdown and vectorized decoding.

Benchmarks are maintained at [BENCHMARKS.md](./BENCHMARKS.md).

## Motivation

When handed a 40GB CSV file, existing tools were too slow or inflexible. xsv solved this with indexing, slicing, and composable commands. dsv extends that foundation to modern data formats (Parquet, JSONL) while merging community contributions that sat unmerged for years. If you work with tabular data on the command line, dsv is the tool xsv should have become.

## License

Dual-licensed under MIT or [UNLICENSE](https://unlicense.org/).

dsv is a fork of [BurntSushi/xsv](https://github.com/BurntSushi/xsv) by Andrew Gallant (BurntSushi).
