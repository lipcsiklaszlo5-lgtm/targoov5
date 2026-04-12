use anyhow::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Raw representation of a single data row from the source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawRow {
    pub source_file: String,
    pub row_index: usize, // 0-based index in the sheet/csv
    pub headers: Vec<String>, // Normalized headers from the first row
    pub values: Vec<String>, // String values corresponding to headers
    pub raw_line: String, // For debugging/quarantine
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

    /// Main entry point: takes a file path, returns a vector of RawRows
    pub fn parse_to_raw_rows(&self, file_path: &Path) -> Result<Vec<RawRow>> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "csv" => self.parse_csv(file_path),
            "xlsx" | "xls" | "xlsm" => self.parse_excel(file_path),
            _ => Err(anyhow!("Unsupported file format: {}", extension)),
        }
    }

    fn parse_csv(&self, file_path: &Path) -> Result<Vec<RawRow>> {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_path(file_path)
            .context("Failed to open CSV file")?;

        let headers: Vec<String> = reader
            .headers()
            .context("Failed to read CSV headers")?
            .iter()
            .map(|h| Self::normalize_string(h))
            .collect();

        let mut rows = Vec::new();
        for (idx, result) in reader.records().enumerate() {
            let record = result.context(format!("Failed to parse CSV row {}", idx))?;
            
            // Skip completely empty rows
            if record.iter().all(|f| f.trim().is_empty()) {
                continue;
            }

            let values: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            
            // Pad values to match header length (handles trailing empty columns)
            let mut padded_values = values;
            padded_values.resize(headers.len(), String::new());

            // EXTRA PROTECTION: Skip rows that are metadata-only
            if Self::find_value_column_index_only(&headers, &padded_values).is_none() {
                continue;
            }

            rows.push(RawRow {
                source_file: file_name.clone(),
                row_index: idx,
                headers: headers.clone(),
                values: padded_values,
                raw_line: record.iter().collect::<Vec<&str>>().join(","),
            });
        }

        Ok(rows)
    }

    fn parse_excel(&self, file_path: &Path) -> Result<Vec<RawRow>> {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut workbook = open_workbook_auto(file_path)
            .context("Failed to open Excel file")?;

        let sheet_names = workbook.sheet_names();
        if sheet_names.is_empty() {
            return Err(anyhow!("Excel file contains no sheets"));
        }

        // Process the first sheet only (standard for ESG data dumps)
        let first_sheet = &sheet_names[0];
        let range = workbook
            .worksheet_range(first_sheet)
            .context(format!("Failed to read sheet '{}'", first_sheet))?;

        let mut rows = Vec::new();
        let mut headers: Vec<String> = Vec::new();
        let mut header_set = false;

        for (row_idx, row_cells) in range.rows().enumerate() {
            let row_values: Vec<String> = row_cells
                .iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    Data::String(s) => s.clone(),
                    Data::Float(f) => f.to_string(),
                    Data::Int(i) => i.to_string(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(d) => d.to_string(),
                    Data::Error(e) => format!("ERROR: {:?}", e),
                    _ => String::new(),
                })
                .collect();

            // Skip completely empty rows
            if row_values.iter().all(|v: &String| v.trim().is_empty()) {
                continue;
            }

            if !header_set {
                headers = row_values.iter().map(|h| Self::normalize_string(h)).collect();
                header_set = true;
                continue;
            }

            // Pad values to match header length
            let mut padded_values = row_values;
            padded_values.resize(headers.len(), String::new());

            // EXTRA PROTECTION: Skip rows that are metadata-only
            if Self::find_value_column_index_only(&headers, &padded_values).is_none() {
                continue;
            }

            rows.push(RawRow {
                source_file: file_name.clone(),
                row_index: row_idx - 1, // 0-based data index (excluding header)
                headers: headers.clone(),
                values: padded_values,
                raw_line: format!("Row {} (Excel)", row_idx),
            });
        }

        Ok(rows)
    }

    /// Normalize strings for header matching: lowercase, trim, underscores to spaces
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

    /// Heuristically finds the column index containing numeric values for a given row
    /// This is used when the file has multiple descriptive columns and we need to find the "value" column.
    pub fn find_value_column(row: &RawRow) -> Option<usize> {
        // Common value column header keywords
        let value_keywords = [
            "value", "wert", "amount", "betrag", "emission", "menge", "quantity",
            "total", "sum", "co2", "tco2", "kgco2", "kwh", "usd", "eur", "gbp",
            "cost", "spend", "consumption", "verbrauch", "fogyasztás",
        ];

        // 1. Try to find by header keyword, strictly excluding metadata columns
        for (idx, header) in row.headers.iter().enumerate() {
            let norm_header = Self::normalize_string(header);
            
            // IF header contains ANY excluded keyword, it CANNOT be the value column
            if EXCLUDED_HEADERS.iter().any(|ex| norm_header.contains(ex)) || norm_header.contains("id") {
                continue;
            }

            if value_keywords.iter().any(|kw| norm_header.contains(kw)) {
                // Check if the corresponding value is parseable as a number
                if idx < row.values.len() {
                    let val = &row.values[idx];
                    if Self::is_potentially_numeric(val) {
                        return Some(idx);
                    }
                }
            }
        }

        // 2. Fallback: find the first column with a numeric-looking value that is NOT in excluded headers
        for (idx, val) in row.values.iter().enumerate() {
            if idx < row.headers.len() {
                let norm_header = Self::normalize_string(&row.headers[idx]);
                // If the header of this column contains an excluded keyword or 'id', skip it
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

    /// Helper for early row skipping: finds value column index from headers and values only
    pub fn find_value_column_index_only(headers: &[String], values: &[String]) -> Option<usize> {
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
        None
    }

    /// Checks if a string can be parsed as a number (after cleaning currency/separators)
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
            .replace("k", "000") // Crude but effective for "48k"
            .replace("K", "000");
        cleaned.parse::<f64>().is_ok()
    }
}

impl Default for IngestionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses a raw string value into an f64, handling currency and thousand separators
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
        .replace('\'', "") // Swiss/German thousand separator
        .replace(' ', "")
        .replace("~", "")
        .replace("k", "e3")
        .replace("K", "e3")
        .replace("m", "e6")
        .replace("M", "e6");

    // Handle European decimal comma (1.234,56 -> 1234.56)
    // This logic assumes if there is a comma AND a period, period is thousand sep.
    let final_cleaned = if cleaned.contains(',') && cleaned.contains('.') {
        // If both exist, assume comma is decimal if it is the last one
        if cleaned.rfind(',').unwrap() > cleaned.rfind('.').unwrap() {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if cleaned.contains(',') && !cleaned.contains('.') {
        // Only comma exists. If there are multiple, it's a thousand sep.
        if cleaned.matches(',').count() > 1 {
            cleaned.replace(',', "")
        } else {
            // Single comma: could be thousand or decimal. Assume decimal.
            cleaned.replace(',', ".")
        }
    } else {
        cleaned
    };

    final_cleaned.parse::<f64>().ok()
}

/// Helper to check if a header is in the excluded metadata list
pub fn is_excluded_header(header: &str) -> bool {
    let normalized = header.to_lowercase().replace('_', " ").replace('-', " ").replace('.', " ").replace('/', " ").trim().to_string();
    EXCLUDED_HEADERS.iter().any(|ex| normalized.contains(ex))
}
