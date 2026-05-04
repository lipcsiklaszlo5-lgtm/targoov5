// src/ingest/plugins/fwf.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct FixedWidthPlugin;

#[derive(Debug, Deserialize)]
struct FwfSchema {
    pub fields: Vec<FwfFieldDef>,
}

#[derive(Debug, Deserialize)]
struct FwfFieldDef {
    pub name: String,
    pub start: usize,
    pub length: usize,
    pub field_type: Option<String>,
}

impl IngestPlugin for FixedWidthPlugin {
    fn name(&self) -> &'static str { "FixedWidthPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::FixedWidth]
    }

    fn stream(
        &self,
        path: &Path,
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        // Séma betöltése (data/fixed_width_schema.json)
        let schema_path = Path::new("data/fixed_width_schema.json");
        let schema_file = File::open(schema_path)
            .map_err(|e| IngestError::Io(format!("FWF séma hiányzik: {}", e)))?;
        let schema: FwfSchema = serde_json::from_reader(schema_file)
            .map_err(|e| IngestError::CorruptFile(format!("Hibás FWF séma JSON: {}", e)))?;

        let file = File::open(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;
        let reader = BufReader::new(file);
        let source_file = path.to_string_lossy().to_string();

        Ok(Box::new(FwfIter {
            reader,
            schema,
            source_file,
            line_count: 0,
        }))
    }
}

struct FwfIter {
    reader: BufReader<File>,
    schema: FwfSchema,
    source_file: String,
    line_count: u64,
}

impl Iterator for FwfIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => None, // EOF
            Ok(_) => {
                self.line_count += 1;
                let trimmed_line = line.trim_end_matches(['\r', '\n']);
                let mut fields = std::collections::HashMap::new();

                for field_def in &self.schema.fields {
                    let end = field_def.start + field_def.length;
                    let val = if trimmed_line.len() >= end {
                        &trimmed_line[field_def.start..end]
                    } else if trimmed_line.len() > field_def.start {
                        &trimmed_line[field_def.start..]
                    } else {
                        ""
                    };

                    fields.insert(
                        field_def.name.clone(),
                        infer_fwf_field(val.trim(), field_def.field_type.as_deref()),
                    );
                }

                Some(Ok(RawRow {
                    source_line: self.line_count,
                    source_file: self.source_file.clone(),
                    sheet_name: None,
                    fields,
                    raw_bytes: Some(line.into_bytes()),
                }))
            }
            Err(e) => {
                self.line_count += 1;
                Some(Err(IngestError::Io(e.to_string())))
            }
        }
    }
}

fn infer_fwf_field(s: &str, field_type: Option<&str>) -> RawField {
    if s.is_empty() { return RawField::Null; }

    match field_type {
        Some("number") | Some("float") => {
            if let Ok(f) = s.replace(',', ".").parse::<f64>() {
                return RawField::Number(f);
            }
        }
        Some("integer") | Some("int") => {
            if let Ok(i) = s.parse::<i64>() {
                return RawField::Integer(i);
            }
        }
        Some("bool") | Some("boolean") => {
            if s.eq_ignore_ascii_case("true") || s == "1" || s.eq_ignore_ascii_case("y") {
                return RawField::Bool(true);
            }
            if s.eq_ignore_ascii_case("false") || s == "0" || s.eq_ignore_ascii_case("n") {
                return RawField::Bool(false);
            }
        }
        _ => {}
    }

    // Fallback heurisztika ha nincs típus vagy parse hiba
    if let Ok(i) = s.parse::<i64>() { return RawField::Integer(i); }
    if let Ok(f) = s.replace(',', ".").parse::<f64>() { return RawField::Number(f); }
    
    RawField::Text(s.to_string())
}
