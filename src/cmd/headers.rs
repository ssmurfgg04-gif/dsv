use std::io;

use tabwriter::TabWriter;

use crate::config::Delimiter;
use crate::util;
use crate::CliResult;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Vec<String>,
    #[arg(long = "just-names")]
    pub flag_just_names: bool,
    #[arg(long = "intersect")]
    pub flag_intersect: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let configs = util::many_configs(&args.arg_input, args.flag_delimiter, true)?;

    let num_inputs = configs.len();
    let mut headers: Vec<Vec<u8>> = vec![];
    for conf in configs.into_iter() {
        let mut rdr = conf.reader()?;
        for header in rdr.byte_headers()?.iter() {
            if !args.flag_intersect || !headers.iter().any(|h| &**h == header) {
                headers.push(header.to_vec());
            }
        }
    }

    let mut wtr: Box<dyn io::Write> = if args.flag_just_names {
        Box::new(io::stdout())
    } else {
        Box::new(TabWriter::new(io::stdout()))
    };
    for (i, header) in headers.into_iter().enumerate() {
        if num_inputs == 1 && !args.flag_just_names {
            write!(&mut wtr, "{}\t", i + 1)?;
        }
        wtr.write_all(&header)?;
        wtr.write_all(b"\n")?;
    }
    wtr.flush()?;
    Ok(())
}
