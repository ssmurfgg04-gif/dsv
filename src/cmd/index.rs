use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use csv_index::RandomAccessSimple;

use crate::CliResult;
use crate::config::{Config, Delimiter};
use crate::util;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
#[arg()]
    pub arg_input: String,
#[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
#[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {

    let pidx = match &args.flag_output {
        None => util::idx_path(&Path::new(&args.arg_input.clone())),
        Some(p) => PathBuf::from(&p),
    };

    let rconfig = Config::new(&Some(args.arg_input.clone()))
                         .delimiter(args.flag_delimiter);
    let mut rdr = rconfig.reader_file()?;
    let mut wtr = io::BufWriter::new(fs::File::create(&pidx)?);
    RandomAccessSimple::create(&mut rdr, &mut wtr)?;
    Ok(())
}
