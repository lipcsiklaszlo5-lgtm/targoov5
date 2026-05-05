use thiserror::Error;
use crate::models::QuarantineReason;

#[derive(Error, Debug, Clone)]
pub enum CalculationError {
    #[error("Hiányzó emissziós faktor: {0}")]
    MissingEmissionFactor(String),

    #[error("Érvénytelen mértékegység konverzió: {0} -> {1}")]
    InvalidUnitConversion(String, String),

    #[error("Range-Guard hiba: {0} meghaladja a biztonsági korlátot")]
    RangeGuardViolation(String),

    #[error("Érvénytelen mértékegység a kategóriához: {0}")]
    InvalidUnit(String),

    #[error("Belső számítási hiba: {0}")]
    InternalError(String),

    #[error("Joghatóság-specifikus validációs hiba: {0}")]
    JurisdictionMismatch(String),
}

/// A számítás végeredménye a bizalmi indexszel együtt
#[derive(Debug, Clone)]
pub struct CalculationResult {
    pub tco2e: f64,
    pub confidence: f32,
}

impl CalculationError {
    /// Átfordítás a globális QuarantineReason típusra a naplózáshoz
    pub fn to_quarantine_reason(&self) -> QuarantineReason {
        match self {
            CalculationError::MissingEmissionFactor(_) => QuarantineReason::MissingEmissionFactor,
            CalculationError::RangeGuardViolation(_) => QuarantineReason::RangeGuardFail,
            _ => QuarantineReason::ParseError,
        }
    }
}
