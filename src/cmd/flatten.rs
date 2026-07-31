use std::borrow::Cow;
use std::io::{self, Write};

use tabwriter::TabWriter;

use crate::config::{Config, Delimiter};
use crate::util;
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'c', long = "condense", value_name = "arg")]
    pub flag_condense: Option<usize>,
    #[arg(short = 'S', long = "separator", value_name = "arg")]
    pub flag_separator: Option<String>,
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
    let headers = rdr.byte_headers()?.clone();

    let mut wtr = TabWriter::new(io::stdout());
    let sep = match args.flag_separator {
        None => Cow::Borrowed(&b"#"[..]),
        Some(ref sep) => Cow::Owned(sep.clone().into_bytes()),
    };
    let mut first = true;
    for r in rdr.byte_records() {
        if !first {
            writeln!(&mut wtr, "{}", String::from_utf8_lossy(&sep))?;
        }
        first = false;
        let r = r?;
        for (i, (header, field)) in headers.iter().zip(&r).enumerate() {
            if rconfig.no_headers {
                write!(&mut wtr, "{}", i)?;
            } else {
                wtr.write_all(header)?;
            }
            wtr.write_all(b"\t")?;
            wtr.write_all(&util::condense(Cow::Borrowed(field), args.flag_condense))?;
            wtr.write_all(b"\n")?;
        }
    }
    wtr.flush()?;
    Ok(())
}
