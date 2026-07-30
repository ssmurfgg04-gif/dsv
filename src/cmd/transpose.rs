use csv::ByteRecord;
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
    let mut rdr = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let headers = rdr.byte_headers()?.clone();
    let mut rows: Vec<ByteRecord> = Vec::new();
    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? { rows.push(rec.clone()); }

    if args.flag_no_headers {
        let ncols = if rows.is_empty() { 0 } else { rows[0].len() };
        for col in 0..ncols {
            let mut out = ByteRecord::new();
            for row in &rows {
                out.push_field(row.get(col).unwrap_or(b""));
            }
            wtr.write_byte_record(&out)?;
        }
    } else {
        let ncols = headers.len();
        for col in 0..ncols {
            let mut out = ByteRecord::new();
            out.push_field(headers.get(col).unwrap_or(b""));
            for row in &rows {
                out.push_field(row.get(col).unwrap_or(b""));
            }
            wtr.write_byte_record(&out)?;
        }
    }
    wtr.flush()?;
    Ok(())
}
