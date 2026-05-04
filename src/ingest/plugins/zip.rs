// src/ingest/plugins/zip.rs
use super::IngestPlugin;
use crate::ingest::types::*;
use std::path::Path;
use zip::ZipArchive;
use tempfile;

pub struct ZipPlugin;

impl IngestPlugin for ZipPlugin {
    fn name(&self) -> &'static str { "ZipPlugin" }

    fn supported_formats(&self) -> &[DetectedFormat] {
        &[DetectedFormat::Zip]
    }

    fn stream(
        &self,
        path: &Path,
        _meta: &IngestMeta,
    ) -> IngestResult<Box<dyn Iterator<Item = IngestResult<RawRow>> + Send + 'static>> {
        // ZIP: minden belső fájlt temp könyvtárba csomagol ki,
        // majd rekurzívan feldolgoz.
        // LIMIT: max 3 rekurziós szint (ZIP-in-ZIP-in-ZIP ellen).

        let file = std::fs::File::open(path)
            .map_err(|e| IngestError::Io(e.to_string()))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| IngestError::ZipError(e.to_string()))?;

        let tmp = tempfile::tempdir()
            .map_err(|e: std::io::Error| IngestError::Io(e.to_string()))?;

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
        let mut all_rows: Vec<IngestResult<RawRow>> = vec![];
        
        for p in extracted_paths {
            let engine = crate::ingest::IngestEngine::new();
            match engine.open(&p) {
                Ok((_, iter)) => {
                    for row in iter {
                        all_rows.push(row);
                    }
                }
                Err(e) => {
                    all_rows.push(Err(e));
                }
            }
        }

        Ok(Box::new(all_rows.into_iter()))
    }
}
