use std::fs;
use std::io;

use crossbeam_channel;
use csv;
use stats::{Frequencies, merge_all};
use threadpool::ThreadPool;

use crate::CliResult;
use crate::config::{Config, Delimiter};
use crate::index::Indexed;
use crate::select::{SelectColumns, Selection};
use crate::util;
use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct Args {
#[arg()]
    pub arg_input: Option<String>,
#[arg(short = 's', long = "select", default_value = "")]
    pub flag_select: SelectColumns,
#[arg(long = "limit", value_name = "arg", default_value_t = 10)]
    pub flag_limit: usize,
#[arg(long = "asc")]
    pub flag_asc: bool,
#[arg(long = "no-nulls")]
    pub flag_no_nulls: bool,
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
    let rconfig = args.rconfig();

    let mut wtr = Config::new(&args.flag_output).writer()?;
    let (headers, tables) = match args.rconfig().indexed()? {
        Some(ref mut idx) if args.njobs() > 1 => args.parallel_ftables(idx),
        _ => args.sequential_ftables(),
    }?;

    wtr.write_record(vec!["field", "value", "count"])?;
    let head_ftables = headers.into_iter().zip(tables.into_iter());
    for (i, (header, ftab)) in head_ftables.enumerate() {
        let mut header = header.to_vec();
        if rconfig.no_headers {
            header = (i+1).to_string().into_bytes();
        }
        for (value, count) in args.counts(&ftab).into_iter() {
            let count = count.to_string();
            let row = vec![&*header, &*value, count.as_bytes()];
            wtr.write_record(row)?;
        }
    }
    Ok(())
}

type ByteString = Vec<u8>;
type Headers = csv::ByteRecord;
type FTable = Frequencies<Vec<u8>>;
type FTables = Vec<Frequencies<Vec<u8>>>;

impl Args {
    fn rconfig(&self) -> Config {
        Config::new(&self.arg_input)
            .delimiter(self.flag_delimiter)
            .no_headers(self.flag_no_headers)
            .select(self.flag_select.clone())
    }

    fn counts(&self, ftab: &FTable) -> Vec<(ByteString, u64)> {
        let mut counts = if self.flag_asc {
            ftab.least_frequent()
        } else {
            ftab.most_frequent()
        };
        if self.flag_limit > 0 {
            counts = counts.into_iter().take(self.flag_limit).collect();
        }
        counts.into_iter().map(|(bs, c)| {
            if b"" == &**bs {
                (b"(NULL)"[..].to_vec(), c)
            } else {
                (bs.clone(), c)
            }
        }).collect()
    }

    fn sequential_ftables(&self) -> CliResult<(Headers, FTables)> {
        let mut rdr = self.rconfig().reader()?;
        let (headers, sel) = self.sel_headers(&mut rdr)?;
        Ok((headers, self.ftables(&sel, rdr.byte_records())?))
    }

    fn parallel_ftables(&self, idx: &mut Indexed<fs::File, fs::File>)
                       -> CliResult<(Headers, FTables)> {
        let mut rdr = self.rconfig().reader()?;
        let (headers, sel) = self.sel_headers(&mut rdr)?;

        if idx.count() == 0 {
            return Ok((headers, vec![]));
        }

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
                let _ = send.send(args.ftables(&sel, it).unwrap());
            });
        }
        drop(send);
        Ok((headers, merge_all(recv.into_iter()).unwrap()))
    }

    fn ftables<I>(&self, sel: &Selection, it: I) -> CliResult<FTables>
            where I: Iterator<Item=csv::Result<csv::ByteRecord>> {
        let null = &b""[..].to_vec();
        let nsel = sel.normal();
        let mut tabs: Vec<_> =
            (0..nsel.len()).map(|_| Frequencies::new()).collect();
        for row in it {
            let row = row?;
            for (i, field) in nsel.select(row.into_iter()).enumerate() {
                let field = trim(field.to_vec());
                if !field.is_empty() {
                    tabs[i].add(field);
                } else {
                    if !self.flag_no_nulls {
                        tabs[i].add(null.clone());
                    }
                }
            }
        }
        Ok(tabs)
    }

    fn sel_headers<R: io::Read>(&self, rdr: &mut csv::Reader<R>)
                  -> CliResult<(csv::ByteRecord, Selection)> {
        let headers = rdr.byte_headers()?;
        let sel = self.rconfig().selection(headers)?;
        Ok((sel.select(headers).map(|h| h.to_vec()).collect(), sel))
    }

    fn njobs(&self) -> usize {
        if self.flag_jobs == 0 { num_cpus::get() } else { self.flag_jobs }
    }
}

fn trim(bs: ByteString) -> ByteString {
    match String::from_utf8(bs) {
        Ok(s) => s.trim().as_bytes().to_vec(),
        Err(bs) => bs.into_bytes(),
    }
}
