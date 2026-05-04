use super::IngestPlugin;
use crate::ingest::types::*;
use std::path::Path;
use std::collections::HashMap;

pub struct PdfPlugin;

impl IngestPlugin for PdfPlugin {
    fn name(&self) -> &'static str {
        "PdfPlugin"
    }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Pdf]
    }

    fn stream(
        &self,
        path: &Path,
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        // pdf-extract: szöveg kinyerése oldalanként
        let text = pdf_extract::extract_text(path)
            .map_err(|e| IngestError::PdfExtraction {
                page: 0,
                detail: e.to_string(),
            })?;

        let source_file = path.to_string_lossy().to_string();
        let lines: Vec<String> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(String::from)
            .collect();

        Ok(Box::new(PdfIter {
            lines,
            current: 0,
            source_file,
        }))
    }
}

struct PdfIter {
    lines: Vec<String>,
    current: usize,
    source_file: String,
}

impl Iterator for PdfIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.lines.len() {
            return None;
        }
        let line = self.lines[self.current].clone();
        self.current += 1;

        let mut fields = HashMap::new();
        fields.insert("raw_text".to_string(), RawField::Text(line.clone()));

        // Egyszerű kulcs:érték parse (pl. "Lieferant: ACME GmbH")
        if let Some((k, v)) = line.split_once(':') {
            fields.insert(k.trim().to_string(), RawField::Text(v.trim().to_string()));
        }

        Some(Ok(RawRow {
            source_line: self.current as u64,
            source_file: self.source_file.clone(),
            sheet_name: None,
            fields,
            raw_bytes: Some(line.into_bytes()),
        }))
    }
}
