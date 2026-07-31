use csv;

use crate::config::{Config, Delimiter};
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
    #[arg(long = "quote", value_name = "arg", default_value = "\"")]
    pub flag_quote: Delimiter,
    #[arg(long = "escape", value_name = "arg")]
    pub flag_escape: Option<Delimiter>,
    #[arg(long = "no-quoting")]
    pub flag_no_quoting: bool,
}

pub fn run(args: &Args) -> CliResult<()> {
    let mut rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(true)
        .quote(args.flag_quote.as_byte());
    let wconfig = Config::new(&args.flag_output);

    if let Some(escape) = args.flag_escape {
        rconfig = rconfig.escape(Some(escape.as_byte())).double_quote(false);
    }
    if args.flag_no_quoting {
        rconfig = rconfig.quoting(false);
    }

    let mut rdr = rconfig.reader()?;
    let mut wtr = wconfig.writer()?;
    let mut row = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut row)? {
        wtr.write_record(&row)?;
    }
    wtr.flush()?;
    Ok(())
}
