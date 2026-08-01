use csv::ByteRecord;
use std::io::{Read, Write};
use std::path::Path;

#[cfg(feature = "jsonl")]
use std::io::{BufRead, BufReader};

#[cfg(any(feature = "jsonl", feature = "parquet"))]
use crate::CliError;
use crate::CliResult;

fn strip_gz(p: &str) -> String {
    let l = p.to_lowercase();
    if l.ends_with(".gz") {
        p[..p.len() - 3].to_owned()
    } else {
        p.to_owned()
    }
}

#[cfg(feature = "parquet")]
fn open_gzip(path: &str) -> std::io::Result<Box<dyn Read>> {
    let file = std::fs::File::open(path)?;
    if strip_gz(path) != path {
        Ok(Box::new(flate2::read::MultiGzDecoder::new(file)))
    } else {
        Ok(Box::new(file))
    }
}

fn open_gzip_write(path: &str) -> std::io::Result<Box<dyn Write>> {
    let file = std::fs::File::create(path)?;
    if strip_gz(path) != path {
        Ok(Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::default(),
        )))
    } else {
        Ok(Box::new(file))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    Csv,
    Tsv,
    #[cfg(feature = "jsonl")]
    Jsonl,
    #[cfg(feature = "parquet")]
    Parquet,
}

impl Format {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Format {
        let raw = path.as_ref().display().to_string();
        let p = strip_gz(&raw).to_lowercase();
        if p.ends_with(".tsv") || p.ends_with(".tab") {
            return Format::Tsv;
        }
        #[cfg(feature = "parquet")]
        if p.ends_with(".parquet") || p.ends_with(".par") {
            return Format::Parquet;
        }
        #[cfg(feature = "jsonl")]
        if p.ends_with(".jsonl") || p.ends_with(".ndjson") {
            return Format::Jsonl;
        }
        Format::Csv
    }
}

pub enum DataReader {
    Csv(csv::Reader<Box<dyn Read>>),
    Tsv(csv::Reader<Box<dyn Read>>),
    #[cfg(feature = "jsonl")]
    Jsonl(JsonlReader),
    #[cfg(feature = "parquet")]
    Parquet(ParquetReader),
}

impl DataReader {
    pub fn from_reader<R: Read + 'static>(
        r: R,
        fmt: Format,
        delim: Option<u8>,
        no_headers: bool,
    ) -> CliResult<DataReader> {
        match fmt {
            Format::Csv | Format::Tsv => {
                let rdr = csv::ReaderBuilder::new()
                    .delimiter(delim.unwrap_or(if fmt == Format::Tsv { b'\t' } else { b',' }))
                    .has_headers(!no_headers)
                    .flexible(true)
                    .from_reader(Box::new(r) as Box<dyn Read>);
                Ok(if fmt == Format::Tsv {
                    DataReader::Tsv(rdr)
                } else {
                    DataReader::Csv(rdr)
                })
            }
            #[cfg(feature = "jsonl")]
            Format::Jsonl => Ok(DataReader::Jsonl(JsonlReader::new(r, no_headers)?)),
            #[cfg(feature = "parquet")]
            Format::Parquet => Err(CliError::Other(
                "Parquet reader must be created from a file path".into(),
            )),
        }
    }

    #[cfg(feature = "parquet")]
    pub fn from_path(
        path: &str,
        fmt: Format,
        delim: Option<u8>,
        no_headers: bool,
    ) -> CliResult<DataReader> {
        match fmt {
            Format::Csv | Format::Tsv => {
                let file = open_gzip(path)?;
                DataReader::from_reader(file, fmt, delim, no_headers)
            }
            #[cfg(feature = "jsonl")]
            Format::Jsonl => {
                let file = open_gzip(path)?;
                DataReader::from_reader(file, fmt, delim, no_headers)
            }
            #[cfg(feature = "parquet")]
            Format::Parquet => {
                let pr = ParquetReader::from_file(path)?;
                Ok(DataReader::Parquet(pr))
            }
        }
    }

    pub fn headers(&mut self) -> CliResult<ByteRecord> {
        match self {
            DataReader::Csv(r) | DataReader::Tsv(r) => {
                let h = r.headers()?.clone().into();
                Ok(h)
            }
            #[cfg(feature = "jsonl")]
            DataReader::Jsonl(r) => r.headers(),
            #[cfg(feature = "parquet")]
            DataReader::Parquet(r) => r.headers(),
        }
    }

    pub fn read_byte_record(&mut self, rec: &mut ByteRecord) -> CliResult<bool> {
        match self {
            DataReader::Csv(r) | DataReader::Tsv(r) => Ok(r.read_byte_record(rec)?),
            #[cfg(feature = "jsonl")]
            DataReader::Jsonl(r) => r.read_byte_record(rec),
            #[cfg(feature = "parquet")]
            DataReader::Parquet(r) => r.read_byte_record(rec),
        }
    }
}

pub enum DataWriter {
    Csv(csv::Writer<Box<dyn Write>>),
    Tsv(csv::Writer<Box<dyn Write>>),
    #[cfg(feature = "jsonl")]
    Jsonl(JsonlWriter),
    #[cfg(feature = "parquet")]
    Parquet(ParquetWriter),
}

impl DataWriter {
    pub fn from_writer<W: Write + 'static>(
        w: W,
        fmt: Format,
        delim: Option<u8>,
    ) -> CliResult<DataWriter> {
        let delim = delim.unwrap_or(if fmt == Format::Tsv { b'\t' } else { b',' });
        match fmt {
            Format::Csv => Ok(DataWriter::Csv(
                csv::WriterBuilder::new()
                    .delimiter(delim)
                    .from_writer(Box::new(w) as Box<dyn Write>),
            )),
            Format::Tsv => Ok(DataWriter::Tsv(
                csv::WriterBuilder::new()
                    .delimiter(delim)
                    .from_writer(Box::new(w) as Box<dyn Write>),
            )),
            #[cfg(feature = "jsonl")]
            Format::Jsonl => Ok(DataWriter::Jsonl(JsonlWriter::new(w))),
            #[cfg(feature = "parquet")]
            Format::Parquet => Ok(DataWriter::Parquet(ParquetWriter::new(w)?)),
        }
    }

    pub fn from_path(path: &str, fmt: Format, delim: Option<u8>) -> CliResult<DataWriter> {
        match fmt {
            Format::Csv | Format::Tsv => {
                DataWriter::from_writer(open_gzip_write(path)?, fmt, delim)
            }
            #[cfg(feature = "jsonl")]
            Format::Jsonl => DataWriter::from_writer(open_gzip_write(path)?, fmt, delim),
            #[cfg(feature = "parquet")]
            Format::Parquet => Ok(DataWriter::Parquet(ParquetWriter::from_path(path)?)),
        }
    }

    pub fn write_byte_record(&mut self, rec: &ByteRecord) -> CliResult<()> {
        match self {
            DataWriter::Csv(w) | DataWriter::Tsv(w) => Ok(w.write_byte_record(rec)?),
            #[cfg(feature = "jsonl")]
            DataWriter::Jsonl(w) => w.write_byte_record(rec),
            #[cfg(feature = "parquet")]
            DataWriter::Parquet(w) => w.write_byte_record(rec),
        }
    }

    pub fn write_headers(&mut self, headers: &ByteRecord) -> CliResult<()> {
        match self {
            DataWriter::Csv(w) | DataWriter::Tsv(w) => Ok(w.write_byte_record(headers)?),
            #[cfg(feature = "jsonl")]
            DataWriter::Jsonl(w) => w.set_headers(headers),
            #[cfg(feature = "parquet")]
            DataWriter::Parquet(w) => w.set_schema(headers),
        }
    }

    pub fn flush(&mut self) -> CliResult<()> {
        match self {
            DataWriter::Csv(w) | DataWriter::Tsv(w) => Ok(w.flush()?),
            #[cfg(feature = "jsonl")]
            DataWriter::Jsonl(w) => w.flush(),
            #[cfg(feature = "parquet")]
            DataWriter::Parquet(w) => w.flush(),
        }
    }
}

// === JSONL Reader ===
#[cfg(feature = "jsonl")]
pub struct JsonlReader {
    reader: BufReader<Box<dyn Read>>,
    headers: ByteRecord,
    line: String,
    buf: Vec<u8>,
    no_headers: bool,
}

#[cfg(feature = "jsonl")]
impl JsonlReader {
    pub fn new<R: Read + 'static>(r: R, no_headers: bool) -> CliResult<JsonlReader> {
        let mut reader = JsonlReader {
            reader: BufReader::new(Box::new(r) as Box<dyn Read>),
            headers: ByteRecord::new(),
            line: String::new(),
            buf: Vec::new(),
            no_headers,
        };
        if !no_headers {
            reader.read_headers()?;
        }
        Ok(reader)
    }

    fn read_headers(&mut self) -> CliResult<()> {
        self.line.clear();
        self.reader.read_line(&mut self.line)?;
        let line = self.line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let v: serde_json::Value = serde_json::from_str(line)?;
        let obj = v
            .as_object()
            .ok_or_else(|| CliError::Other("JSONL root must be object".into()))?;
        for key in obj.keys() {
            self.headers.push_field(key.as_bytes());
        }
        self.buf = line.as_bytes().to_vec();
        Ok(())
    }

    pub fn headers(&self) -> CliResult<ByteRecord> {
        Ok(self.headers.clone())
    }

    pub fn read_byte_record(&mut self, rec: &mut ByteRecord) -> CliResult<bool> {
        rec.clear();
        if !self.buf.is_empty() {
            let line = std::mem::take(&mut self.buf);
            self.json_to_record(&line, rec)?;
            return Ok(true);
        }
        loop {
            self.line.clear();
            let n = self.reader.read_line(&mut self.line)?;
            if n == 0 {
                return Ok(false);
            }
            let line = self.line.trim();
            if line.is_empty() {
                continue;
            }
            self.json_to_record(line.as_bytes(), rec)?;
            return Ok(true);
        }
    }

    fn json_to_record(&self, data: &[u8], rec: &mut ByteRecord) -> CliResult<()> {
        let v: serde_json::Value = serde_json::from_slice(data)?;
        let obj = v
            .as_object()
            .ok_or_else(|| CliError::Other("JSONL root must be object".into()))?;
        if self.no_headers {
            for val in obj.values() {
                rec.push_field(&json_val_bytes(val));
            }
        } else {
            for h in self.headers.iter() {
                let key =
                    std::str::from_utf8(h).map_err(|e| CliError::Other(format!("UTF-8: {e}")))?;
                match obj.get(key) {
                    Some(val) => rec.push_field(&json_val_bytes(val)),
                    None => rec.push_field(b""),
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "jsonl")]
fn json_val_bytes(val: &serde_json::Value) -> Vec<u8> {
    match val {
        serde_json::Value::Null => b"".to_vec(),
        serde_json::Value::Bool(b) => (if *b { "true" } else { "false" }).as_bytes().to_vec(),
        serde_json::Value::Number(n) => n.to_string().as_bytes().to_vec(),
        serde_json::Value::String(s) => s.as_bytes().to_vec(),
        _ => val.to_string().as_bytes().to_vec(),
    }
}

// === JSONL Writer ===
#[cfg(feature = "jsonl")]
pub struct JsonlWriter {
    writer: Box<dyn Write>,
    headers: Option<ByteRecord>,
}

#[cfg(feature = "jsonl")]
impl JsonlWriter {
    pub fn new<W: Write + 'static>(w: W) -> JsonlWriter {
        JsonlWriter {
            writer: Box::new(w) as Box<dyn Write>,
            headers: None,
        }
    }

    pub fn set_headers(&mut self, headers: &ByteRecord) -> CliResult<()> {
        self.headers = Some(headers.clone());
        Ok(())
    }

    pub fn write_byte_record(&mut self, rec: &ByteRecord) -> CliResult<()> {
        if let Some(headers) = &self.headers {
            write!(self.writer, "{{")?;
            for (i, h) in headers.iter().enumerate() {
                if i > 0 {
                    write!(self.writer, ",")?;
                }
                let key =
                    std::str::from_utf8(h).map_err(|e| CliError::Other(format!("UTF-8: {e}")))?;
                write!(self.writer, "\"{}\":", key)?;
                if i < rec.len() {
                    let val = std::str::from_utf8(&rec[i])
                        .map_err(|e| CliError::Other(format!("UTF-8: {e}")))?;
                    serde_json::to_writer(
                        &mut self.writer,
                        &serde_json::Value::String(val.to_owned()),
                    )?;
                } else {
                    serde_json::to_writer(&mut self.writer, &serde_json::Value::Null)?;
                }
            }
            writeln!(self.writer, "}}")?;
        } else {
            write!(self.writer, "[")?;
            for (i, f) in rec.iter().enumerate() {
                if i > 0 {
                    write!(self.writer, ",")?;
                }
                let val =
                    std::str::from_utf8(f).map_err(|e| CliError::Other(format!("UTF-8: {e}")))?;
                serde_json::to_writer(
                    &mut self.writer,
                    &serde_json::Value::String(val.to_owned()),
                )?;
            }
            writeln!(self.writer, "]")?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> CliResult<()> {
        Ok(self.writer.flush()?)
    }
}

// === Parquet Reader ===
#[cfg(feature = "parquet")]
pub struct ParquetReader {
    reader: Box<dyn parquet::file::reader::FileReader + Send>,
    headers: ByteRecord,
    headers_read: bool,
    row_group: usize,
    num_row_groups: usize,
    batch_records: Vec<Option<Vec<Vec<u8>>>>,
    row_idx: usize,
}

#[cfg(feature = "parquet")]
impl ParquetReader {
    pub fn from_file(path: &str) -> CliResult<ParquetReader> {
        use parquet::file::reader::FileReader;
        let file = std::fs::File::open(path)?;
        let reader = parquet::file::reader::SerializedFileReader::new(file)?;
        let md = reader.metadata();
        let num_row_groups = md.num_row_groups();
        Ok(ParquetReader {
            reader: Box::new(reader),
            headers: ByteRecord::new(),
            headers_read: false,
            row_group: 0,
            num_row_groups,
            batch_records: Vec::new(),
            row_idx: 0,
        })
    }

    pub fn headers(&mut self) -> CliResult<ByteRecord> {
        if !self.headers_read {
            let md = self.reader.metadata();
            let file_meta = md.file_metadata();
            let schema = file_meta.schema_descr();
            for col in schema.columns() {
                self.headers.push_field(col.name().as_bytes());
            }
            self.headers_read = true;
        }
        Ok(self.headers.clone())
    }

    pub fn read_byte_record(&mut self, rec: &mut ByteRecord) -> CliResult<bool> {
        use parquet::record::Field;
        loop {
            if self.row_idx < self.batch_records.len() {
                rec.clear();
                if let Some(Some(fields)) = self.batch_records.get(self.row_idx) {
                    for f in fields {
                        rec.push_field(f);
                    }
                }
                self.row_idx += 1;
                return Ok(true);
            }
            if self.row_group >= self.num_row_groups {
                return Ok(false);
            }
            let iter = self.reader.get_row_iter(None)?;
            let mut batch = Vec::new();
            for row_res in iter {
                let row = row_res?;
                let mut fields = Vec::new();
                for (_name, field) in row.get_column_iter() {
                    let val = match field {
                        Field::Str(s) => s.as_bytes().to_vec(),
                        Field::Long(v) => v.to_string().into_bytes(),
                        Field::Double(v) => v.to_string().into_bytes(),
                        Field::Int(v) => v.to_string().into_bytes(),
                        Field::Float(v) => v.to_string().into_bytes(),
                        Field::Bool(v) => (if *v { "true" } else { "false" }).as_bytes().to_vec(),
                        Field::Short(v) => v.to_string().into_bytes(),
                        Field::Byte(v) => v.to_string().into_bytes(),
                        Field::Bytes(v) => v.data().to_vec(),
                        Field::Null => Vec::new(),
                        _ => format!("{field}").as_bytes().to_vec(),
                    };
                    fields.push(val);
                }
                batch.push(Some(fields));
            }
            self.batch_records = batch;
            self.row_idx = 0;
            self.row_group += 1;
        }
    }
}

// === Parquet Writer ===
#[cfg(feature = "parquet")]
pub struct ParquetWriter {
    schema_headers: Option<ByteRecord>,
    records: Vec<ByteRecord>,
    output_path: Option<String>,
}

#[cfg(feature = "parquet")]
impl ParquetWriter {
    pub fn new<W: Write + 'static>(_w: W) -> CliResult<ParquetWriter> {
        Ok(ParquetWriter {
            schema_headers: None,
            records: Vec::new(),
            output_path: None,
        })
    }

    pub fn from_path(path: &str) -> CliResult<ParquetWriter> {
        Ok(ParquetWriter {
            schema_headers: None,
            records: Vec::new(),
            output_path: Some(path.to_owned()),
        })
    }

    pub fn set_schema(&mut self, headers: &ByteRecord) -> CliResult<()> {
        self.schema_headers = Some(headers.clone());
        Ok(())
    }

    pub fn write_byte_record(&mut self, rec: &ByteRecord) -> CliResult<()> {
        self.records.push(rec.clone());
        Ok(())
    }

    pub fn flush(&mut self) -> CliResult<()> {
        if self.records.is_empty() {
            return Ok(());
        }
        let hdrs = match &self.schema_headers {
            Some(h) => h.clone(),
            None => {
                let h = self.records[0].clone();
                self.schema_headers = Some(h.clone());
                h
            }
        };
        let ncols = hdrs.len();
        let mut builders: Vec<arrow::array::GenericByteBuilder<arrow::array::types::Utf8Type>> = (0
            ..ncols)
            .map(|_| arrow::array::GenericByteBuilder::<arrow::array::types::Utf8Type>::new())
            .collect();
        for rec in &self.records {
            for (i, b) in builders.iter_mut().enumerate() {
                if i < rec.len() {
                    let s = String::from_utf8_lossy(&rec[i]);
                    b.append_value(&s);
                } else {
                    b.append_null();
                }
            }
        }
        let arrays: Vec<arrow::array::ArrayRef> = builders
            .into_iter()
            .map(|mut b| std::sync::Arc::new(b.finish()) as arrow::array::ArrayRef)
            .collect();
        let field_names: Vec<&str> = hdrs
            .iter()
            .map(|h| std::str::from_utf8(h).unwrap_or("col"))
            .collect();
        let fields: Vec<arrow::datatypes::Field> = field_names
            .iter()
            .map(|n| arrow::datatypes::Field::new(*n, arrow::datatypes::DataType::Utf8, true))
            .collect();
        let schema = arrow::datatypes::Schema::new(fields);
        let batch =
            arrow::record_batch::RecordBatch::try_new(std::sync::Arc::new(schema.clone()), arrays)?;
        if let Some(path) = &self.output_path {
            let file = std::fs::File::create(path)?;
            let mut writer =
                parquet::arrow::ArrowWriter::try_new(file, std::sync::Arc::new(schema), None)?;
            writer.write(&batch)?;
            writer.close()?;
        }
        self.records.clear();
        Ok(())
    }
}
