// src/ingest/plugins/xlsx.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use calamine::{open_workbook_auto, Data, Reader, Sheets};
use std::path::Path;

pub struct XlsxPlugin;

impl IngestPlugin for XlsxPlugin {
    fn name(&self) -> &'static str { "XlsxPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Xlsx, DetectedFormat::Xlsm]
    }

    fn stream(
        &self,
        path: &Path,
        meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {

        let source_file = path.to_string_lossy().to_string();

        let mut workbook: Sheets<_> = open_workbook_auto(path)
            .map_err(|e| IngestError::CorruptFile(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let first_sheet = sheet_names.first()
            .ok_or(IngestError::EmptyInput)?
            .clone();

        let range = workbook.worksheet_range(&first_sheet)
            .map_err(|e| IngestError::CorruptFile(e.to_string()))?;

        let rows: Vec<Vec<Data>> = range.rows()
            .map(|r| r.to_vec())
            .collect();

        let iter = XlsxIter {
            rows,
            headers: vec![],
            current: 0,
            source_file,
            sheet_name: first_sheet,
        };

        Ok(Box::new(iter))
    }
}

struct XlsxIter {
    rows: Vec<Vec<Data>>,
    headers: Vec<String>,
    current: usize,
    source_file: String,
    sheet_name: String,
}

impl Iterator for XlsxIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        // Fejléc inicializálás
        if self.headers.is_empty() {
            if let Some(header_row) = self.rows.get(0) {
                self.headers = header_row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let s = cell.to_string().trim().to_string();
                        if s.is_empty() { format!("col_{}", i) } else { s }
                    })
                    .collect();
                self.current = 1;
            } else {
                return None;
            }
        }

        if self.current >= self.rows.len() { return None; }

        let row = &self.rows[self.current];
        self.current += 1;

        let fields = self.headers.iter()
            .zip(row.iter().chain(std::iter::repeat(&Data::Empty)))
            .map(|(h, cell)| (h.clone(), data_to_raw_field(cell)))
            .collect();

        Some(Ok(RawRow {
            source_line: self.current as u64,
            source_file: self.source_file.clone(),
            sheet_name: Some(self.sheet_name.clone()),
            fields,
            raw_bytes: None,
        }))
    }
}

fn data_to_raw_field(dt: &Data) -> RawField {
    match dt {
        Data::Empty         => RawField::Null,
        Data::String(s)     => RawField::Text(s.clone()),
        Data::Float(f)      => RawField::Number(*f),
        Data::Int(i)        => RawField::Integer(*i),
        Data::Bool(b)       => RawField::Bool(*b),
        Data::DateTime(f)   => RawField::Date(format!("{}", f)),
        Data::Error(_)      => RawField::Null,
        Data::DateTimeIso(s) => RawField::Date(s.clone()),
        Data::DurationIso(s) => RawField::Text(s.clone()),
    }
}
