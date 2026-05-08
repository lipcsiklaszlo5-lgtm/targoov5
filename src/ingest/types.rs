use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use chrono;

/// Egyetlen mező értéke a nyers, normalizálatlan bemeneti rétegből.
#[derive(Debug, Clone)]
pub enum RawField {
    Text(String),
    Number(f64),
    Integer(i64),
    Bool(bool),
    Date(String),     // ISO-8601 string, még nem parse-olt
    Null,
}

impl RawField {
    /// Konvertálás String-gé (normalizációs réteghez).
    pub fn to_string_lossy(&self) -> String {
        match self {
            RawField::Text(s)    => s.clone(),
            RawField::Number(n)  => n.to_string(),
            RawField::Integer(i) => i.to_string(),
            RawField::Bool(b)    => b.to_string(),
            RawField::Date(d)    => d.clone(),
            RawField::Null       => String::new(),
        }
    }
}

/// Egyetlen "sor" a nyers bemeneti fájlból.
/// A `fields` HashMap kulcsa az eredeti fejléc (vagy "col_N" ha nincs fejléc).
#[derive(Debug, Clone)]
pub struct RawRow {
    /// Sor sorszáma a forrás fájlban (1-alapú).
    pub source_line: u64,
    /// Forrás fájl azonosítója (pl. ZIP-en belüli path).
    pub source_file: String,
    /// Lap neve (Excel esetén), egyébként None.
    pub sheet_name: Option<String>,
    /// Mezők: fejléc → érték.
    pub fields: HashMap<String, RawField>,
    /// Eredeti, feldolgozatlan sor szövege (debug/audit célra).
    pub raw_bytes: Option<Vec<u8>>,
}

impl RawRow {
    pub fn get_value_as_f64(&self, field_name: &str) -> Option<f64> {
        self.fields.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(field_name))
            .and_then(|(_, v)| match v {
                RawField::Number(n) => Some(*n),
                RawField::Integer(i) => Some(*i as f64),
                RawField::Text(s) => s.parse::<f64>().ok(),
                _ => None,
            })
    }

    pub fn get_spend_amount_and_unit(&self) -> (f64, String) {
        let amount = self.get_value_as_f64("Value")
            .or_else(|| self.get_value_as_f64("Amount"))
            .unwrap_or(0.0);
        let unit = self.fields.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Unit"))
            .and_then(|(_, v)| if let RawField::Text(s) = v { Some(s.clone()) } else { None })
            .unwrap_or_else(|| "EUR".to_string());
        (amount, unit)
    }
}

/// Fájl-szintű metaadat, a stream elején egyszer keletkezik.
#[derive(Debug, Clone)]
pub struct IngestMeta {
    pub file_path: PathBuf,
    pub detected_format: DetectedFormat,
    pub file_size_bytes: u64,
    /// SHA-256 a teljes bemeneti fájlról (audit chain input).
    pub sha256: String,
    pub encoding: String,           // pl. "UTF-8", "Windows-1252"
    pub delimiter: Option<char>,    // CSV/TSV esetén
    pub sheet_names: Vec<String>,   // Excel esetén
    pub total_rows_estimate: Option<u64>,
    pub ingest_started_at: chrono::DateTime<chrono::Utc>,
}

impl IngestMeta {
    pub fn from_path(path: &Path, format: DetectedFormat) -> IngestResult<Self> {
        let metadata = std::fs::metadata(path).map_err(|e| IngestError::Io(e.to_string()))?;
        
        // Alapértelmezett delimiter a formátum alapján
        let delimiter = match format {
            DetectedFormat::Csv => Some(','),
            DetectedFormat::Tsv => Some('\t'),
            _ => None,
        };

        Ok(Self {
            file_path: path.to_path_buf(),
            detected_format: format,
            file_size_bytes: metadata.len(),
            sha256: String::new(), // FIXME: Implement SHA-256 calculation if needed
            encoding: "UTF-8".to_string(),
            delimiter,
            sheet_names: vec![],
            total_rows_estimate: None,
            ingest_started_at: chrono::Utc::now(),
        })
    }
}

/// Feldolgozási hibák — panic-mentes, minden variáns recoverable.
#[derive(Debug, Error, Clone)]
pub enum IngestError {
    #[error("IO hiba: {0}")]
    Io(String),

    #[error("Kódolási hiba (sor {line}): {detail}")]
    Encoding { line: u64, detail: String },

    #[error("Parse hiba (sor {line}, mező '{field}'): {detail}")]
    ParseError { line: u64, field: String, detail: String },

    #[error("Nem támogatott formátum: {0}")]
    UnsupportedFormat(String),

    #[error("Sérült fájl: {0}")]
    CorruptFile(String),

    #[error("PDF szövegkinyerési hiba (oldal {page}): {detail}")]
    PdfExtraction { page: u32, detail: String },

    #[error("ZIP hiba: {0}")]
    ZipError(String),

    #[error("Fejléc hiányzik vagy kétértelmű: {0}")]
    HeaderError(String),

    #[error("Üres fájl vagy üres lap")]
    EmptyInput,

    #[error("Memória limit túllépés: {0}")]
    MemoryLimit(String),
}

pub type IngestResult<T> = Result<T, IngestError>;

/// Karantén rekord: egy hibás sor és a hozzátartozó hiba.
#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    pub source_file: String,
    pub source_line: u64,
    pub raw_content: String,
    pub error: IngestError,
    pub quarantined_at: chrono::DateTime<chrono::Utc>,
}

/// Karantén napló — SQLite `quarantine_log` táblába kerül.
#[derive(Debug, Default, Clone)]
pub struct QuarantineLog {
    pub entries: Vec<QuarantineEntry>,
}

impl QuarantineLog {
    pub fn add(&mut self, entry: QuarantineEntry) {
        self.entries.push(entry);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn error_rate(&self, total_rows: u64) -> f64 {
        if total_rows == 0 { return 0.0; }
        self.entries.len() as f64 / total_rows as f64
    }
}

/// Felismert fájlformátumok.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetectedFormat {
    Csv,
    Tsv,
    Xlsx,
    Xls,
    Xlsm,
    FixedWidth,
    Pdf,
    Json,
    Xml,
    Zip,
    Unknown(String),
}
