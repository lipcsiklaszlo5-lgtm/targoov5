use super::IngestPlugin;
use crate::ingest::types::*;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use serde_json::{Deserializer, Value};
use std::collections::HashMap;

pub struct JsonPlugin;

impl IngestPlugin for JsonPlugin {
    fn name(&self) -> &'static str {
        "JsonPlugin"
    }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Json]
    }

    fn stream(
        &self,
        path: &Path,
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        let file = File::open(path).map_err(|e| IngestError::Io(e.to_string()))?;
        let reader = BufReader::new(file);
        let source_file = path.to_string_lossy().to_string();

        let iter = Deserializer::from_reader(reader)
            .into_iter::<Value>()
            .enumerate()
            .map(move |(idx, res)| {
                match res {
                    Ok(value) => {
                        let fields = match value {
                            Value::Object(map) => {
                                map.into_iter().map(|(k, v)| (k, json_value_to_raw_field(v))).collect()
                            }
                            _ => {
                                let mut fields = HashMap::new();
                                fields.insert("value".to_string(), json_value_to_raw_field(value));
                                fields
                            }
                        };

                        Ok(RawRow {
                            source_line: (idx + 1) as u64,
                            source_file: source_file.clone(),
                            sheet_name: None,
                            fields,
                            raw_bytes: None,
                        })
                    }
                    Err(e) => {
                        Err(IngestError::ParseError {
                            line: (idx + 1) as u64,
                            field: "json".to_string(),
                            detail: e.to_string(),
                        })
                    }
                }
            });

        Ok(Box::new(iter))
    }
}

fn json_value_to_raw_field(v: Value) -> RawField {
    match v {
        Value::Null => RawField::Null,
        Value::Bool(b) => RawField::Bool(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                RawField::Integer(i)
            } else {
                RawField::Number(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => RawField::Text(s),
        Value::Array(a) => RawField::Text(serde_json::to_string(&a).unwrap_or_default()),
        Value::Object(o) => RawField::Text(serde_json::to_string(&o).unwrap_or_default()),
    }
}
