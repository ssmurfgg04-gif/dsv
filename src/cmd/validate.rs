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
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let conf = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .flexible(true);

    let mut rdr = conf.reader()?;
    let headers = if conf.no_headers {
        None
    } else {
        let h = rdr.byte_headers()?.clone();
        Some(h.len())
    };

    let mut wtr = Config::new(&args.flag_output).writer()?;
    let mut record = csv::ByteRecord::new();
    let mut row_num: u64 = if conf.no_headers { 1 } else { 2 };
    let mut errors = 0u64;

    if let Some(header_len) = headers {
        while rdr.read_byte_record(&mut record)? {
            if record.len() != header_len {
                errors += 1;
                wtr.write_record([
                    row_num.to_string().as_bytes(),
                    b"length mismatch",
                    format!("expected {}, got {}", header_len, record.len()).as_bytes(),
                ])?;
            }
            row_num += 1;
        }
    } else {
        let mut expected = None;
        while rdr.read_byte_record(&mut record)? {
            match expected {
                None => expected = Some(record.len()),
                Some(exp) if record.len() != exp => {
                    errors += 1;
                    wtr.write_record([
                        row_num.to_string().as_bytes(),
                        b"length mismatch",
                        format!("expected {}, got {}", exp, record.len()).as_bytes(),
                    ])?;
                }
                _ => {}
            }
            row_num += 1;
        }
    }
    wtr.flush()?;

    if errors > 0 {
        eprintln!("dsv validate: {} invalid record(s)", errors);
        std::process::exit(1);
    }
    Ok(())
}
