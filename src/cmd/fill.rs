use csv::ByteRecord;
use crate::CliResult;
use crate::CliError;
use crate::config::{Config, Delimiter};
use crate::select::SelectColumns;
use clap::Parser;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg()]
    pub arg_input: Option<String>,
    #[arg(short = 's', long = "select", default_value = "")]
    pub flag_select: SelectColumns,
    #[arg(short = 'o', long = "output", value_name = "file")]
    pub flag_output: Option<String>,
    #[arg(short = 'n', long = "no-headers")]
    pub flag_no_headers: bool,
    #[arg(short = 'd', long = "delimiter", value_name = "arg")]
    pub flag_delimiter: Option<Delimiter>,
    #[arg(short = 'g', long = "group")]
    pub flag_group: Vec<String>,
    #[arg(short = 'b', long = "backfill")]
    pub flag_backfill: bool,
}

pub fn run(args: &Args) -> CliResult<()> {
    let rconfig = Config::new(&args.arg_input)
        .delimiter(args.flag_delimiter)
        .no_headers(args.flag_no_headers)
        .select(args.flag_select.clone());
    let mut rdr = rconfig.reader()?;
    let mut wtr = Config::new(&args.flag_output).writer()?;

    let headers = rdr.byte_headers()?.clone();
    let sel = rconfig.selection(&headers)?;
    if args.flag_select.is_empty() && args.flag_group.is_empty() {
        return Err(CliError::Other("fill requires either --select or --group".into()));
    }

    if !rconfig.no_headers { wtr.write_record(&headers)?; }

    let mut rows: Vec<ByteRecord> = Vec::new();
    let mut rec = ByteRecord::new();
    while rdr.read_byte_record(&mut rec)? { rows.push(rec.clone()); }

    if rows.is_empty() { return Ok(()); }

    let cols: Vec<usize> = if !args.flag_select.is_empty() {
        sel.to_vec()
    } else { (0..rows[0].len()).collect() };

    let _group_cols: Vec<usize> = if !args.flag_group.is_empty() {
        let gh_str = args.flag_group.join(",");
        let gcols = SelectColumns::from_str(&gh_str).map_err(|e| CliError::Other(e))?;
        let gsel = gcols.selection(&headers, !rconfig.no_headers)
            .map_err(|e| CliError::Other(e))?;
        gsel.to_vec()
    } else { Vec::new() };

    if args.flag_backfill {
        for i in (0..rows.len()).rev() {
            for &c in &cols {
                if rows[i].get(c).map(|f| f.is_empty()).unwrap_or(true) {
                    if let Some(next) = (i+1..rows.len()).find(|&j| !rows[j].get(c).map(|f| f.is_empty()).unwrap_or(true)) {
                        let mut new = rows[i].clone();
                        new.push_field(&rows[next][c]);
                        rows[i] = new;
                    }
                }
            }
        }
    } else {
        let mut last_val: Vec<Option<Vec<u8>>> = vec![None; rows[0].len()];
        for i in 0..rows.len() {
            for &c in &cols {
                if rows[i].get(c).map(|f| f.is_empty()).unwrap_or(true) {
                    if let Some(ref val) = last_val[c] {
                        let mut new = rows[i].clone();
                        new.push_field(val);
                        rows[i] = new;
                    }
                } else {
                    last_val[c] = Some(rows[i][c].to_vec());
                }
            }
        }
    }

    for row in &rows { wtr.write_byte_record(row)?; }
    wtr.flush()?;
    Ok(())
}
