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

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColType {
    Null,
    Unicode,
    Float,
    Integer,
}

impl ColType {
    fn from_sample(sample: &[u8]) -> ColType {
        if sample.is_empty() {
            return ColType::Null;
        }
        let string = match std::str::from_utf8(sample) {
            Err(_) => return ColType::Unicode,
            Ok(s) => s,
        };
        if string.parse::<i64>().is_ok() {
            ColType::Integer
        } else if string.parse::<f64>().is_ok() {
            ColType::Float
        } else {
            ColType::Unicode
        }
    }

    fn merge(&mut self, other: ColType) {
        *self = match (*self, other) {
            (ColType::Null, any) | (any, ColType::Null) => any,
            (a, b) if a == b => a,
            (ColType::Integer, ColType::Float) | (ColType::Float, ColType::Integer) => {
                ColType::Float
            }
            _ => ColType::Unicode,
        };
    }
}

impl std::fmt::Display for ColType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ColType::Null => write!(f, "null"),
            ColType::Unicode => write!(f, "string"),
            ColType::Float => write!(f, "float"),
            ColType::Integer => write!(f, "integer"),
        }
    }
}

pub fn run(args: &Args) -> CliResult<()> {
    let conf = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers);

    let mut rdr = conf.reader()?;
    let headers = rdr.byte_headers()?.clone();
    let mut types: Vec<ColType> = (0..headers.len()).map(|_| ColType::Null).collect();

    let mut record = csv::ByteRecord::new();
    while rdr.read_byte_record(&mut record)? {
        for (i, ty) in types.iter_mut().enumerate() {
            let sample = if i < record.len() { &record[i] } else { b"" };
            let t = ColType::from_sample(sample);
            ty.merge(t);
        }
    }

    let mut wtr = Config::new(&args.flag_output).writer()?;
    for (i, header) in headers.iter().enumerate() {
        let name = if conf.no_headers {
            format!("column_{}", i + 1).into_bytes()
        } else {
            header.to_vec()
        };
        wtr.write_record([name.as_slice(), types[i].to_string().as_bytes()])?;
    }
    wtr.flush()?;
    Ok(())
}
