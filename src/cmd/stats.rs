use std::borrow::ToOwned;
use std::default::Default;
use std::fmt;
use std::fs;
use std::io;
use std::iter::FromIterator;
use std::str::{self, FromStr};

use crossbeam_channel;
use csv;
use stats::{merge_all, Commute, MinMax, OnlineStats, Unsorted};
use threadpool::ThreadPool;

use crate::config::{Config, Delimiter};
use crate::index::Indexed;
use crate::select::{SelectColumns, Selection};
use crate::util;
use crate::CliResult;

use self::FieldType::{Float, Integer, Null, Unicode, Unknown};
use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 's', long = "select", default_value = "")]
    pub flag_select: SelectColumns,
    #[arg(long = "everything")]
    pub flag_everything: bool,
    #[arg(long = "mode")]
    pub flag_mode: bool,
    #[arg(long = "cardinality")]
    pub flag_cardinality: bool,
    #[arg(long = "median")]
    pub flag_median: bool,
    #[arg(long = "nulls")]
    pub flag_nulls: bool,
    #[arg(short = 'j', long = "jobs", value_name = "arg", default_value_t = 0)]
    pub flag_jobs: usize,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let mut wtr = Config::new(&args.flag_output).writer()?;
    let (headers, stats) = match args.rconfig().indexed()? {
        None => args.sequential_stats(),
        Some(idx) => {
            if args.flag_jobs == 1 {
                args.sequential_stats()
            } else {
                args.parallel_stats(idx)
            }
        }
    }?;
    let stats = args.stats_to_records(stats);

    wtr.write_record(&args.stat_headers())?;
    let fields = headers.iter().zip(stats);
    for (i, (header, stat)) in fields.enumerate() {
        let header = if args.flag_no_headers {
            i.to_string().into_bytes()
        } else {
            header.to_vec()
        };
        let stat = stat.iter().map(|f| f.as_bytes());
        wtr.write_record(vec![&*header].into_iter().chain(stat))?;
    }
    wtr.flush()?;
    Ok(())
}

impl Args {
    fn sequential_stats(&self) -> CliResult<(csv::ByteRecord, Vec<Stats>)> {
        let mut rdr = self.rconfig().reader()?;
        let (headers, sel) = self.sel_headers(&mut rdr)?;
        let stats = self.compute(&sel, rdr.byte_records())?;
        Ok((headers, stats))
    }

    fn parallel_stats(
        &self,
        idx: Indexed<fs::File, fs::File>,
    ) -> CliResult<(csv::ByteRecord, Vec<Stats>)> {
        // N.B. This method doesn't handle the case when the number of records
        // is zero correctly. So we use `sequential_stats` instead.
        if idx.count() == 0 {
            return self.sequential_stats();
        }

        let mut rdr = self.rconfig().reader()?;
        let (headers, sel) = self.sel_headers(&mut rdr)?;

        let chunk_size = util::chunk_size(idx.count() as usize, self.njobs());
        let nchunks = util::num_of_chunks(idx.count() as usize, chunk_size);

        let pool = ThreadPool::new(self.njobs());
        let (send, recv) = crossbeam_channel::bounded(0);
        for i in 0..nchunks {
            let (send, args, sel) = (send.clone(), self.clone(), sel.clone());
            pool.execute(move || {
                let mut idx = args.rconfig().indexed().unwrap().unwrap();
                idx.seek((i * chunk_size) as u64).unwrap();
                let it = idx.byte_records().take(chunk_size);
                let _ = send.send(args.compute(&sel, it).unwrap());
            });
        }
        drop(send);
        Ok((
            headers,
            merge_all(recv.into_iter()).unwrap_or_else(Vec::new),
        ))
    }

    fn stats_to_records(&self, stats: Vec<Stats>) -> Vec<csv::StringRecord> {
        let mut records: Vec<_> =
            std::iter::repeat_n(csv::StringRecord::new(), stats.len()).collect();
        let pool = ThreadPool::new(self.njobs());
        let mut results = vec![];
        for mut stat in stats.into_iter() {
            let (send, recv) = crossbeam_channel::bounded(0);
            results.push(recv);
            pool.execute(move || {
                let _ = send.send(stat.to_record());
            });
        }
        for (i, recv) in results.into_iter().enumerate() {
            records[i] = recv.recv().unwrap();
        }
        records
    }

    fn compute<I>(&self, sel: &Selection, it: I) -> CliResult<Vec<Stats>>
    where
        I: Iterator<Item = csv::Result<csv::ByteRecord>>,
    {
        let mut stats = self.new_stats(sel.len());
        for row in it {
            let row = row?;
            for (i, field) in sel.select(&row).enumerate() {
                stats[i].add(field);
            }
        }
        Ok(stats)
    }

    fn sel_headers<R: io::Read>(
        &self,
        rdr: &mut csv::Reader<R>,
    ) -> CliResult<(csv::ByteRecord, Selection)> {
        let headers = rdr.byte_headers()?.clone();
        let sel = self.rconfig().selection(&headers)?;
        Ok((csv::ByteRecord::from_iter(sel.select(&headers)), sel))
    }

    fn rconfig(&self) -> Config {
        Config::new(&self.arg_input)
            .delimiter(self.flag_delimiter)
            .no_headers(self.flag_no_headers)
            .select(self.flag_select.clone())
    }

    fn njobs(&self) -> usize {
        if self.flag_jobs == 0 {
            num_cpus::get()
        } else {
            self.flag_jobs
        }
    }

    fn new_stats(&self, record_len: usize) -> Vec<Stats> {
        std::iter::repeat_n(
            Stats::new(WhichStats {
                include_nulls: self.flag_nulls,
                sum: true,
                range: true,
                dist: true,
                cardinality: self.flag_cardinality || self.flag_everything,
                median: self.flag_median || self.flag_everything,
                mode: self.flag_mode || self.flag_everything,
            }),
            record_len,
        )
        .collect()
    }

    fn stat_headers(&self) -> csv::StringRecord {
        let mut fields = vec![
            "field",
            "type",
            "sum",
            "min",
            "max",
            "min_length",
            "max_length",
            "mean",
            "stddev",
        ];
        let all = self.flag_everything;
        if self.flag_median || all {
            fields.push("median");
        }
        if self.flag_mode || all {
            fields.push("mode");
        }
        if self.flag_cardinality || all {
            fields.push("cardinality");
        }
        csv::StringRecord::from(fields)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WhichStats {
    include_nulls: bool,
    sum: bool,
    range: bool,
    dist: bool,
    cardinality: bool,
    median: bool,
    mode: bool,
}

impl Commute for WhichStats {
    fn merge(&mut self, other: WhichStats) {
        assert_eq!(*self, other);
    }
}

#[derive(Clone)]
struct Stats {
    typ: FieldType,
    sum: Option<TypedSum>,
    minmax: Option<TypedMinMax>,
    online: Option<OnlineStats>,
    mode: Option<Unsorted<Vec<u8>>>,
    median: Option<Unsorted<f64>>,
    which: WhichStats,
}

impl Stats {
    fn new(which: WhichStats) -> Stats {
        let (mut sum, mut minmax, mut online, mut mode, mut median) =
            (None, None, None, None, None);
        if which.sum {
            sum = Some(Default::default());
        }
        if which.range {
            minmax = Some(Default::default());
        }
        if which.dist {
            online = Some(Default::default());
        }
        if which.mode || which.cardinality {
            mode = Some(Default::default());
        }
        if which.median {
            median = Some(Default::default());
        }
        Stats {
            typ: Default::default(),
            sum,
            minmax,
            online,
            mode,
            median,
            which,
        }
    }

    fn add(&mut self, sample: &[u8]) {
        let sample_type = FieldType::from_sample(sample);
        self.typ.merge(sample_type);

        let t = self.typ;
        if let Some(v) = self.sum.as_mut() {
            v.add(t, sample)
        }
        if let Some(v) = self.minmax.as_mut() {
            v.add(t, sample)
        }
        if let Some(v) = self.mode.as_mut() {
            v.add(sample.to_vec())
        }
        match self.typ {
            Unknown => {}
            Null => {
                if self.which.include_nulls {
                    if let Some(v) = self.online.as_mut() {
                        v.add_null();
                    }
                }
            }
            Unicode => {}
            Float | Integer => {
                if sample_type.is_null() {
                    if self.which.include_nulls {
                        if let Some(v) = self.online.as_mut() {
                            v.add_null();
                        }
                    }
                } else {
                    let n = from_bytes::<f64>(sample).unwrap();
                    if let Some(v) = self.median.as_mut() {
                        v.add(n);
                    }
                    if let Some(v) = self.online.as_mut() {
                        v.add(n);
                    }
                }
            }
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_record(&mut self) -> csv::StringRecord {
        let typ = self.typ;
        let mut pieces = vec![];
        let empty = || "".to_owned();

        pieces.push(self.typ.to_string());
        match self.sum.as_ref().and_then(|sum| sum.show(typ)) {
            Some(sum) => {
                pieces.push(sum);
            }
            None => {
                pieces.push(empty());
            }
        }
        match self.minmax.as_ref().and_then(|mm| mm.show(typ)) {
            Some(mm) => {
                pieces.push(mm.0);
                pieces.push(mm.1);
            }
            None => {
                pieces.push(empty());
                pieces.push(empty());
            }
        }
        match self.minmax.as_ref().and_then(|mm| mm.len_range()) {
            Some(mm) => {
                pieces.push(mm.0);
                pieces.push(mm.1);
            }
            None => {
                pieces.push(empty());
                pieces.push(empty());
            }
        }

        if !self.typ.is_number() {
            pieces.push(empty());
            pieces.push(empty());
        } else {
            match self.online {
                Some(ref v) => {
                    pieces.push(v.mean().to_string());
                    pieces.push(v.stddev().to_string());
                }
                None => {
                    pieces.push(empty());
                    pieces.push(empty());
                }
            }
        }
        match self.median.as_mut().and_then(|v| v.median()) {
            None => {
                if self.which.median {
                    pieces.push(empty());
                }
            }
            Some(v) => {
                pieces.push(v.to_string());
            }
        }
        match self.mode.as_mut() {
            None => {
                if self.which.mode {
                    pieces.push(empty());
                }
                if self.which.cardinality {
                    pieces.push(empty());
                }
            }
            Some(ref mut v) => {
                if self.which.mode {
                    let lossy = |s: Vec<u8>| -> String { String::from_utf8_lossy(&s).into_owned() };
                    pieces.push(v.mode().map_or("N/A".to_owned(), lossy));
                }
                if self.which.cardinality {
                    pieces.push(v.cardinality().to_string());
                }
            }
        }
        csv::StringRecord::from(pieces)
    }
}

impl Commute for Stats {
    fn merge(&mut self, other: Stats) {
        self.typ.merge(other.typ);
        self.sum.merge(other.sum);
        self.minmax.merge(other.minmax);
        self.online.merge(other.online);
        self.mode.merge(other.mode);
        self.median.merge(other.median);
        self.which.merge(other.which);
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
enum FieldType {
    Unknown,
    #[default]
    Null,
    Unicode,
    Float,
    Integer,
}

impl FieldType {
    fn from_sample(sample: &[u8]) -> FieldType {
        if sample.is_empty() {
            return Null;
        }
        let string = match str::from_utf8(sample) {
            Err(_) => return Unknown,
            Ok(s) => s,
        };
        if string.parse::<i64>().is_ok() {
            return Integer;
        }
        if string.parse::<f64>().is_ok() {
            return Float;
        }
        Unicode
    }

    fn is_number(&self) -> bool {
        *self == Float || *self == Integer
    }

    fn is_null(&self) -> bool {
        *self == Null
    }
}

impl Commute for FieldType {
    fn merge(&mut self, other: FieldType) {
        *self = match (*self, other) {
            (Unicode, Unicode) => Unicode,
            (Float, Float) => Float,
            (Integer, Integer) => Integer,
            // Null does not impact the type.
            (Null, any) | (any, Null) => any,
            // There's no way to get around an unknown.
            (Unknown, _) | (_, Unknown) => Unknown,
            // Integers can degrate to floats.
            (Float, Integer) | (Integer, Float) => Float,
            // Numbers can degrade to Unicode strings.
            (Unicode, Float) | (Float, Unicode) => Unicode,
            (Unicode, Integer) | (Integer, Unicode) => Unicode,
        };
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Unknown => write!(f, "Unknown"),
            Null => write!(f, "NULL"),
            Unicode => write!(f, "Unicode"),
            Float => write!(f, "Float"),
            Integer => write!(f, "Integer"),
        }
    }
}

/// TypedSum keeps a rolling sum of the data seen.
///
/// It sums integers until it sees a float, at which point it sums floats.
#[derive(Clone, Default)]
struct TypedSum {
    integer: i64,
    float: Option<f64>,
}

impl TypedSum {
    fn add(&mut self, typ: FieldType, sample: &[u8]) {
        if sample.is_empty() {
            return;
        }
        match typ {
            Float => {
                let float: f64 = from_bytes::<f64>(sample).unwrap();
                match self.float {
                    None => {
                        self.float = Some((self.integer as f64) + float);
                    }
                    Some(ref mut f) => {
                        *f += float;
                    }
                }
            }
            Integer => {
                if let Some(ref mut float) = self.float {
                    *float += from_bytes::<f64>(sample).unwrap();
                } else {
                    self.integer += from_bytes::<i64>(sample).unwrap();
                }
            }
            _ => {}
        }
    }

    fn show(&self, typ: FieldType) -> Option<String> {
        match typ {
            Null | Unicode | Unknown => None,
            Integer => Some(self.integer.to_string()),
            Float => Some(self.float.unwrap_or(0.0).to_string()),
        }
    }
}

impl Commute for TypedSum {
    fn merge(&mut self, other: TypedSum) {
        match (self.float, other.float) {
            (Some(f1), Some(f2)) => self.float = Some(f1 + f2),
            (Some(f1), None) => self.float = Some(f1 + (other.integer as f64)),
            (None, Some(f2)) => self.float = Some((self.integer as f64) + f2),
            (None, None) => self.integer += other.integer,
        }
    }
}

/// TypedMinMax keeps track of minimum/maximum values for each possible type
/// where min/max makes sense.
#[derive(Clone, Default)]
struct TypedMinMax {
    strings: MinMax<Vec<u8>>,
    str_len: MinMax<usize>,
    integers: MinMax<i64>,
    floats: MinMax<f64>,
}

impl TypedMinMax {
    fn add(&mut self, typ: FieldType, sample: &[u8]) {
        self.str_len.add(sample.len());
        if sample.is_empty() {
            return;
        }
        self.strings.add(sample.to_vec());
        match typ {
            Unicode | Unknown | Null => {}
            Float => {
                let n = str::from_utf8(sample)
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap();
                self.floats.add(n);
                self.integers.add(n as i64);
            }
            Integer => {
                let n = str::from_utf8(sample)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap();
                self.integers.add(n);
                self.floats.add(n as f64);
            }
        }
    }

    fn len_range(&self) -> Option<(String, String)> {
        match (self.str_len.min(), self.str_len.max()) {
            (Some(min), Some(max)) => Some((min.to_string(), max.to_string())),
            _ => None,
        }
    }

    fn show(&self, typ: FieldType) -> Option<(String, String)> {
        match typ {
            Null => None,
            Unicode | Unknown => match (self.strings.min(), self.strings.max()) {
                (Some(min), Some(max)) => {
                    let min = String::from_utf8_lossy(min).to_string();
                    let max = String::from_utf8_lossy(max).to_string();
                    Some((min, max))
                }
                _ => None,
            },
            Integer => match (self.integers.min(), self.integers.max()) {
                (Some(min), Some(max)) => Some((min.to_string(), max.to_string())),
                _ => None,
            },
            Float => match (self.floats.min(), self.floats.max()) {
                (Some(min), Some(max)) => Some((min.to_string(), max.to_string())),
                _ => None,
            },
        }
    }
}

impl Commute for TypedMinMax {
    fn merge(&mut self, other: TypedMinMax) {
        self.strings.merge(other.strings);
        self.str_len.merge(other.str_len);
        self.integers.merge(other.integers);
        self.floats.merge(other.floats);
    }
}

fn from_bytes<T: FromStr>(bytes: &[u8]) -> Option<T> {
    str::from_utf8(bytes).ok().and_then(|s| s.parse().ok())
}
