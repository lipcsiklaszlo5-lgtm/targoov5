use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchemaError {
    #[error("Schema file not found")]
    NotFound(#[from] std::io::Error),
    #[error("Invalid JSON format")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Invalid schema structure")]
    InvalidSchema,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub start: Option<usize>,
    pub length: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Schema {
    pub schema_id: String,
    pub schema_version: String,
    pub description: String,
    pub fields: Vec<Field>,
}

#[derive(Default)]
pub struct SchemaRegistry {
    pub schemas: HashMap<String, Schema>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    pub fn load_schema(&mut self, path: &Path) -> Result<Schema, SchemaError> {
        let content = fs::read_to_string(path)?;
        let schema: Schema = serde_json::from_str(&content)?;
        self.schemas.insert(schema.schema_id.clone(), schema.clone());
        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sample_sap_schema() {
        let mut registry = SchemaRegistry::new();
        let path = Path::new("schemas/registry/sample_sap_export.json");
        let schema = registry.load_schema(path).expect("Failed to load schema");
        assert_eq!(schema.schema_id, "sap_export_v1");
        assert_eq!(schema.fields.len(), 3);
        assert_eq!(schema.fields[0].name, "company_id");
    }
}
