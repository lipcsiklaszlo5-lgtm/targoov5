use super::IngestPlugin;
use crate::ingest::types::*;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use quick_xml::reader::Reader;
use quick_xml::events::Event;

pub struct XmlPlugin;

impl IngestPlugin for XmlPlugin {
    fn name(&self) -> &'static str {
        "XmlPlugin"
    }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Xml]
    }

    fn stream(
        &self,
        path: &Path,
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        let file = File::open(path).map_err(|e| IngestError::Io(e.to_string()))?;
        let reader = BufReader::new(file);
        let mut xml_reader = Reader::from_reader(reader);
        xml_reader.config_mut().trim_text(true);

        let source_file = path.to_string_lossy().to_string();

        Ok(Box::new(XmlIter {
            reader: xml_reader,
            source_file,
            line: 0,
            depth: 0,
            finished: false,
            buf: Vec::new(),
        }))
    }
}

struct XmlIter {
    reader: Reader<BufReader<File>>,
    source_file: String,
    line: u64,
    depth: usize,
    finished: bool,
    buf: Vec<u8>,
}

impl Iterator for XmlIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf) {
                Ok(Event::Start(ref e)) => {
                    self.depth += 1;
                    if self.depth == 2 {
                        // Level 2 element: Treat as a row
                        let row_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                        match self.parse_row(&row_name) {
                            Ok(row) => return Some(Ok(row)),
                            Err(e) => return Some(Err(e)),
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    self.depth -= 1;
                }
                Ok(Event::Eof) => {
                    self.finished = true;
                    return None;
                }
                Err(e) => {
                    self.finished = true;
                    return Some(Err(IngestError::ParseError {
                        line: self.line,
                        field: "xml".to_string(),
                        detail: e.to_string(),
                    }));
                }
                _ => {}
            }
        }
    }
}

impl XmlIter {
    /// Parses a "row" element (depth 2) and its immediate children.
    fn parse_row(&mut self, row_name: &str) -> IngestResult<RawRow> {
        self.line += 1;
        let mut fields = HashMap::new();
        let mut current_field: Option<String> = None;
        let mut row_buf = Vec::new();

        loop {
            row_buf.clear();
            match self.reader.read_event_into(&mut row_buf) {
                Ok(Event::Start(ref e)) => {
                    current_field = Some(String::from_utf8_lossy(e.name().as_ref()).to_string());
                    
                    // Also parse attributes as fields
                    for attr in e.attributes().flatten() {
                        let key = format!("{}_{}", current_field.as_ref().unwrap(), String::from_utf8_lossy(attr.key.as_ref()));
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        fields.insert(key, RawField::Text(val));
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if let Some(ref field_name) = current_field {
                        let val = e.unescape().map(|c| c.into_owned()).unwrap_or_else(|_| String::from_utf8_lossy(e.as_ref()).to_string());
                        fields.insert(field_name.clone(), infer_field(&val));
                    }
                }
                Ok(Event::End(ref e)) => {
                    if String::from_utf8_lossy(e.name().as_ref()) == row_name {
                        self.depth -= 1;
                        return Ok(RawRow {
                            source_line: self.line,
                            source_file: self.source_file.clone(),
                            sheet_name: None,
                            fields,
                            raw_bytes: None,
                        });
                    }
                    current_field = None;
                }
                Ok(Event::Empty(ref e)) => {
                    // Self-closing tag
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    for attr in e.attributes().flatten() {
                        let key = format!("{}_{}", name, String::from_utf8_lossy(attr.key.as_ref()));
                        let val = String::from_utf8_lossy(&attr.value).to_string();
                        fields.insert(key, RawField::Text(val));
                    }
                }
                Ok(Event::Eof) => {
                    return Err(IngestError::CorruptFile("Unexpected EOF in XML row".to_string()));
                }
                Err(e) => {
                    return Err(IngestError::ParseError {
                        line: self.line,
                        field: row_name.to_string(),
                        detail: e.to_string(),
                    });
                }
                _ => {}
            }
        }
    }
}

fn infer_field(s: &str) -> RawField {
    if s.is_empty() { return RawField::Null; }
    if let Ok(i) = s.parse::<i64>() { return RawField::Integer(i); }
    if let Ok(f) = s.parse::<f64>() { return RawField::Number(f); }
    if s.eq_ignore_ascii_case("true") { return RawField::Bool(true); }
    if s.eq_ignore_ascii_case("false") { return RawField::Bool(false); }
    RawField::Text(s.to_string())
}
