use serde::{Deserialize, Serialize};
use super::omnibus_validator::ObligationStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportingScope {
    Basic,      // Scope 1 + Scope 2
    Standard,   // Scope 1 + 2 + Scope 3 (Core categories)
    Full,       // All 15 Scope 3 categories + PCAF 2025
}

pub fn determine_scope(obligation: ObligationStatus, has_financial_data: bool) -> ReportingScope {
    match obligation {
        ObligationStatus::Obligated => {
            if has_financial_data {
                ReportingScope::Full
            } else {
                ReportingScope::Standard
            }
        }
        ObligationStatus::Voluntary => ReportingScope::Basic,
        ObligationStatus::Unknown => ReportingScope::Standard, // Safety default
    }
}
