use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::io::{BufReader, BufRead};
use std::fs::File;

/// Raw representation of a single data row from the source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRow {
    pub source_file: Arc<str>,
    pub row_index: usize, // 0-based index in the sheet/csv
    pub headers: Arc<[Arc<str>]>, // Normalized headers shared across rows
    pub values: Vec<Arc<str>>, // String values corresponding to headers
    pub other_columns: Vec<Arc<str>>, // Non-numeric values for context
    pub raw_line: Arc<str>, // For debugging/quarantine
}

const EXCLUDED_HEADERS: &[&str] = &[
    "id", "company id", "company_id", "companyid", "company", "name", "year", "date", "period",
    "description", "notes", "comment", "source", "row", "index", "id_number",
    "unternehmen", "jahr", "datum", "beschreibung",
    "azonosito", "ceg", "nev", "ev", "leiras"
];

pub struct IngestionEngine {
    // Configuration could go here (e.g., delimiters, encoding hints)
}

impl IngestionEngine {
    pub fn new() -> Self {
        Self {}
    }

    /// Streaming entry point: returns an iterator over Result<RawRow>
    pub fn parse_to_stream(&self, file_path: &Path) -> Result<Box<dyn Iterator<Item = Result<RawRow>> + Send + 'static>> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "csv" => self.stream_csv(file_path),
            "xlsx" | "xls" | "xlsm" => self.stream_excel(file_path),
            _ => Err(anyhow!("Unsupported file format: {}", extension)),
        }
    }

    /// Main entry point: takes a file path, returns a vector of RawRows
    #[deprecated(since = "0.2.0", note = "Use parse_to_stream instead for better memory efficiency")]
    pub fn parse_to_raw_rows(&self, file_path: &Path) -> Result<Vec<RawRow>> {
        self.parse_to_stream(file_path)?
            .collect::<Result<Vec<RawRow>>>()
    }

    fn stream_csv(&self, file_path: &Path) -> Result<Box<dyn Iterator<Item = Result<RawRow>> + Send + 'static>> {
        let file_name: Arc<str> = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
            .into();

        let file = File::open(file_path).context("Failed to open CSV file")?;
        let mut buf_reader = BufReader::new(file);

        // Detect delimiter (semicolon vs comma)
        let mut delimiter = b',';
        {
            let mut first_line = String::new();
            if let Ok(mut br) = File::open(file_path).map(BufReader::new) {
                let _ = br.read_line(&mut first_line);
                let has_comma = first_line.contains(',');
                let has_semicolon = first_line.contains(';');
                if has_semicolon && !has_comma {
                    delimiter = b';';
                } else if has_semicolon && has_comma {
                    let commas = first_line.matches(',').count();
                    let semicolons = first_line.matches(';').count();
                    if semicolons > commas {
                        delimiter = b';';
                    }
                }
            }
        }

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(buf_reader);

        let headers: Arc<[Arc<str>]> = reader
            .headers()
            .context("Failed to read CSV headers")?
            .iter()
            .map(|h| Self::normalize_string(h).into())
            .collect::<Vec<Arc<str>>>()
            .into();

        let header_clone = headers.clone();
        let file_name_clone = file_name.clone();

        let iter = reader.into_records().enumerate().filter_map(move |(idx, result)| {
            let record = match result {
                Ok(r) => r,
                Err(e) => return Some(Err(anyhow!("Failed to parse CSV row {}: {}", idx, e))),
            };

            // Skip completely empty rows
            if record.iter().all(|f| f.trim().is_empty()) {
                return None;
            }

            let values: Vec<Arc<str>> = record.iter().map(|f| Arc::from(f)).collect();
            
            // Pad values to match header length
            let mut padded_values = values;
            while padded_values.len() < header_clone.len() {
                padded_values.push(Arc::from(""));
            }

            // EXTRA PROTECTION: Skip rows that are metadata-only
            let value_col_idx = Self::find_value_column_index_only_arc(&header_clone, &padded_values);
            if value_col_idx.is_none() {
                return None;
            }

            let other_columns = padded_values.iter().enumerate()
                .filter(|(i, _)| Some(*i) != value_col_idx)
                .map(|(_, v)| v.clone())
                .filter(|v| !v.trim().is_empty() && v.len() > 2)
                .collect();

            Some(Ok(RawRow {
                source_file: file_name_clone.clone(),
                row_index: idx,
                headers: header_clone.clone(),
                values: padded_values,
                other_columns,
                raw_line: record.iter().collect::<Vec<&str>>().join(",").into(),
            }))
        });

        Ok(Box::new(iter))
    }

    fn stream_excel(&self, file_path: &Path) -> Result<Box<dyn Iterator<Item = Result<RawRow>> + Send + 'static>> {
        let file_name: Arc<str> = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
            .into();

        let mut workbook = open_workbook_auto(file_path)
            .context("Failed to open Excel file")?;

        let sheet_names = workbook.sheet_names();
        if sheet_names.is_empty() {
            return Err(anyhow!("Excel file contains no sheets"));
        }

        let first_sheet = sheet_names[0].clone();
        let range = workbook
            .worksheet_range(&first_sheet)
            .context(format!("Failed to read sheet '{}'", first_sheet))?;

        // calamine's Range and Rows borrow from workbook/range.
        // To make it 'static, we collect the data into a Vec of RawRows.
        // This is not true streaming for Excel (limited by calamine), 
        // but it satisfies the Iterator requirement and uses Arc<str> to save memory.
        let mut rows_iter = range.rows();
        
        let headers: Arc<[Arc<str>]> = if let Some(first_row) = rows_iter.next() {
            first_row.iter()
                .map(|cell| Self::normalize_string(&Self::cell_to_string(cell)).into())
                .collect::<Vec<Arc<str>>>()
                .into()
        } else {
            return Err(anyhow!("Excel sheet is empty"));
        };

        let mut results = Vec::new();
        for (row_idx, row_cells) in rows_iter.enumerate() {
            let row_values: Vec<Arc<str>> = row_cells
                .iter()
                .map(|cell| Arc::from(Self::cell_to_string(cell)))
                .collect();

            if row_values.iter().all(|v| v.trim().is_empty()) {
                continue;
            }

            let mut padded_values = row_values;
            while padded_values.len() < headers.len() {
                padded_values.push(Arc::from(""));
            }

            let value_col_idx = Self::find_value_column_index_only_arc(&headers, &padded_values);
            if value_col_idx.is_none() {
                continue;
            }

            let other_columns = padded_values.iter().enumerate()
                .filter(|(i, _)| Some(*i) != value_col_idx)
                .map(|(_, v)| v.clone())
                .filter(|v| !v.trim().is_empty() && v.len() > 2)
                .collect();

            results.push(Ok(RawRow {
                source_file: file_name.clone(),
                row_index: row_idx,
                headers: headers.clone(),
                values: padded_values,
                other_columns,
                raw_line: format!("Row {} (Excel)", row_idx + 1).into(),
            }));
        }

        Ok(Box::new(results.into_iter()))
    }

    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Float(f) => f.to_string(),
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(d) => d.to_string(),
            Data::Error(e) => format!("ERROR: {:?}", e),
            _ => String::new(),
        }
    }

    fn normalize_string(input: &str) -> String {
        input
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ")
            .replace('.', " ")
            .replace('/', " ")
            .trim()
            .to_string()
    }

    pub fn find_value_column(row: &RawRow) -> Option<usize> {
        Self::find_value_column_index_only_arc(&row.headers, &row.values)
    }

    pub fn find_value_column_index_only_arc(headers: &[Arc<str>], values: &[Arc<str>]) -> Option<usize> {
        let value_keywords = [
            "value", "wert", "amount", "betrag", "emission", "menge", "quantity",
            "total", "sum", "co2", "tco2", "kgco2", "kwh", "usd", "eur", "gbp",
            "cost", "spend", "consumption", "verbrauch", "fogyasztás",
        ];

        for (idx, header) in headers.iter().enumerate() {
            let norm_header = Self::normalize_string(header);
            if EXCLUDED_HEADERS.iter().any(|ex| norm_header.contains(ex)) || norm_header.contains("id") {
                continue;
            }
            if value_keywords.iter().any(|kw| norm_header.contains(kw)) {
                if idx < values.len() {
                    if Self::is_potentially_numeric(&values[idx]) {
                        return Some(idx);
                    }
                }
            }
        }
        
        // Fallback search
        for (idx, val) in values.iter().enumerate() {
            if idx < headers.len() {
                let norm_header = Self::normalize_string(&headers[idx]);
                if EXCLUDED_HEADERS.iter().any(|ex| norm_header.contains(ex)) || norm_header.contains("id") {
                    continue;
                }
            }
            if Self::is_potentially_numeric(val) {
                return Some(idx);
            }
        }
        None
    }

    fn is_potentially_numeric(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }
        let cleaned = s
            .replace('$', "")
            .replace('€', "")
            .replace('£', "")
            .replace(',', "")
            .replace(' ', "")
            .replace("~", "")
            .replace("k", "000") 
            .replace("K", "000");
        cleaned.parse::<f64>().is_ok()
    }
}

impl Default for IngestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_numeric_cell(raw: &str) -> Option<f64> {
    if raw.is_empty() {
        return None;
    }

    let cleaned = raw
        .replace('$', "")
        .replace('€', "")
        .replace('£', "")
        .replace("USD", "")
        .replace("EUR", "")
        .replace("GBP", "")
        .replace(',', "")
        .replace('\'', "")
        .replace(' ', "")
        .replace("~", "")
        .replace("k", "e3")
        .replace("K", "e3")
        .replace("m", "e6")
        .replace("M", "e6");

    let final_cleaned = if cleaned.contains(',') && cleaned.contains('.') {
        if cleaned.rfind(',').unwrap_or(0) > cleaned.rfind('.').unwrap_or(0) {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if cleaned.contains(',') && !cleaned.contains('.') {
        if cleaned.matches(',').count() > 1 {
            cleaned.replace(',', "")
        } else {
            cleaned.replace(',', ".")
        }
    } else {
        cleaned
    };

    final_cleaned.parse::<f64>().ok()
}

pub fn is_excluded_header(header: &str) -> bool {
    let normalized = header.to_lowercase().replace('_', " ").replace('-', " ").replace('.', " ").replace('/', " ").trim().to_string();
    EXCLUDED_HEADERS.iter().any(|ex| normalized.contains(ex))
}
