use crate::ingest::{
    plugins::*,
    types::DetectedFormat,
};

pub struct PluginRegistry {
    plugins: Vec<Box<dyn crate::ingest::plugins::IngestPlugin>>,
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
    ) -> Option<&dyn crate::ingest::plugins::IngestPlugin> {
        self.plugins.iter()
            .find(|p| p.supported_formats().contains(format))
            .map(|p| p.as_ref())
    }

    /// Plugin regisztrálása futásidőben (pl. teszteléshez).
    pub fn register(&mut self, plugin: Box<dyn crate::ingest::plugins::IngestPlugin>) {
        self.plugins.push(plugin);
    }
}
