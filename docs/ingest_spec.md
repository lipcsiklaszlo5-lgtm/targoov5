# Targoo V2 — Universal Ingestion Engine Specification

**Verzió:** 1.0.0  
**Dátum:** 2026-05-04  
**Státusz:** DRAFT — implementációra kész  
**Modul:** `src/ingest/`

-----

## 1. Összefoglaló

Az Universal Ingestion Engine (UIE) a Targoo V2 belépési rétege. Feladata: bármilyen formátumú, koszos vállalati fájlt egységes `RawRow` stream-mé alakítani, amelyet a normalizációs pipeline tovább dolgoz fel. A motor panic-mentes, streaming-alapú, plugin-architektúrájú, és a 4 vCPU / 16 GB RAM Codespaces/Kaggle korláthoz igazított.

-----

## 2. Architektúra áttekintés

```
┌─────────────────────────────────────────────────────────────────┐
│                     Külső bemenetek                             │
│  CSV  TSV  XLSX  XLS  XLSM  TXT(FW)  PDF  JSON  XML  ZIP       │
└────────────────────────┬────────────────────────────────────────┘
                         │  <fájl path / byte stream>
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  FormatDetector                                  │
│  1. Magic byte olvasás (első 8 byte)                            │
│  2. Kiterjesztés fallback                                        │
│  3. Tartalom-alapú heurisztika (UTF-8 scan, XML prolog, stb.)   │
└────────────────────────┬────────────────────────────────────────┘
                         │  DetectedFormat enum
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│                  PluginRegistry                                  │
│  plugin_for(format) → Box<dyn IngestPlugin>                │
└────────────────────────┬────────────────────────────────────────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
    CsvPlugin      XlsxPlugin      PdfPlugin   ...
    TsvPlugin      XlsPlugin       JsonPlugin
                   XlsmPlugin      XmlPlugin
                   FwfPlugin       ZipPlugin (rekurzív)
                         │
                         ▼
          ┌──────────────────────────────┐
          │  Iterator<Item = IngestResult<RawRow>>  │
          └──────────────┬───────────────┘
                         │
          ┌──────────────┴───────────────┐
          ▼                              ▼
   RawRow (OK path)             QuarantineLog (hibás sor)
          │
          ▼
   Normalizációs pipeline (következő réteg)
```

-----

## 3. Fájlstruktúra

```
src/
└── ingest/
    ├── mod.rs              # pub use, IngestEngine belépési pont
    ├── types.rs            # RawField, RawRow, IngestMeta, IngestError, stb.
    ├── detector.rs         # FormatDetector, DetectedFormat
    ├── registry.rs         # PluginRegistry
    ├── quarantine.rs       # QuarantineLog, QuarantineWriter
    └── plugins/
        ├── mod.rs          # pub use minden plugin
        ├── csv.rs          # CsvPlugin (+ TSV)
        ├── xlsx.rs         # XlsxPlugin (XLSX, XLSM)
        ├── xls.rs          # XlsPlugin (XLS legacy)
        ├── fwf.rs          # FixedWidthPlugin
        ├── pdf.rs          # PdfPlugin
        ├── json.rs         # JsonPlugin
        ├── xml.rs          # XmlPlugin
        └── zip.rs          # ZipPlugin (rekurzív)
```

-----

## 4. Rust típusdefiníciók

### 4.1 `src/ingest/types.rs`

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

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
#[derive(Debug, Default)]
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
```

-----

### 4.2 `src/ingest/mod.rs`

```rust
pub mod detector;
pub mod plugins;
pub mod quarantine;
pub mod registry;
pub mod types;

pub use types::*;
use detector::FormatDetector;
use registry::PluginRegistry;
use std::path::Path;

/// Fő belépési pont. Fájlt streamel RawRow iterátorként.
pub struct IngestEngine {
    registry: PluginRegistry,
    pub quarantine: QuarantineLog,
}

impl IngestEngine {
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::default(),
            quarantine: QuarantineLog::default(),
        }
    }

    /// Fájl betöltése streaming módban.
    /// Visszaad: (IngestMeta, impl Iterator<Item = IngestResult<RawRow>>)
    pub fn open(
        &mut self,
        path: &Path,
    ) -> IngestResult<(IngestMeta, Box<dyn Iterator<Item = IngestResult<RawRow>> + '_>)> {
        let format = FormatDetector::detect(path)?;
        let meta = IngestMeta::from_path(path, format.clone())?;
        let plugin = self.registry.plugin_for(&format)
            .ok_or_else(|| IngestError::UnsupportedFormat(format!("{:?}", format)))?;

        let iter = plugin.stream(path, &meta)?;
        Ok((meta, iter))
    }

    /// Kényelmi metódus: összes sort összegyűjti,
    /// hibás sorokat karanténba helyezi, OK sorokat visszaadja.
    pub fn ingest_all(&mut self, path: &Path) -> IngestResult<(IngestMeta, Vec<RawRow>)> {
        let (meta, stream) = self.open(path)?;
        let mut rows = Vec::new();

        for result in stream {
            match result {
                Ok(row) => rows.push(row),
                Err(e) => {
                    self.quarantine.add(QuarantineEntry {
                        source_file: path.to_string_lossy().to_string(),
                        source_line: 0, // plugin tölti fel
                        raw_content: String::new(),
                        error: e,
                        quarantined_at: chrono::Utc::now(),
                    });
                }
            }
        }

        Ok((meta, rows))
    }
}
```

-----

## 5. Plugin trait

### `src/ingest/plugins/mod.rs`

```rust
use crate::ingest::types::{IngestMeta, IngestResult, RawRow};
use std::path::Path;

/// Minden fájlformátum-plugin ezt a trait-et implementálja.
pub trait IngestPlugin: Send + Sync {
    /// A plugin neve (debug/log célra).
    fn name(&self) -> &'static str;

    /// Támogatott formátumok listája.
    fn supported_formats(&self) -> &[crate::ingest::types::DetectedFormat];

    /// Streaming iterator. Soronként ad vissza RawRow-t.
    /// SOHA nem pánikol — hibás sort Err(IngestError)-ként adja vissza.
    fn stream<'a>(
        &'a self,
        path: &'a Path,
        meta: &'a IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + 'a>>;

    /// Opcionális: fejléc-sor vizsgálat formátum-validációhoz.
    fn probe(&self, path: &Path) -> bool {
        let _ = path;
        true
    }
}

pub mod csv;
pub mod xlsx;
pub mod xls;
pub mod fwf;
pub mod pdf;
pub mod json;
pub mod xml;
pub mod zip;

pub use csv::CsvPlugin;
pub use xlsx::XlsxPlugin;
pub use xls::XlsPlugin;
pub use fwf::FixedWidthPlugin;
pub use pdf::PdfPlugin;
pub use json::JsonPlugin;
pub use xml::XmlPlugin;
pub use zip::ZipPlugin;
```

-----

## 6. Plugin implementációk (vázlatok)

### 6.1 CSV/TSV Plugin

```rust
// src/ingest/plugins/csv.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use csv::ReaderBuilder;
use std::path::Path;

pub struct CsvPlugin;

impl IngestPlugin for CsvPlugin {
    fn name(&self) -> &'static str { "CsvPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Csv, DetectedFormat::Tsv]
    }

    fn stream<'a>(
        &'a self,
        path: &'a Path,
        meta: &'a IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + 'a>> {
        let delimiter = meta.delimiter.unwrap_or(',') as u8;

        // BOM-toleráns, encoding-aware olvasás
        let rdr = ReaderBuilder::new()
            .delimiter(delimiter)
            .flexible(true)           // eltérő hosszúságú sorok OK
            .trim(csv::Trim::All)
            .from_path(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;

        let source_file = path.to_string_lossy().to_string();

        Ok(Box::new(CsvIter {
            rdr,
            source_file,
            line: 1,
        }))
    }
}

struct CsvIter {
    rdr: csv::Reader<std::fs::File>,
    source_file: String,
    line: u64,
}

impl Iterator for CsvIter {
    type Item = IngestResult<RawRow>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut record = csv::StringRecord::new();
        match self.rdr.read_record(&mut record) {
            Ok(false) => None, // EOF
            Err(e) => {
                self.line += 1;
                Some(Err(IngestError::ParseError {
                    line: self.line,
                    field: String::new(),
                    detail: e.to_string(),
                }))
            }
            Ok(true) => {
                self.line += 1;
                let headers = self.rdr.headers()
                    .ok()
                    .cloned()
                    .unwrap_or_default();

                let fields = headers.iter()
                    .zip(record.iter())
                    .map(|(h, v)| {
                        let key = if h.is_empty() {
                            format!("col_{}", headers.len())
                        } else {
                            h.to_string()
                        };
                        (key, infer_field(v))
                    })
                    .collect();

                Some(Ok(RawRow {
                    source_line: self.line,
                    source_file: self.source_file.clone(),
                    sheet_name: None,
                    fields,
                    raw_bytes: None,
                }))
            }
        }
    }
}

/// Egyszerű típusinferencia — szöveges marad, ha nem egyértelmű.
fn infer_field(s: &str) -> RawField {
    if s.is_empty() { return RawField::Null; }
    if let Ok(i) = s.parse::<i64>() { return RawField::Integer(i); }
    if let Ok(f) = s.parse::<f64>() { return RawField::Number(f); }
    if s.eq_ignore_ascii_case("true") { return RawField::Bool(true); }
    if s.eq_ignore_ascii_case("false") { return RawField::Bool(false); }
    RawField::Text(s.to_string())
}
```

-----

### 6.2 XLSX Plugin

```rust
// src/ingest/plugins/xlsx.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use calamine::{open_workbook_auto, DataType, Reader, Xlsx};
use std::path::Path;

pub struct XlsxPlugin;

impl IngestPlugin for XlsxPlugin {
    fn name(&self) -> &'static str { "XlsxPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Xlsx, DetectedFormat::Xlsm]
    }

    fn stream<'a>(
        &'a self,
        path: &'a Path,
        _meta: &'a IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + 'a>> {
        let source_file = path.to_string_lossy().to_string();

        // calamine: csak az első lapot streameli (memória-hatékony)
        // Teljes munkafüzet feldolgozáshoz: lapok iterálása
        let mut workbook: Xlsx<_> = open_workbook_auto(path)
            .map_err(|e| IngestError::CorruptFile(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let first_sheet = sheet_names.first()
            .ok_or(IngestError::EmptyInput)?
            .clone();

        let range = workbook.worksheet_range(&first_sheet)
            .map_err(|e| IngestError::CorruptFile(e.to_string()))?;

        let rows: Vec<Vec<DataType>> = range.rows()
            .map(|r| r.to_vec())
            .collect();

        let iter = XlsxIter {
            rows,
            headers: vec![],
            current: 1, // 0 = fejléc sor
            source_file,
            sheet_name: first_sheet,
        };

        Ok(Box::new(iter))
    }
}

struct XlsxIter {
    rows: Vec<Vec<DataType>>,
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
            if let Some(header_row) = self.rows.first() {
                self.headers = header_row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let s = cell.to_string().trim().to_string();
                        if s.is_empty() { format!("col_{}", i) } else { s }
                    })
                    .collect();
            } else {
                return None;
            }
        }

        if self.current >= self.rows.len() { return None; }

        let row = &self.rows[self.current];
        self.current += 1;

        let fields = self.headers.iter()
            .zip(row.iter().chain(std::iter::repeat(&DataType::Empty)))
            .map(|(h, cell)| (h.clone(), datatype_to_raw_field(cell)))
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

fn datatype_to_raw_field(dt: &DataType) -> RawField {
    match dt {
        DataType::Empty         => RawField::Null,
        DataType::String(s)     => RawField::Text(s.clone()),
        DataType::Float(f)      => RawField::Number(*f),
        DataType::Int(i)        => RawField::Integer(*i),
        DataType::Bool(b)       => RawField::Bool(*b),
        DataType::DateTime(f)   => RawField::Date(format!("{}", f)),
        DataType::Error(_)      => RawField::Null,
        DataType::DateTimeIso(s) => RawField::Date(s.clone()),
        DataType::DurationIso(s) => RawField::Text(s.clone()),
    }
}
```

-----

### 6.3 PDF Plugin

```rust
// src/ingest/plugins/pdf.rs
// Számlák és beszállítói nyilatkozatok szövegkinyerése.
// Estratégia: pdf-extract crate (lopdf wrapper), oldalanként → sorok.

use super::IngestPlugin;
use crate::ingest::types::*;
use std::path::Path;

pub struct PdfPlugin;

impl IngestPlugin for PdfPlugin {
    fn name(&self) -> &'static str { "PdfPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Pdf]
    }

    fn stream<'a>(
        &'a self,
        path: &'a Path,
        _meta: &'a IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + 'a>> {
        // pdf-extract: szöveg kinyerése oldalanként
        let text = pdf_extract::extract_text(path)
            .map_err(|e| IngestError::PdfExtraction { page: 0, detail: e.to_string() })?;

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
        if self.current >= self.lines.len() { return None; }
        let line = self.lines[self.current].clone();
        self.current += 1;

        let mut fields = std::collections::HashMap::new();
        fields.insert("raw_text".to_string(), RawField::Text(line.clone()));

        // Egyszerű kulcs:érték parse (pl. "Lieferant: ACME GmbH")
        if let Some((k, v)) = line.split_once(':') {
            fields.insert(
                k.trim().to_string(),
                RawField::Text(v.trim().to_string()),
            );
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
```

-----

### 6.4 ZIP Plugin (rekurzív)

```rust
// src/ingest/plugins/zip.rs
use super::IngestPlugin;
use crate::ingest::{detector::FormatDetector, registry::PluginRegistry, types::*};
use std::{io::Read, path::Path};
use zip::ZipArchive;

pub struct ZipPlugin;

impl IngestPlugin for ZipPlugin {
    fn name(&self) -> &'static str { "ZipPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Zip]
    }

    fn stream<'a>(
        &'a self,
        path: &'a Path,
        _meta: &'a IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + 'a>> {
        // ZIP: minden belső fájlt temp könyvtárba csomagol ki,
        // majd rekurzívan feldolgoz.
        // LIMIT: max 3 rekurziós szint (ZIP-in-ZIP-in-ZIP ellen).

        let file = std::fs::File::open(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| IngestError::ZipError(e.to_string()))?;

        let tmp = tempfile::tempdir()
            .map_err(|e| IngestError::Io(e.to_string()))?;

        let mut extracted_paths: Vec<std::path::PathBuf> = vec![];

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| IngestError::ZipError(e.to_string()))?;
            if entry.is_file() {
                let outpath = tmp.path().join(entry.name());
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| IngestError::Io(e.to_string()))?;
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| IngestError::Io(e.to_string()))?;
                std::io::copy(&mut entry, &mut outfile)
                    .map_err(|e| IngestError::Io(e.to_string()))?;
                extracted_paths.push(outpath);
            }
        }

        // Rekurzív feldolgozás: minden kicsomagolt fájlt egy IngestEngine-nek ad
        // A tényleges implementáció lazy iterátorra épül
        let rows: Vec<IngestResult<RawRow>> = extracted_paths
            .iter()
            .flat_map(|p| {
                let mut engine = crate::ingest::IngestEngine::new();
                match engine.open(p) {
                    Ok((_, iter)) => iter.collect::<Vec<_>>(),
                    Err(e) => vec![Err(e)],
                }
            })
            .collect();

        Ok(Box::new(rows.into_iter()))
    }
}
```

-----

## 7. Format Detector

### `src/ingest/detector.rs`

```rust
use crate::ingest::types::{DetectedFormat, IngestError, IngestResult};
use std::{io::Read, path::Path};

pub struct FormatDetector;

/// Magic byte minták
const XLSX_MAGIC: &[u8] = &[0x50, 0x4B, 0x03, 0x04]; // PK\x03\x04 (ZIP alapú)
const XLS_MAGIC:  &[u8] = &[0xD0, 0xCF, 0x11, 0xE0]; // OLE2
const PDF_MAGIC:  &[u8] = b"%PDF";

impl FormatDetector {
    pub fn detect(path: &Path) -> IngestResult<DetectedFormat> {
        let magic = Self::read_magic(path)?;

        // 1. Magic byte alapú felismerés
        if magic.starts_with(XLS_MAGIC) {
            return Ok(DetectedFormat::Xls);
        }
        if magic.starts_with(PDF_MAGIC) {
            return Ok(DetectedFormat::Pdf);
        }
        if magic.starts_with(XLSX_MAGIC) {
            // ZIP → lehet XLSX, XLSM, vagy sima ZIP
            // Kiterjesztés alapján döntünk
            return Ok(Self::zip_subtype(path));
        }

        // 2. Kiterjesztés alapú fallback
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "csv"  => return Ok(DetectedFormat::Csv),
                "tsv"  => return Ok(DetectedFormat::Tsv),
                "txt"  => return Ok(Self::probe_text(path)),
                "json" => return Ok(DetectedFormat::Json),
                "xml"  => return Ok(DetectedFormat::Xml),
                "zip"  => return Ok(DetectedFormat::Zip),
                _      => {}
            }
        }

        // 3. Tartalom-alapú heurisztika
        Self::content_probe(path, &magic)
    }

    fn read_magic(path: &Path) -> IngestResult<Vec<u8>> {
        let mut f = std::fs::File::open(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;
        let mut buf = [0u8; 8];
        let n = f.read(&mut buf).map_err(|e| IngestError::Io(e.to_string()))?;
        Ok(buf[..n].to_vec())
    }

    fn zip_subtype(path: &Path) -> DetectedFormat {
        match path.extension().and_then(|e| e.to_str()) {
            Some("xlsx") => DetectedFormat::Xlsx,
            Some("xlsm") => DetectedFormat::Xlsm,
            _            => DetectedFormat::Zip,
        }
    }

    /// Fixed-width vs sima CSV: ha nincs vessző/pontosvessző → FWF
    fn probe_text(path: &Path) -> DetectedFormat {
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut sample = vec![0u8; 512];
            if f.read(&mut sample).is_ok() {
                let s = String::from_utf8_lossy(&sample);
                if s.contains(',') { return DetectedFormat::Csv; }
                if s.contains('\t') { return DetectedFormat::Tsv; }
                return DetectedFormat::FixedWidth;
            }
        }
        DetectedFormat::Unknown("txt".to_string())
    }

    fn content_probe(path: &Path, magic: &[u8]) -> IngestResult<DetectedFormat> {
        // XML prolog
        if magic.starts_with(b"<?xml") || magic.starts_with(b"<") {
            return Ok(DetectedFormat::Xml);
        }
        // JSON
        if magic.starts_with(b"{") || magic.starts_with(b"[") {
            return Ok(DetectedFormat::Json);
        }
        // Egyéb: CSV-nek tekintjük
        Ok(DetectedFormat::Csv)
    }
}
```

-----

## 8. Plugin Registry

### `src/ingest/registry.rs`

```rust
use crate::ingest::{
    plugins::*,
    types::DetectedFormat,
};

pub struct PluginRegistry {
    plugins: Vec<Box<dyn super::plugins::IngestPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self {
            plugins: vec![
                Box::new(CsvPlugin),
                Box::new(XlsxPlugin),
                Box::new(XlsPlugin),
                Box::new(FixedWidthPlugin),
                Box::new(PdfPlugin),
                Box::new(JsonPlugin),
                Box::new(XmlPlugin),
                Box::new(ZipPlugin),
            ],
        }
    }
}

impl PluginRegistry {
    pub fn plugin_for(
        &self,
        format: &DetectedFormat,
    ) -> Option<&dyn super::plugins::IngestPlugin> {
        self.plugins.iter()
            .find(|p| p.supported_formats().contains(format))
            .map(|p| p.as_ref())
    }

    /// Plugin regisztrálása futásidőben (pl. teszteléshez).
    pub fn register(&mut self, plugin: Box<dyn super::plugins::IngestPlugin>) {
        self.plugins.push(plugin);
    }
}
```

-----

## 9. Quarantine Writer (SQLite)

### `src/ingest/quarantine.rs`

```rust
use crate::ingest::types::{QuarantineEntry, QuarantineLog};
use rusqlite::{params, Connection};

/// Karantén log mentése az `esg_state` SQLite adatbázisba.
pub fn flush_quarantine(conn: &Connection, log: &QuarantineLog) -> rusqlite::Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS quarantine_log (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            source_file      TEXT    NOT NULL,
            source_line      INTEGER NOT NULL,
            raw_content      TEXT,
            error_type       TEXT    NOT NULL,
            error_detail     TEXT    NOT NULL,
            quarantined_at   TEXT    NOT NULL
        );
    ")?;

    let mut stmt = conn.prepare("
        INSERT INTO quarantine_log
            (source_file, source_line, raw_content, error_type, error_detail, quarantined_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
    ")?;

    for entry in &log.entries {
        stmt.execute(params![
            entry.source_file,
            entry.source_line,
            entry.raw_content,
            format!("{:?}", std::mem::discriminant(&entry.error)),
            entry.error.to_string(),
            entry.quarantined_at.to_rfc3339(),
        ])?;
    }

    Ok(())
}
```

-----

## 10. Cargo.toml függőségek

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# CSV parsing (streaming, BOM-toleráns)
csv = "1.3"

# Excel (XLSX, XLS, XLSM) — pure Rust, nincs natív függőség
calamine = { version = "0.24", features = ["dates"] }

# PDF szövegkinyerés
pdf-extract = "0.7"

# JSON streaming
serde_json = "1"
serde = { version = "1", features = ["derive"] }

# XML streaming (SAX-stílusú, kis memória)
quick-xml = { version = "0.36", features = ["serialize"] }

# ZIP
zip = { version = "2", default-features = false, features = ["deflate"] }

# Temp fájlok (ZIP kicsomagoláshoz)
tempfile = "3"

# Hiba-kezelés (thiserror — panic-mentes)
thiserror = "1"
anyhow = "1"    # magasabb szintű hibaok-láncoláshoz

# SHA-256 (audit chain)
sha2 = "0.10"
hex = "0.4"

# Dátum/idő
chrono = { version = "0.4", features = ["serde"] }

# Párhuzamos feldolgozás (Rayon — Codespaces 4 vCPU-hoz igazítva)
rayon = "1.10"

# Kódolás-konverzió (Windows-1252 / Latin-2 ERP exportok)
encoding_rs = "0.8"
encoding_rs_io = "0.1"

# SQLite (WORM audit log, quarantine_log)
rusqlite = { version = "0.31", features = ["bundled"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile = "3"
assert_matches = "1.5"
```

-----

## 11. Memória-kezelési stratégia (platform korlátok)

|Formátum|Stratégia                         |Max memória terhelés    |
|--------|----------------------------------|------------------------|
|CSV/TSV |`csv::Reader` — soronkénti olvasás|~1 KB/sor               |
|XLSX    |`calamine` Range iterátor         |~50 MB/lap (16GB-nál OK)|
|XLS     |`calamine` teljes betöltés        |max ~30 MB              |
|PDF     |Teljes szöveg String              |~10 MB/dokumentum       |
|JSON    |`serde_json::StreamDeserializer`  |~1 KB/objektum          |
|XML     |`quick-xml` SAX parser            |~1 KB/elem              |
|ZIP     |Kicsomagolás tempdir-be           |Fájl méretének 2x       |
|FWF     |BufReader soronként               |~1 KB/sor               |

**Kaggle (13 GB tárhely)**: ZIP kicsomagolás közbülső fájljai korlátozhatják a kapacitást. Megoldás: `TMPDIR` env változó `/kaggle/working/`-re állítva.

-----

## 12. Hibakezelési elvek

1. **Nulla panic** — minden `unwrap()` és `expect()` helyett `?` operátor és `IngestError`.
1. **Karantén, nem leállás** — ha egy sor hibás, a `QuarantineLog`-ba kerül, a feldolgozás folytatódik.
1. **Error rate küszöb** — ha `quarantine.error_rate() > 0.5` (>50% hibás sor), az `IngestEngine` figyelmeztetést logol, de nem áll le.
1. **Részleges fájl** — csonkított XLSX/ZIP esetén az eddig kinyert sorok megmaradnak.
1. **Encoding fallback lánc**: UTF-8 → BOM detektálás → Windows-1252 → Latin-2 → veszteséges konverzió.

-----

## 13. CLI integráció (meglévő –input kapcsolóhoz)

```rust
// main.rs vagy cli.rs kiegészítése
use targoo_v2::ingest::IngestEngine;

fn run_ingestion(input_path: &std::path::Path) -> anyhow::Result<()> {
    let mut engine = IngestEngine::new();
    let (meta, rows) = engine.ingest_all(input_path)?;

    tracing::info!(
        file = %meta.file_path.display(),
        format = ?meta.detected_format,
        rows = rows.len(),
        quarantined = engine.quarantine.entries.len(),
        "Ingestion complete"
    );

    if !engine.quarantine.is_empty() {
        tracing::warn!(
            count = engine.quarantine.entries.len(),
            rate = %format!("{:.1}%", engine.quarantine.error_rate(rows.len() as u64 + engine.quarantine.entries.len() as u64) * 100.0),
            "Quarantined rows"
        );
    }

    // Tovább: normalizációs pipeline
    // normalize_rows(rows, &meta, &db_conn)?;

    Ok(())
}
```

-----

## 14. Tesztelési vázlat

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_csv_streaming() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "Lieferant,Menge_kWh,Scope").unwrap();
        writeln!(f, "ACME GmbH,1500.5,1").unwrap();
        writeln!(f, ",,").unwrap(); // üres sor
        writeln!(f, "Bosch AG,bad_number,2").unwrap();

        let mut engine = IngestEngine::new();
        let (meta, rows) = engine.ingest_all(f.path()).unwrap();

        assert_eq!(meta.detected_format, DetectedFormat::Csv);
        assert!(!rows.is_empty());
        // Üres sor ne okozzon pánikot
    }

    #[test]
    fn test_format_detector_xlsx() {
        // A ZIP magic byte + .xlsx kiterjesztés → Xlsx
        let format = FormatDetector::detect(
            std::path::Path::new("tests/fixtures/sample.xlsx")
        ).unwrap();
        assert_eq!(format, DetectedFormat::Xlsx);
    }

    #[test]
    fn test_quarantine_on_corrupt_row() {
        // Hibás encoding → karanténba kerül, nem panic
        let mut engine = IngestEngine::new();
        // ... fixture setup ...
        assert!(engine.quarantine.entries.len() >= 0); // nem pánikol
    }
}
```

-----

## 15. Következő lépések

|Prioritás|Feladat                                  |Megjegyzés                |
|---------|-----------------------------------------|--------------------------|
|P0       |`CsvPlugin` + `XlsxPlugin` implementálása|ERP exportok 90%-a        |
|P0       |`FormatDetector` magic byte tesztelése   |                          |
|P1       |`QuarantineLog` → SQLite flush           |WORM audit chain          |
|P1       |Encoding detection (`encoding_rs`)       |SAP Windows-1252 exportok |
|P2       |`PdfPlugin` → PDF számlák                |Scope 3 beszállítói adatok|
|P2       |`ZipPlugin` rekurzió mélység limit       |max 3 szint               |
|P3       |`JsonPlugin` + `XmlPlugin`               |API/EDI integrációkhoz    |
|P3       |Rayon párhuzamos ZIP kicsomagolás        |4 vCPU kihasználása       |

-----

*Targoo V2 — Universal Ingestion Engine Spec v1.0.0 | 2026-05-04*
