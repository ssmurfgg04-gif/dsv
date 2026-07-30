use std::fs;


use crate::CliResult;
use crate::config::{Config, Delimiter};
use crate::index::Indexed;
use crate::util;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
#[arg()]
    pub arg_input: Option<String>,
#[arg(short = 's', long = "start", value_name = "arg")]
    pub flag_start: Option<usize>,
#[arg(short = 'e', long = "end", value_name = "arg")]
    pub flag_end: Option<usize>,
#[arg(short = 'l', long = "len", value_name = "arg")]
    pub flag_len: Option<usize>,
#[arg(short = 'i', long = "index", value_name = "arg")]
    pub flag_index: Option<usize>,
#[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
#[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
#[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    match args.rconfig().indexed()? {
        None => args.no_index(),
        Some(idxed) => args.with_index(idxed),
    }
}

impl Args {
    fn no_index(&self) -> CliResult<()> {
        let mut rdr = self.rconfig().reader()?;
        let mut wtr = self.wconfig().writer()?;
        self.rconfig().write_headers(&mut rdr, &mut wtr)?;

        let (start, end) = self.range()?;
        for r in rdr.byte_records().skip(start).take(end - start) {
            wtr.write_byte_record(&r?)?;
        }
        Ok(wtr.flush()?)
    }

    fn with_index(
        &self,
        mut idx: Indexed<fs::File, fs::File>,
    ) -> CliResult<()> {
        let mut wtr = self.wconfig().writer()?;
        self.rconfig().write_headers(&mut *idx, &mut wtr)?;

        let (start, end) = self.range()?;
        if end - start == 0 {
            return Ok(());
        }
        idx.seek(start as u64)?;
        for r in idx.byte_records().take(end - start) {
            wtr.write_byte_record(&r?)?;
        }
        wtr.flush()?;
        Ok(())
    }

    fn range(&self) -> Result<(usize, usize), String> {
        util::range(
            self.flag_start, self.flag_end, self.flag_len, self.flag_index)
    }

    fn rconfig(&self) -> Config {
        Config::new(&self.arg_input)
            .delimiter(self.flag_delimiter)
            .no_headers(self.flag_no_headers)
    }

    fn wconfig(&self) -> Config {
        Config::new(&self.flag_output)
    }
}
