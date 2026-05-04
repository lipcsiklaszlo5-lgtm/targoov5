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
    fn stream(
        &self,
        path: &Path,
        meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>>;

    /// Opcionális: fejléc-sor vizsgálat formátum-validációhoz.
    fn probe(&self, path: &Path) -> bool {
        let _ = path;
        true
    }
}

pub mod csv;
pub mod xlsx;
pub mod zip;
pub mod fwf;
pub mod pdf;
pub mod json;
pub mod xml;

pub mod xls {
    use super::*;
    pub struct XlsPlugin;
    impl IngestPlugin for XlsPlugin {
        fn name(&self) -> &'static str { "XlsPlugin" }
        fn supported_formats(&self) -> &[crate::ingest::types::DetectedFormat] { &[crate::ingest::types::DetectedFormat::Xls] }
        fn stream(&self, _p: &Path, _m: &IngestMeta) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> { Err(crate::ingest::types::IngestError::UnsupportedFormat("Not implemented".into())) }
    }
}

pub use csv::CsvPlugin;
pub use xlsx::XlsxPlugin;
pub use xls::XlsPlugin;
pub use fwf::FixedWidthPlugin;
pub use pdf::PdfPlugin;
pub use json::JsonPlugin;
pub use xml::XmlPlugin;
pub use zip::ZipPlugin;
