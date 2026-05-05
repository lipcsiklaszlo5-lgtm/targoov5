pub mod models;
pub mod scope1;
pub mod scope2;
pub mod scope3;

use crate::config::models::ValidatedConfig;
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
}
