use std::cmp;

use csv;

use crate::CliResult;
use crate::config::{Config, Delimiter};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
#[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'l', long = "length", value_name = "arg")]
    pub flag_length: Option<usize>,
#[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
#[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let config = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(true)
        .flexible(true);
    let length = match args.flag_length {
        Some(length) => {
            if length == 0 {
                return fail!("Length must be greater than 0.");
            }
            length
        }
        None => {
            if config.is_std() {
                return fail!("<stdin> cannot be used in this command. \
                              Please specify a file path.");
            }
            let mut maxlen = 0usize;
            let mut rdr = config.reader()?;
            let mut record = csv::ByteRecord::new();
            while rdr.read_byte_record(&mut record)? {
                let mut index = 0;
                let mut nonempty_count = 0;
                for field in &record {
                    index += 1;
                    if index == 1 || !field.is_empty() {
                        nonempty_count = index;
                    }
                }
                maxlen = cmp::max(maxlen, nonempty_count);
            }
            maxlen
        }
    };

    let mut rdr = config.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;
    for r in rdr.byte_records() {
        let mut r = r?;
        if length >= r.len() {
            for _ in r.len()..length {
                r.push_field(b"");
            }
        } else {
            r.truncate(length);
        }
        wtr.write_byte_record(&r)?;
    }
    wtr.flush()?;
    Ok(())
}
