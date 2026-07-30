use std::io;
use std::process;

use clap::{Parser, Subcommand};

macro_rules! wout {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stdout(), $($arg)*)).unwrap();
    });
}

macro_rules! werr {
    ($($arg:tt)*) => ({
        use std::io::Write;
        (writeln!(&mut ::std::io::stderr(), $($arg)*)).unwrap();
    });
}

macro_rules! fail {
    ($e:expr) => (Err(::std::convert::From::from($e)));
}

macro_rules! command_list {
    () => (
"    cat         Concatenate by row or column
    convert     Convert between CSV/TSV/Parquet/JSONL
    count       Count records
    dedup       Deduplicate rows
    exclude     Exclude rows matching a regex
    fill        Fill empty values
    fixlengths  Makes all records have same length
    flatten     Show one field per line
    fmt         Format CSV output (change field delimiter)
    frequency   Show frequency tables
    headers     Show header names
    help        Show this usage message.
    index       Create CSV index for faster access
    input       Read CSV data with special quoting rules
    join        Join CSV files
    partition   Partition CSV data based on a column value
    rename      Rename column headers
    reverse     Reverse rows of CSV data
    sample      Randomly sample CSV data
    search      Search CSV data with regexes
    select      Select columns from CSV
    slice       Slice records from CSV
    sort        Sort CSV data
    split       Split CSV data into many files
    stats       Compute basic statistics
    table       Align CSV data into columns
    transpose   Transpose rows and columns
"
    )
}

mod cmd;
mod config;
mod data;
mod index;
mod select;
mod util;

#[derive(Parser)]
#[command(
    name = "dsv",
    version = env!("CARGO_PKG_VERSION"),
    about = "dsv is a suite of CSV/Parquet/JSONL command line utilities."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long, global = true, help = "List all commands available.")]
    list: bool,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Concatenate by row or column")]
    Cat(cmd::cat::Args),
    #[command(about = "Convert between CSV/TSV/Parquet/JSONL")]
    Convert(cmd::convert::Args),
    #[command(about = "Count records")]
    Count(cmd::count::Args),
    #[command(about = "Deduplicate rows")]
    Dedup(cmd::dedup::Args),
    #[command(about = "Exclude rows matching a regex")]
    Exclude(cmd::exclude::Args),
    #[command(about = "Fill empty values")]
    Fill(cmd::fill::Args),
    #[command(name = "fixlengths", about = "Makes all records have same length")]
    FixLengths(cmd::fixlengths::Args),
    #[command(about = "Show one field per line")]
    Flatten(cmd::flatten::Args),
    #[command(about = "Format CSV output (change field delimiter)")]
    Fmt(cmd::fmt::Args),
    #[command(about = "Show frequency tables")]
    Frequency(cmd::frequency::Args),
    #[command(about = "Show header names")]
    Headers(cmd::headers::Args),
    #[command(about = "Create CSV index for faster access")]
    Index(cmd::index::Args),
    #[command(about = "Read CSV data with special quoting rules")]
    Input(cmd::input::Args),
    #[command(about = "Join CSV files")]
    Join(cmd::join::Args),
    #[command(about = "Partition CSV data based on a column value")]
    Partition(cmd::partition::Args),
    #[command(about = "Rename column headers")]
    Rename(cmd::rename::Args),
    #[command(about = "Reverse rows of CSV data")]
    Reverse(cmd::reverse::Args),
    #[command(about = "Randomly sample CSV data")]
    Sample(cmd::sample::Args),
    #[command(about = "Search CSV data with regexes")]
    Search(cmd::search::Args),
    #[command(about = "Select columns from CSV")]
    Select(cmd::select::Args),
    #[command(about = "Slice records from CSV")]
    Slice(cmd::slice::Args),
    #[command(about = "Sort CSV data")]
    Sort(cmd::sort::Args),
    #[command(about = "Split CSV data into many files")]
    Split(cmd::split::Args),
    #[command(about = "Compute basic statistics")]
    Stats(cmd::stats::Args),
    #[command(about = "Align CSV data into columns")]
    Table(cmd::table::Args),
    #[command(about = "Transpose rows and columns")]
    Transpose(cmd::transpose::Args),
}

fn main() {
    let cli = Cli::parse();
    if cli.list {
        wout!(concat!("Installed commands:", command_list!()));
        return;
    }
    match cli.command {
        None => {
            werr!(concat!(
                "dsv is a suite of CSV/Parquet/JSONL command line utilities.\n\
\n\
Please choose one of the following commands:", command_list!()));
            process::exit(0);
        }
        Some(cmd) => {
            match cmd.run() {
                Ok(()) => process::exit(0),
                Err(CliError::Csv(err)) => {
                    werr!("{}", err);
                    process::exit(1);
                }
                Err(CliError::Io(ref err))
                        if err.kind() == io::ErrorKind::BrokenPipe => {
                    process::exit(0);
                }
                Err(CliError::Io(err)) => {
                    werr!("{}", err);
                    process::exit(1);
                }
                Err(CliError::Other(msg)) => {
                    werr!("{}", msg);
                    process::exit(1);
                }
            }
        }
    }
}

impl Command {
    fn run(self) -> CliResult<()> {
        match self {
            Command::Cat(args) => cmd::cat::run(&args),
            Command::Convert(args) => cmd::convert::run(&args),
            Command::Count(args) => cmd::count::run(&args),
            Command::Dedup(args) => cmd::dedup::run(&args),
            Command::Exclude(args) => cmd::exclude::run(&args),
            Command::Fill(args) => cmd::fill::run(&args),
            Command::FixLengths(args) => cmd::fixlengths::run(&args),
            Command::Flatten(args) => cmd::flatten::run(&args),
            Command::Fmt(args) => cmd::fmt::run(&args),
            Command::Frequency(args) => cmd::frequency::run(&args),
            Command::Headers(args) => cmd::headers::run(&args),
            Command::Index(args) => cmd::index::run(&args),
            Command::Input(args) => cmd::input::run(&args),
            Command::Join(args) => cmd::join::run(&args),
            Command::Partition(args) => cmd::partition::run(&args),
            Command::Rename(args) => cmd::rename::run(&args),
            Command::Reverse(args) => cmd::reverse::run(&args),
            Command::Sample(args) => cmd::sample::run(&args),
            Command::Search(args) => cmd::search::run(&args),
            Command::Select(args) => cmd::select::run(&args),
            Command::Slice(args) => cmd::slice::run(&args),
            Command::Sort(args) => cmd::sort::run(&args),
            Command::Split(args) => cmd::split::run(&args),
            Command::Stats(args) => cmd::stats::run(&args),
            Command::Table(args) => cmd::table::run(&args),
            Command::Transpose(args) => cmd::transpose::run(&args),
        }
    }
}

pub type CliResult<T> = Result<T, CliError>;

#[derive(Debug)]
pub enum CliError {
    Csv(csv::Error),
    Io(io::Error),
    Other(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            CliError::Csv(ref e) => { e.fmt(f) }
            CliError::Io(ref e) => { e.fmt(f) }
            CliError::Other(ref s) => { f.write_str(s) }
        }
    }
}

impl From<csv::Error> for CliError {
    fn from(err: csv::Error) -> CliError {
        if !err.is_io_error() {
            return CliError::Csv(err);
        }
        match err.into_kind() {
            csv::ErrorKind::Io(v) => From::from(v),
            _ => unreachable!(),
        }
    }
}

impl From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        CliError::Io(err)
    }
}

impl From<String> for CliError {
    fn from(err: String) -> CliError {
        CliError::Other(err)
    }
}

impl<'a> From<&'a str> for CliError {
    fn from(err: &'a str) -> CliError {
        CliError::Other(err.to_owned())
    }
}

impl From<regex::Error> for CliError {
    fn from(err: regex::Error) -> CliError {
        CliError::Other(format!("{:?}", err))
    }
}

impl From<anyhow::Error> for CliError {
    fn from(err: anyhow::Error) -> CliError {
        CliError::Other(format!("{err:?}"))
    }
}

#[cfg(feature = "parquet")]
impl From<parquet::errors::ParquetError> for CliError {
    fn from(err: parquet::errors::ParquetError) -> CliError {
        CliError::Other(format!("Parquet: {err}"))
    }
}

#[cfg(feature = "parquet")]
impl From<arrow::error::ArrowError> for CliError {
    fn from(err: arrow::error::ArrowError) -> CliError {
        CliError::Other(format!("Arrow: {err}"))
    }
}

#[cfg(feature = "jsonl")]
impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> CliError {
        CliError::Other(format!("JSON: {err}"))
    }
}
