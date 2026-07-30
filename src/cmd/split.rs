use std::fs;
use std::io;
use std::path::Path;

use crossbeam_channel;
use csv;
use threadpool::ThreadPool;

use crate::CliResult;
use crate::config::{Config, Delimiter};
use crate::index::Indexed;
use crate::util::{self, FilenameTemplate};
use clap::Parser;

#[derive(Parser, Clone, Debug)]
pub struct Args {
#[arg()]
    pub arg_outdir: String,
#[arg()]
    pub arg_input: Option<String>,
#[arg(long = "size", value_name = "arg")]
    pub flag_size: usize,
#[arg(short = 'j', long = "jobs", value_name = "arg", default_value_t = 0)]
    pub flag_jobs: usize,
#[arg(long = "filename", value_name = "arg", default_value = "{}.csv")]
    pub flag_filename: FilenameTemplate,
#[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
#[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    if args.flag_size == 0 {
        return fail!("--size must be greater than 0.");
    }
    fs::create_dir_all(&args.arg_outdir)?;

    match args.rconfig().indexed()? {
        Some(idx) => args.parallel_split(idx),
        None => args.sequential_split(),
    }
}

impl Args {
    fn sequential_split(&self) -> CliResult<()> {
        let rconfig = self.rconfig();
        let mut rdr = rconfig.reader()?;
        let headers = rdr.byte_headers()?.clone();

        let mut wtr = self.new_writer(&headers, 0)?;
        let mut i = 0;
        let mut row = csv::ByteRecord::new();
        while rdr.read_byte_record(&mut row)? {
            if i > 0 && i % self.flag_size == 0 {
                wtr.flush()?;
                wtr = self.new_writer(&headers, i)?;
            }
            wtr.write_byte_record(&row)?;
            i += 1;
        }
        wtr.flush()?;
        Ok(())
    }

    fn parallel_split(
        &self,
        idx: Indexed<fs::File, fs::File>,
    ) -> CliResult<()> {
        let nchunks = util::num_of_chunks(
            idx.count() as usize, self.flag_size);
        let pool = ThreadPool::new(self.njobs());
        let (tx, rx) = crossbeam_channel::bounded::<()>(0);
        for i in 0..nchunks {
            let args = self.clone();
            let tx = tx.clone();
            pool.execute(move || {
                let conf = args.rconfig();
                let mut idx = conf.indexed().unwrap().unwrap();
                let headers = idx.byte_headers().unwrap().clone();
                let mut wtr = args
                    .new_writer(&headers, i * args.flag_size)
                    .unwrap();

                idx.seek((i * args.flag_size) as u64).unwrap();
                for row in idx.byte_records().take(args.flag_size) {
                    let row = row.unwrap();
                    wtr.write_byte_record(&row).unwrap();
                }
                wtr.flush().unwrap();
                drop(tx);
            });
        }
        drop(tx);
        let _ = rx.recv();
        Ok(())
    }

    fn new_writer(
        &self,
        headers: &csv::ByteRecord,
        start: usize,
    ) -> CliResult<csv::Writer<Box<dyn io::Write+'static>>> {
        let dir = Path::new(&self.arg_outdir);
        let path = dir.join(self.flag_filename.filename(&format!("{}", start)));
        let spath = Some(path.display().to_string());
        let mut wtr = Config::new(&spath).writer()?;
        if !self.rconfig().no_headers {
            wtr.write_record(headers)?;
        }
        Ok(wtr)
    }

    fn rconfig(&self) -> Config {
        Config::new(&self.arg_input)
            .delimiter(self.flag_delimiter)
            .no_headers(self.flag_no_headers)
    }

    fn njobs(&self) -> usize {
        if self.flag_jobs == 0 {
            num_cpus::get()
        } else {
            self.flag_jobs
        }
    }
}
