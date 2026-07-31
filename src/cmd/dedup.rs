use crate::config::{Config, Delimiter};
use crate::select::SelectColumns;
use crate::CliResult;
use clap::Parser;
use csv::ByteRecord;
use std::collections::HashSet;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 's', long = "select", default_value = "")]
    pub flag_select: SelectColumns,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .select(args.flag_select.clone());
    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let headers = rdr.byte_headers()?.clone();
    let sel = rconfig.selection(&headers)?;

    if !rconfig.no_headers {
        wtr.write_record(&headers)?;
    }

    let mut seen: HashSet<Vec<Vec<u8>>> = HashSet::new();
    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? {
        let key: Vec<Vec<u8>> = sel.select(&rec).map(|f| f.to_vec()).collect();
        if seen.insert(key) {
            wtr.write_byte_record(&rec)?;
        }
    }
    wtr.flush()?;
    Ok(())
}
