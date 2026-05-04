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
        &self,
        path: &Path,
    ) -> IngestResult<(IngestMeta, Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>)> {
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
