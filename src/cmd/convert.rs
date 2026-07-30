use crate::CliResult;
use crate::config::Config;
use crate::data::{DataWriter, Format};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: String,
    #[arg()]
    pub arg_output: String,
    #[arg(short = 'f', long = "from")]
    pub flag_from: Option<String>,
    #[arg(short = 't', long = "to")]
    pub flag_to: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<crate::config::Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let in_fmt = match &args.flag_from {
        Some(f) => Format::from_path(f),
        None => Format::from_path(&args.arg_input),
    };
    let out_fmt = match &args.flag_to {
        Some(f) => Format::from_path(f),
        None => Format::from_path(&args.arg_output),
    };

    let delim = args.flag_delimiter.map(|d| d.as_byte());
    let mut rdr = Config::new(&Some(args.arg_input.clone()))
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .data_reader(in_fmt)?;
    let headers = if !args.flag_no_headers { rdr.headers().ok() } else { None };

    let mut wtr = DataWriter::from_path(&args.arg_output, out_fmt, delim)?;
    if let Some(ref h) = headers { wtr.write_headers(h)?; }
    let mut rec = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? { wtr.write_byte_record(&rec)?; }
    wtr.flush()?;
    Ok(())
}
