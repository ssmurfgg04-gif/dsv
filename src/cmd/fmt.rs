use csv;

use crate::config::{Config, Delimiter};
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 't', long = "out-delimiter", value_name = "arg")]
    pub flag_out_delimiter: Option<Delimiter>,
    #[arg(long = "crlf")]
    pub flag_crlf: bool,
    #[arg(long = "ascii")]
    pub flag_ascii: bool,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
    #[arg(long = "quote", value_name = "arg", default_value = "\"")]
    pub flag_quote: Delimiter,
    #[arg(long = "quote-always")]
    pub flag_quote_always: bool,
    #[arg(long = "escape", value_name = "arg")]
    pub flag_escape: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(true);
    let mut wconfig = Config::new(&args.flag_output)
        .delimiter(args.flag_out_delimiter)
        .crlf(args.flag_crlf);

    if args.flag_ascii {
        wconfig = wconfig
            .delimiter(Some(Delimiter(b'\x1f')))
            .terminator(csv::Terminator::Any(b'\x1e'));
    }
    if args.flag_quote_always {
        wconfig = wconfig.quote_style(csv::QuoteStyle::Always);
    }
    if let Some(escape) = args.flag_escape {
        wconfig = wconfig.escape(Some(escape.as_byte())).double_quote(false);
    }
    wconfig = wconfig.quote(args.flag_quote.as_byte());

    let mut rdr = rconfig.reader()?;
    let mut wtr = wconfig.writer()?;
    let mut r = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut r)? {
        wtr.write_byte_record(&r)?;
    }
    wtr.flush()?;
    Ok(())
}
