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

    fn content_probe(_path: &Path, magic: &[u8]) -> IngestResult<DetectedFormat> {
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
