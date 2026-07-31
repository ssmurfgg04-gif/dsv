use std::borrow::Cow;

use csv;
use tabwriter::TabWriter;

use crate::config::{Config, Delimiter};
use crate::util;
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'w', long = "width", value_name = "arg", default_value_t = 1)]
    pub flag_width: usize,
    #[arg(short = 'p', long = "pad", default_value_t = 2)]
    pub flag_pad: usize,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
    #[arg(short = 'c', long = "condense", value_name = "arg")]
    pub flag_condense: Option<usize>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(true);
    let wconfig = Config::new(&args.flag_output).delimiter(Some(Delimiter(b'\t')));

    let tw = TabWriter::new(wconfig.io_writer()?)
        .minwidth(args.flag_width)
        .padding(args.flag_pad);
    let mut wtr = wconfig.build_writer(tw);
    let mut rdr = rconfig.reader()?;

    let mut record = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut record)? {
        wtr.write_record(
            record
                .iter()
                .map(|f| util::condense(Cow::Borrowed(f), args.flag_condense)),
        )?;
    }
    wtr.flush()?;
    Ok(())
}
