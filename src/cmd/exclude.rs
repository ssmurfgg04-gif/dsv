use regex::bytes::RegexBuilder;
use crate::CliResult;
use crate::config::{Config, Delimiter};
use crate::select::SelectColumns;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_regex: String,
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
    #[arg(short = 'i', long = "ignore-case")]
    pub flag_ignore_case: bool,
}

pub fn run(args: &Args) -> CliResult<()> {
    let pattern = RegexBuilder::new(&*args.arg_regex)
        .case_insensitive(args.flag_ignore_case)
        .build()?;
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .select(args.flag_select.clone());
    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let headers = rdr.byte_headers()?.clone();
    let sel = rconfig.selection(&headers)?;

    if !rconfig.no_headers { wtr.write_record(&headers)?; }
    let mut record = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut record)? {
        let matched = sel.select(&record).any(|f| pattern.is_match(f));
        if !matched { wtr.write_byte_record(&record)?; }
    }
    Ok(wtr.flush()?)
}
