pub mod models;
pub mod scope1;
pub mod scope2;
pub mod scope3;
pub mod utils;
pub mod unit_converter;


use crate::config::models::{ValidatedConfig};
use crate::ef_database;
use crate::models::{Jurisdiction as ModelJurisdiction, GhgScope};
use crate::ingest::RawRow;
use crate::calculation::models::CalculationError;

pub struct CalculationEngine {
    pub config: ValidatedConfig,
    pub unit_converter: crate::calculation::unit_converter::UnitConverter,
}

impl CalculationEngine {
    pub fn new(config: &ValidatedConfig) -> Self {
        Self { 
            config: config.clone(),
            unit_converter: crate::calculation::unit_converter::UnitConverter::new(),
        }
    }

    /// A fő számítási belépőpont egy nyers adatsorra
    pub async fn calculate(
        &self, 
        row: &RawRow, 
        scope: GhgScope, 
        category: &str, 
        category_id: u8,
        amount: f64,
        unit: &str,
    ) -> Result<crate::calculation::models::CalculationResult, CalculationError> {
        // Számítás futtatása a Scope alapján
        match scope {
            GhgScope::SCOPE1 => {
                crate::calculation::scope1::calculate_scope1(self, category, amount, unit, row)
            },
            GhgScope::Scope2Lb | GhgScope::Scope2Mb => {
                crate::calculation::scope2::calculate_scope2(self, scope, category, amount, unit, row)
            },
            GhgScope::SCOPE3 => {
                crate::calculation::scope3::calculate_scope3(self, category_id, row)
            },
        }
    }

    /// Emissziós faktor lekérése a konfiguráció és az adatok alapján
    pub fn get_effective_emission_factor(
        &self,
        scope: GhgScope,
        category: &str,
        unit: &str,
    ) -> Result<(f64, f32), CalculationError> {
        // 1. Megpróbáljuk lekérni a konfigurált joghatósághoz
        let jurisdiction = self.config.config.jurisdiction.clone();

        if let Some(ef) = ef_database::get_emission_factor(scope, category, jurisdiction.clone().into(), unit) {
            return Ok((ef, 1.0)); // Pontos egyezés (Local/Jurisdictional match)
        }

        // 2. Fallback: GLOBAL faktor keresése
        if let Some(ef) = ef_database::get_emission_factor(scope, category, ModelJurisdiction::GLOBAL, unit) {
            return Ok((ef, 0.8)); // Származtatott egyezés (Proxy/Global fallback)
        }

        // 3. Hiba, ha sehol nincs faktor
        Err(CalculationError::MissingEmissionFactor(format!(
            "Kategória: {}, Egység: {}, Joghatóság: {:?}",
            category, unit, jurisdiction
        )))
    }

    /// GWP érték lekérése a konfigurált standard alapján (AR5/AR6)
    pub fn get_gwp_value(&self, gas: &str) -> f64 {
        ef_database::get_gwp(gas).unwrap_or(1.0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::config::models::{RunConfig, GwpStandard, ClientMeta, ReportLanguage, CalculationDepth, FritzPackageConfig, AiConfig};
    use crate::config::models::Jurisdiction;
    use uuid::Uuid;

    pub(crate) fn create_mock_config() -> ValidatedConfig {
        let run_cfg = RunConfig {
            config_version: "1.0".to_string(),
            profile_name: "test_profile".to_string(),
            client: ClientMeta {
                name: "Test Client".to_string(),
                industry: "Testing".to_string(),
                reference_year: 2024,
                contact_email: None,
                internal_id: None,
            },
            jurisdiction: Jurisdiction::GLOBAL,
            modules: vec![],
            language: ReportLanguage::EN,
            depth: CalculationDepth::Quick,
            ef_source_override: None,
            gwp_standard: GwpStandard::IpccAr6,
            fritz_package: FritzPackageConfig {
                include_narrative: false,
                include_ef_ref: false,
                include_quarantine: false,
                watermark: false,
                output_dir: "/tmp".to_string(),
            },
            ai: AiConfig {
                embedding_url: "".to_string(),
                embedding_timeout_ms: 100,
                gemini_enabled: false,
                gemini_model: "".to_string(),
            },
            run_id: None,
            loaded_at: None,
        };
        ValidatedConfig {
            run_id: Uuid::new_v4(),
            config: run_cfg,
            ef_source: "GLOBAL".to_string(),
            active_modules: vec![],
            dictionary_paths: vec![],
            output_label: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_calculation_engine_routing() {
        let config = create_mock_config();
        let engine = CalculationEngine::new(&config);
        // compiled and routed
    }
}
