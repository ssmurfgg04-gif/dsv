use std::borrow::ToOwned;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use crate::index::Indexed;

use crate::data::{DataReader, Format};
use crate::select::Selection;
use crate::util;
#[cfg(feature = "parquet")]
use crate::CliError;
use crate::CliResult;

#[derive(Clone, Copy, Debug)]
pub struct Delimiter(pub u8);

impl Delimiter {
    pub fn as_byte(self) -> u8 {
        self.0
    }
}

impl FromStr for Delimiter {
    type Err = String;
    fn from_str(s: &str) -> Result<Delimiter, String> {
        match s {
            r"\t" => Ok(Delimiter(b'\t')),
            s => {
                let bytes = s.as_bytes();
                if bytes.len() != 1 {
                    return Err(format!(
                        "Could not convert '{}' to a single ASCII character.",
                        s
                    ));
                }
                let c = bytes[0];
                if c.is_ascii() {
                    Ok(Delimiter(c))
                } else {
                    Err(format!("Could not convert '{}' to ASCII delimiter.", s))
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Config {
    path: Option<PathBuf>, // None implies <stdin>
    idx_path: Option<PathBuf>,
    select_columns: Option<crate::select::SelectColumns>,
    delimiter: u8,
    pub no_headers: bool,
    flexible: bool,
    terminator: csv::Terminator,
    quote: u8,
    quote_style: csv::QuoteStyle,
    double_quote: bool,
    escape: Option<u8>,
    quoting: bool,
}

impl Config {
    pub fn new(path: &Option<String>) -> Config {
        let (path, delim) = match *path {
            None => (None, b','),
            Some(ref s) if s.deref() == "-" => (None, b','),
            Some(ref s) => {
                let path = PathBuf::from(s);
                let delim = if path.extension().is_some_and(|v| v == "tsv" || v == "tab") {
                    b'\t'
                } else {
                    b','
                };
                (Some(path), delim)
            }
        };
        Config {
            path,
            idx_path: None,
            select_columns: None,
            delimiter: delim,
            no_headers: false,
            flexible: false,
            terminator: csv::Terminator::Any(b'\n'),
            quote: b'"',
            quote_style: csv::QuoteStyle::Necessary,
            double_quote: true,
            escape: None,
            quoting: true,
        }
    }

    pub fn delimiter(mut self, d: Option<Delimiter>) -> Config {
        if let Some(d) = d {
            self.delimiter = d.as_byte();
        }
        self
    }

    pub fn no_headers(mut self, mut yes: bool) -> Config {
        if env::var("XSV_TOGGLE_HEADERS").unwrap_or("0".to_owned()) == "1" {
            yes = !yes;
        }
        self.no_headers = yes;
        self
    }

    pub fn flexible(mut self, yes: bool) -> Config {
        self.flexible = yes;
        self
    }

    pub fn crlf(mut self, yes: bool) -> Config {
        if yes {
            self.terminator = csv::Terminator::CRLF;
        } else {
            self.terminator = csv::Terminator::Any(b'\n');
        }
        self
    }

    pub fn terminator(mut self, term: csv::Terminator) -> Config {
        self.terminator = term;
        self
    }

    pub fn quote(mut self, quote: u8) -> Config {
        self.quote = quote;
        self
    }

    pub fn quote_style(mut self, style: csv::QuoteStyle) -> Config {
        self.quote_style = style;
        self
    }

    pub fn double_quote(mut self, yes: bool) -> Config {
        self.double_quote = yes;
        self
    }

    pub fn escape(mut self, escape: Option<u8>) -> Config {
        self.escape = escape;
        self
    }

    pub fn quoting(mut self, yes: bool) -> Config {
        self.quoting = yes;
        self
    }

    pub fn select(mut self, sel_cols: crate::select::SelectColumns) -> Config {
        self.select_columns = Some(sel_cols);
        self
    }

    pub fn is_std(&self) -> bool {
        self.path.is_none()
    }

    pub fn selection(&self, first_record: &csv::ByteRecord) -> Result<Selection, String> {
        match self.select_columns {
            None => Err("Config has no 'SelectColums'. Did you call \
                         Config::select?"
                .to_owned()),
            Some(ref sel) => sel.selection(first_record, !self.no_headers),
        }
    }

    pub fn write_headers<R: io::Read, W: io::Write>(
        &self,
        r: &mut csv::Reader<R>,
        w: &mut csv::Writer<W>,
    ) -> csv::Result<()> {
        if !self.no_headers {
            let r = r.byte_headers()?;
            if !r.is_empty() {
                w.write_record(r)?;
            }
        }
        Ok(())
    }

    pub fn writer(&self) -> io::Result<csv::Writer<Box<dyn io::Write + 'static>>> {
        Ok(self.build_writer(self.io_writer()?))
    }

    pub fn reader(&self) -> io::Result<csv::Reader<Box<dyn io::Read + 'static>>> {
        #[cfg(feature = "parquet")]
        if let Some(ref p) = self.path {
            let name = p.display().to_string().to_lowercase();
            if name.ends_with(".parquet") || name.ends_with(".par") {
                return self.parquet_csv_reader();
            }
        }
        #[cfg(feature = "jsonl")]
        if let Some(ref p) = self.path {
            let name = p.display().to_string().to_lowercase();
            if name.ends_with(".jsonl") || name.ends_with(".ndjson") {
                return self.jsonl_csv_reader();
            }
        }
        Ok(self.build_reader(self.io_reader()?))
    }

    #[cfg(feature = "parquet")]
    fn parquet_csv_reader(&self) -> io::Result<csv::Reader<Box<dyn io::Read + 'static>>> {
        use crate::data::ParquetReader;
        let path = self.path.as_ref().unwrap().display().to_string();
        let mut pr = ParquetReader::from_file(&path)
            .map_err(|e| io::Error::other(format!("Parquet: {e}")))?;
        let headers = pr
            .headers()
            .map_err(|e| io::Error::other(format!("Parquet: {e}")))?;
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(self.delimiter)
                .has_headers(false)
                .from_writer(&mut buf);
            wtr.write_byte_record(&headers)
                .map_err(|e| io::Error::other(format!("CSV: {e}")))?;
            let mut rec = csv::ByteRecord::new();
            while pr
                .read_byte_record(&mut rec)
                .map_err(|e| io::Error::other(format!("Parquet: {e}")))?
            {
                wtr.write_byte_record(&rec)
                    .map_err(|e| io::Error::other(format!("CSV: {e}")))?;
            }
            wtr.flush().ok();
        }
        Ok(csv::ReaderBuilder::new()
            .flexible(self.flexible)
            .delimiter(self.delimiter)
            .has_headers(!self.no_headers)
            .quote(self.quote)
            .quoting(self.quoting)
            .escape(self.escape)
            .from_reader(Box::new(io::Cursor::new(buf)) as Box<dyn io::Read>))
    }

    #[cfg(feature = "jsonl")]
    fn jsonl_csv_reader(&self) -> io::Result<csv::Reader<Box<dyn io::Read + 'static>>> {
        use crate::data::JsonlReader;
        let file = self.io_reader()?;
        let mut jr = JsonlReader::new(file, self.no_headers)
            .map_err(|e| io::Error::other(format!("JSONL: {e}")))?;
        let headers = jr
            .headers()
            .map_err(|e| io::Error::other(format!("JSONL: {e}")))?;
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut wtr = csv::WriterBuilder::new()
                .delimiter(self.delimiter)
                .has_headers(false)
                .from_writer(&mut buf);
            if !self.no_headers {
                wtr.write_byte_record(&headers)
                    .map_err(|e| io::Error::other(format!("CSV: {e}")))?;
            }
            let mut rec = csv::ByteRecord::new();
            while jr
                .read_byte_record(&mut rec)
                .map_err(|e| io::Error::other(format!("JSONL: {e}")))?
            {
                wtr.write_byte_record(&rec)
                    .map_err(|e| io::Error::other(format!("CSV: {e}")))?;
            }
            wtr.flush().ok();
        }
        Ok(csv::ReaderBuilder::new()
            .flexible(self.flexible)
            .delimiter(self.delimiter)
            .has_headers(!self.no_headers)
            .quote(self.quote)
            .quoting(self.quoting)
            .escape(self.escape)
            .from_reader(Box::new(io::Cursor::new(buf)) as Box<dyn io::Read>))
    }

    pub fn data_reader(&self, fmt: Format) -> CliResult<DataReader> {
        #[cfg(feature = "parquet")]
        if fmt == Format::Parquet {
            let path = match &self.path {
                Some(p) => p.display().to_string(),
                None => return Err(CliError::Other("Parquet requires a file path".into())),
            };
            return DataReader::from_path(&path, fmt, Some(self.delimiter), self.no_headers);
        }
        let file: Box<dyn io::Read> = self.io_reader()?;
        let delim = Some(self.delimiter);
        DataReader::from_reader(file, fmt, delim, self.no_headers)
    }

    pub fn reader_file(&self) -> io::Result<csv::Reader<fs::File>> {
        match self.path {
            None => Err(io::Error::other("Cannot use <stdin> here")),
            Some(ref p) => fs::File::open(p).map(|f| self.build_reader(f)),
        }
    }

    pub fn index_files(&self) -> io::Result<Option<(csv::Reader<fs::File>, fs::File)>> {
        let (csv_file, idx_file) = match (&self.path, &self.idx_path) {
            (&None, &None) => return Ok(None),
            (&None, &Some(_)) => {
                return Err(io::Error::other(
                    "Cannot use <stdin> with indexes",
                    // Some(format!("index file: {}", p.display()))
                ));
            }
            (Some(p), &None) => {
                // We generally don't want to report an error here, since we're
                // passively trying to find an index.
                let idx_file = match fs::File::open(util::idx_path(p)) {
                    // TODO: Maybe we should report an error if the file exists
                    // but is not readable.
                    Err(_) => return Ok(None),
                    Ok(f) => f,
                };
                (fs::File::open(p)?, idx_file)
            }
            (Some(p), Some(ip)) => (fs::File::open(p)?, fs::File::open(ip)?),
        };
        // If the CSV data was last modified after the index file was last
        // modified, then return an error and demand the user regenerate the
        // index.
        let data_modified = util::last_modified(&csv_file.metadata()?);
        let idx_modified = util::last_modified(&idx_file.metadata()?);
        if data_modified > idx_modified {
            return Err(io::Error::other(
                "The CSV file was modified after the index file. \
                 Please re-create the index.",
            ));
        }
        let csv_rdr = self.build_reader(csv_file);
        Ok(Some((csv_rdr, idx_file)))
    }

    pub fn indexed(&self) -> CliResult<Option<Indexed<fs::File, fs::File>>> {
        match self.index_files()? {
            None => Ok(None),
            Some((r, i)) => Ok(Some(Indexed::open(r, i)?)),
        }
    }

    pub fn io_reader(&self) -> io::Result<Box<dyn io::Read + 'static>> {
        Ok(match self.path {
            None => Box::new(io::stdin()),
            Some(ref p) => match fs::File::open(p) {
                Ok(x) => Box::new(x),
                Err(err) => {
                    let msg = format!("failed to open {}: {}", p.display(), err);
                    return Err(io::Error::new(io::ErrorKind::NotFound, msg));
                }
            },
        })
    }

    pub fn build_reader<R: Read>(&self, rdr: R) -> csv::Reader<R> {
        csv::ReaderBuilder::new()
            .flexible(self.flexible)
            .delimiter(self.delimiter)
            .has_headers(!self.no_headers)
            .quote(self.quote)
            .quoting(self.quoting)
            .escape(self.escape)
            .from_reader(rdr)
    }

    pub fn io_writer(&self) -> io::Result<Box<dyn io::Write + 'static>> {
        Ok(match self.path {
            None => Box::new(io::stdout()),
            Some(ref p) => Box::new(fs::File::create(p)?),
        })
    }

    pub fn build_writer<W: io::Write>(&self, wtr: W) -> csv::Writer<W> {
        csv::WriterBuilder::new()
            .flexible(self.flexible)
            .delimiter(self.delimiter)
            .terminator(self.terminator)
            .quote(self.quote)
            .quote_style(self.quote_style)
            .double_quote(self.double_quote)
            .escape(self.escape.unwrap_or(b'\\'))
            .buffer_capacity(32 * (1 << 10))
            .from_writer(wtr)
    }
}
