use serde::{Deserialize, Serialize};
use super::omnibus_validator::ObligationStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwissCsaValidator {
    pub employee_count: Option<u32>,
    pub revenue_chf: Option<f64>,
    pub balance_sheet_chf: Option<f64>,
    pub is_public_interest: bool, // Bankok, biztosítók, tőzsdén jegyzett cégek
}

impl SwissCsaValidator {
    pub fn new(
        employee_count: Option<u32>, 
        revenue_chf: Option<f64>, 
        balance_sheet_chf: Option<f64>,
        is_public_interest: bool
    ) -> Self {
        Self {
            employee_count,
            revenue_chf,
            balance_sheet_chf,
            is_public_interest,
        }
    }

    /// Ellenőrzi a svájci klímarendelet (Ordinance on Climate Disclosures) 
    /// szerinti jelentéstételi kötelezettséget.
    pub fn check_obligation(&self) -> ObligationStatus {
        // A törvény alapvetően a közérdeklődésre számot tartó cégekre (pl. bankok, biztosítók) vonatkozik
        // De a Net-Zero célok miatt sok nagyvállalat is érintett.
        // Itt a TCFD rendelet szigorú küszöbértékeit alkalmazzuk.
        
        if !self.is_public_interest {
            return ObligationStatus::Voluntary;
        }

        match (self.employee_count, self.revenue_chf, self.balance_sheet_chf) {
            (Some(employees), Some(revenue), Some(balance)) => {
                if employees >= 500 && (balance >= 20_000_000.0 || revenue >= 40_000_000.0) {
                    ObligationStatus::Obligated
                } else {
                    ObligationStatus::Voluntary
                }
            },
            // Ha a bevétel vagy mérleg hiányzik, de a megadott érték meghaladja a küszöböt
            (Some(employees), Some(revenue), None) if employees >= 500 && revenue >= 40_000_000.0 => {
                ObligationStatus::Obligated
            },
            (Some(employees), None, Some(balance)) if employees >= 500 && balance >= 20_000_000.0 => {
                ObligationStatus::Obligated
            },
            // Nincs elég adat a biztos döntéshez
            (None, _, _) => ObligationStatus::Unknown,
            (Some(employees), None, None) if employees >= 500 => ObligationStatus::Unknown,
            _ => ObligationStatus::Voluntary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swiss_csa_obligation() {
        // 1. Nagy svájci bank (közérdekű, 600 alkalmazott, 50M CHF bevétel) -> Kötelezett
        let bank = SwissCsaValidator::new(Some(600), Some(50_000_000.0), None, true);
        assert_eq!(bank.check_obligation(), ObligationStatus::Obligated);

        // 2. Nagy, de nem közérdekű cég -> Önkéntes
        let private_corp = SwissCsaValidator::new(Some(1000), Some(100_000_000.0), Some(50_000_000.0), false);
        assert_eq!(private_corp.check_obligation(), ObligationStatus::Voluntary);

        // 3. Közérdekű, de kicsi (300 alkalmazott) -> Önkéntes
        let small_bank = SwissCsaValidator::new(Some(300), Some(100_000_000.0), Some(50_000_000.0), true);
        assert_eq!(small_bank.check_obligation(), ObligationStatus::Voluntary);

        // 4. Közérdekű, 500 alkalmazott, csak mérlegadat van (30M CHF) -> Kötelezett
        let insurance = SwissCsaValidator::new(Some(500), None, Some(30_000_000.0), true);
        assert_eq!(insurance.check_obligation(), ObligationStatus::Obligated);

        // 5. Nincs elég adat (ismeretlen létszám) -> Ismeretlen
        let unknown = SwissCsaValidator::new(None, Some(100_000_000.0), Some(50_000_000.0), true);
        assert_eq!(unknown.check_obligation(), ObligationStatus::Unknown);
    }
}
