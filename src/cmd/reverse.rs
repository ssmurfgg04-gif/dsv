use crate::CliResult;
use crate::config::{Config, Delimiter};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
#[arg()]
    pub arg_input: Option<String>,
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
        .no_headers(args.flag_no_headers);

    let mut rdr = rconfig.reader()?;

    let mut all = rdr.byte_records().collect::<Result<Vec<_>, _>>()?;
    all.reverse();

    let mut wtr = Config::new(&args.flag_output).writer()?;
    rconfig.write_headers(&mut rdr, &mut wtr)?;
    for r in all.into_iter() {
        wtr.write_byte_record(&r)?;
    }
    Ok(wtr.flush()?)
}
