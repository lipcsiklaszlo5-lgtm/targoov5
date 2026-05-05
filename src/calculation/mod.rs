pub mod models;
pub mod utils;
pub mod scope1;
pub mod scope2;
pub mod scope3;

use crate::config::models::{ValidatedConfig, GwpStandard};
use crate::ef_database;
use crate::models::{Jurisdiction as ModelJurisdiction, GhgScope};
use crate::ingest::RawRow;
use crate::models::LedgerRow;
use crate::calculation::models::CalculationError;

pub struct CalculationEngine<'a> {
    pub config: &'a ValidatedConfig,
}

impl<'a> CalculationEngine<'a> {
    pub fn new(config: &'a ValidatedConfig) -> Self {
        Self { config }
    }

    /// A fő számítási belépőpont egy nyers adatsorra
    pub async fn calculate(&self, _row: &RawRow) -> Result<LedgerRow, CalculationError> {
        // TODO: Implement calculation logic based on config depth and jurisdiction
        unimplemented!("A számítási logika implementálása a következő fázisban történik.")
    }

    /// GWP érték lekérése a konfigurált szabvány szerint (AR5/AR6)
    pub fn get_gwp_value(&self, gas: &str) -> f64 {
        let suffix = match self.config.config.gwp_standard {
            GwpStandard::IpccAr5 => "_AR5",
            GwpStandard::IpccAr6 => "_AR6",
        };
        let gas_with_standard = format!("{}{}", gas, suffix);
        
        ef_database::get_gwp(&gas_with_standard)
            .or_else(|| ef_database::get_gwp(gas)) // Fallback az alapértelmezett (AR6) értékre
            .unwrap_or(1.0) // Ha semmi nincs, CO2-nek vesszük (GWP=1)
    }

    /// Emissziós faktor lekérése fallback logikával (Jurisdiction -> GLOBAL)
    pub fn get_effective_emission_factor(
        &self,
        scope: GhgScope,
        category: &str,
        unit: &str,
    ) -> Result<f64, CalculationError> {
        // 1. Próbálkozás a konfigurált joghatósággal
        let jur = match self.config.config.jurisdiction {
            crate::config::models::Jurisdiction::DE => ModelJurisdiction::DE,
            crate::config::models::Jurisdiction::AT => ModelJurisdiction::AT,
            crate::config::models::Jurisdiction::CH => ModelJurisdiction::CH,
            crate::config::models::Jurisdiction::HU => ModelJurisdiction::HU,
            crate::config::models::Jurisdiction::EU => ModelJurisdiction::EU,
            crate::config::models::Jurisdiction::UK => ModelJurisdiction::UK,
            crate::config::models::Jurisdiction::US => ModelJurisdiction::US,
            crate::config::models::Jurisdiction::GLOBAL => ModelJurisdiction::GLOBAL,
        };

        if let Some(ef) = ef_database::get_emission_factor(scope, category, jur, unit) {
            return Ok(ef);
        }

        // 2. Fallback a GLOBAL-ra, ha még nem az volt
        if jur != ModelJurisdiction::GLOBAL {
            if let Some(ef) = ef_database::get_emission_factor(scope, category, ModelJurisdiction::GLOBAL, unit) {
                return Ok(ef);
            }
        }

        // 3. Ha nincs találat
        Err(CalculationError::MissingEmissionFactor(format!(
            "Scope: {:?}, Category: {}, Unit: {}", scope, category, unit
        )))
    }
}
