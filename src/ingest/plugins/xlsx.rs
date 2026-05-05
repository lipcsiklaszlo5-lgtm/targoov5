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
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {

        let source_file = path.to_string_lossy().to_string();

        let workbook: Sheets<std::io::BufReader<std::fs::File>> = open_workbook_auto(path)
            .map_err(|e| IngestError::CorruptFile(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();

        let iter = XlsxIter {
            workbook,
            sheet_names,
            current_sheet_idx: 0,
            rows: Vec::new(),
            current_row_idx: 0,
            headers: vec![],
            source_file,
        };

        Ok(Box::new(iter))
    }
}

struct XlsxIter {
    workbook: Sheets<std::io::BufReader<std::fs::File>>,
    sheet_names: Vec<String>,
    current_sheet_idx: usize,
    rows: Vec<Vec<Data>>,
    current_row_idx: usize,
    headers: Vec<String>,
    source_file: String,
}

impl Iterator for XlsxIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Ha nincsenek betöltött sorok (vagy a végére értünk), töltsük be a következő lapot
            if self.current_row_idx >= self.rows.len() {
                if self.current_sheet_idx >= self.sheet_names.len() {
                    return None; // Nincs több lap
                }
                
                let sheet_name = &self.sheet_names[self.current_sheet_idx];
                
                match self.workbook.worksheet_range(sheet_name) {
                    Ok(range) => {
                        self.rows = range.rows().map(|r| r.to_vec()).collect();
                    },
                    Err(_) => {
                        self.rows = Vec::new();
                    }
                }
                
                self.current_row_idx = 0;
                self.headers.clear();
                self.current_sheet_idx += 1;
                continue; // Kezdjük el feldolgozni a most betöltött lapot
            }

            // Fejléc inicializálás az új lapon
            if self.headers.is_empty() && !self.rows.is_empty() {
                if let Some(header_row) = self.rows.get(self.current_row_idx) {
                    self.headers = header_row.iter()
                        .enumerate()
                        .map(|(i, cell)| {
                            let s = cell.to_string().trim().to_string();
                            if s.is_empty() { format!("col_{}", i) } else { s }
                        })
                        .collect();
                    self.current_row_idx += 1; // Átlépjük a fejlécet
                    continue;
                }
            }

            let row = &self.rows[self.current_row_idx];
            let row_line_num = self.current_row_idx + 1; // 1-alapú indexelés a hibakereséshez
            self.current_row_idx += 1;

            // Üres sorok átugrása
            if row.iter().all(|c| c == &Data::Empty || c.to_string().trim().is_empty()) {
                continue;
            }

            let sheet_name = self.sheet_names[self.current_sheet_idx - 1].clone();

            let fields = self.headers.iter()
                .zip(row.iter().chain(std::iter::repeat(&Data::Empty)))
                .map(|(h, cell)| (h.clone(), data_to_raw_field(cell)))
                .collect();

            return Some(Ok(RawRow {
                source_line: row_line_num as u64,
                source_file: self.source_file.clone(),
                sheet_name: Some(sheet_name),
                fields,
                raw_bytes: None,
            }));
        }
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
