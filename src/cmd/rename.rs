use csv::ByteRecord;
use crate::CliResult;
use crate::config::{Config, Delimiter};
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_rename: String,
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(false);
    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let headers = rdr.byte_headers()?.clone();
    let mapping: Vec<(String, String)> = args.arg_rename.split(',')
        .map(|pair| {
            let parts: Vec<&str> = pair.split(':').collect();
            if parts.len() == 2 {
                (parts[0].trim().to_owned(), parts[1].trim().to_owned())
            } else {
                let parts2: Vec<&str> = pair.split('=').collect();
                if parts2.len() == 2 {
                    (parts2[0].trim().to_owned(), parts2[1].trim().to_owned())
                } else {
                    (String::new(), String::new())
                }
            }
        })
        .filter(|(a, _b)| !a.is_empty())
        .collect();

    let mut new_headers = ByteRecord::new();
    for h in headers.iter() {
        let key = std::str::from_utf8(h).unwrap_or("");
        let renamed = mapping.iter().find(|(a, _)| a == key).map(|(_, b)| b.as_str()).unwrap_or(key);
        new_headers.push_field(renamed.as_bytes());
    }
    wtr.write_record(&new_headers)?;

    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? { wtr.write_byte_record(&rec)?; }
    wtr.flush()?;
    Ok(())
}
