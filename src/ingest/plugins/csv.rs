// src/ingest/plugins/csv.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use csv::ReaderBuilder;
use std::path::Path;

pub struct CsvPlugin;

impl IngestPlugin for CsvPlugin {
    fn name(&self) -> &'static str { "CsvPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Csv, DetectedFormat::Tsv]
    }

    fn stream(
        &self,
        path: &Path,
        meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        let delimiter = meta.delimiter.unwrap_or(',') as u8;

        // BOM-toleráns, encoding-aware olvasás
        let rdr = ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)           // eltérő hosszúságú sorok OK
            .trim(csv::Trim::All)
            .from_path(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;

        let source_file = path.to_string_lossy().to_string();

        Ok(Box::new(CsvIter {
            rdr,
            source_file,
            line: 0,
        }))
    }
}

struct CsvIter {
    rdr: csv::Reader<std::fs::File>,
    source_file: String,
    line: u64,
}

impl Iterator for CsvIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut record = csv::StringRecord::new();
        match self.rdr.read_record(&mut record) {
            Ok(false) => None, // EOF
            Err(e) => {
                self.line += 1;
                Some(Err(IngestError::ParseError {
                    line: self.line,
                    field: String::new(),
                    detail: e.to_string(),
                }))
            }
            Ok(true) => {
                self.line += 1;
                let headers = self.rdr.headers()
                    .ok()
                    .cloned()
                    .unwrap_or_default();

                let fields = headers.iter()
                    .zip(record.iter())
                    .map(|(h, v)| {
                        let key = if h.is_empty() {
                            format!("col_{}", headers.len())
                        } else {
                            h.to_string()
                        };
                        (key, infer_field(v))
                    })
                    .collect();

                Some(Ok(RawRow {
                    source_line: self.line,
                    source_file: self.source_file.clone(),
                    sheet_name: None,
                    fields,
                    raw_bytes: None,
                }))
            }
        }
    }
}

/// Egyszerű típusinferencia — szöveges marad, ha nem egyértelmű.
fn infer_field(s: &str) -> RawField {
    if s.is_empty() { return RawField::Null; }
    if let Ok(i) = s.parse::<i64>() { return RawField::Integer(i); }
    if let Ok(f) = s.parse::<f64>() { return RawField::Number(f); }
    if s.eq_ignore_ascii_case("true") { return RawField::Bool(true); }
    if s.eq_ignore_ascii_case("false") { return RawField::Bool(false); }
    RawField::Text(s.to_string())
}
