use csv;

use crate::config::{Config, Delimiter};
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let conf = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers);

    let count = match conf.indexed()? {
        Some(idx) => idx.count(),
        None => {
            let mut rdr = conf.reader()?;
            let mut count = 0u64;
            let mut record = csv::ByteRecord::new();
            while rdr.read_byte_record(&mut record)? {
                count += 1;
            }
            count
        }
    };
    println!("{}", count);
    Ok(())
}
