use crate::models::LedgerRow;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierRisk {
    pub supplier_name: String,
    pub total_tco2e: f64,
    pub pct_of_scope3_cat1: f64,
    pub spend_usd: Option<f64>,
    pub emission_intensity: Option<f64>,
    pub risk_tier: RiskTier,
    pub lksg_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskTier {
    Critical,
    High,
    Medium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LksgComplianceRow {
    pub supplier_name: String,
    pub risk_category: String, // "High Risk", "Medium Risk", "Low Risk"
    pub lksg_relevant: bool,
    pub required_action: String, // "Audit szükséges", "Monitoring", "Dokumentáció ellenőrzése"
}

pub fn run_lksg_analysis(ledger: &[LedgerRow]) -> Vec<LksgComplianceRow> {
    let cat1_rows: Vec<_> = ledger.iter().filter(|r| {
        r.scope3_extension.as_ref()
            .map(|s3| s3.category_id == 1)
            .unwrap_or(false)
    }).collect();

    let cat1_total_tco2e: f64 = cat1_rows.iter().map(|r| r.tco2e).sum();
    if cat1_total_tco2e == 0.0 {
        return Vec::new();
    }

    let mut supplier_map: HashMap<String, (f64, f64, String)> = HashMap::new();
    for row in cat1_rows {
        let supplier = row.raw_header.clone();
        let entry = supplier_map.entry(supplier).or_insert((0.0, 0.0, row.ghg_subcategory.clone()));
        entry.0 += row.tco2e;
        if let Some(spend) = row.scope3_extension.as_ref().and_then(|s| s.spend_usd_normalized) {
            entry.1 += spend;
        }
    }

    let high_risk_sectors = ["textil", "bőr", "elektronika", "bányászat", "textile", "leather", "electronics", "mining"];

    let results: Vec<LksgComplianceRow> = supplier_map.into_iter().map(|(name, (tco2e, spend, subcat))| {
        let pct = tco2e / cat1_total_tco2e;
        let intensity = if spend > 0.0 { tco2e / spend } else { 0.0 };
        
        let is_high_risk_sector = high_risk_sectors.iter().any(|&s| 
            name.to_lowercase().contains(s) || subcat.to_lowercase().contains(s)
        );

        let lksg_relevant = is_high_risk_sector || intensity > 0.8 || pct > 0.15; // 0.8 intensity as mock P75

        let risk_category = if lksg_relevant && is_high_risk_sector {
            "High Risk".to_string()
        } else if lksg_relevant {
            "Medium Risk".to_string()
        } else {
            "Low Risk".to_string()
        };

        let required_action = if risk_category == "High Risk" {
            "Audit szükséges".to_string()
        } else if risk_category == "Medium Risk" {
            "Monitoring".to_string()
        } else {
            "Dokumentáció ellenőrzése".to_string()
        };

        LksgComplianceRow {
            supplier_name: name,
            risk_category,
            lksg_relevant,
            required_action,
        }
    }).collect();

    results
}

pub fn run_supply_chain_stress_test(ledger: &[LedgerRow]) -> Vec<SupplierRisk> {
    let cat1_rows: Vec<_> = ledger.iter().filter(|r| {
        r.scope3_extension.as_ref()
            .map(|s3| s3.category_id == 1)
            .unwrap_or(false)
    }).collect();

    let cat1_total_tco2e: f64 = cat1_rows.iter().map(|r| r.tco2e).sum();
    if cat1_total_tco2e == 0.0 {
        return Vec::new();
    }

    let mut supplier_map: HashMap<String, (f64, f64)> = HashMap::new();
    for row in cat1_rows {
        let supplier = row.raw_header.clone(); // Egyszerűsítés: a fejlécből vesszük
        let entry = supplier_map.entry(supplier).or_insert((0.0, 0.0));
        entry.0 += row.tco2e;
        if let Some(spend) = row.scope3_extension.as_ref().and_then(|s| s.spend_usd_normalized) {
            entry.1 += spend;
        }
    }

    let mut risks: Vec<SupplierRisk> = supplier_map.into_iter().map(|(name, (tco2e, spend))| {
        let pct = tco2e / cat1_total_tco2e;
        let intensity = if spend > 0.0 { Some(tco2e / spend) } else { None };
        let risk_tier = if pct > 0.30 { RiskTier::Critical } else if pct > 0.05 { RiskTier::High } else { RiskTier::Medium };
        let lksg_flag = pct > 0.15 || intensity.unwrap_or(0.0) > 500.0; // 500 tCO2e/USD küszöb

        SupplierRisk {
            supplier_name: name,
            total_tco2e: tco2e,
            pct_of_scope3_cat1: pct * 100.0,
            spend_usd: if spend > 0.0 { Some(spend) } else { None },
            emission_intensity: intensity,
            risk_tier,
            lksg_flag,
        }
    }).collect();

    risks.sort_by(|a, b| b.total_tco2e.partial_cmp(&a.total_tco2e).unwrap());
    risks.truncate(10); // Top 10
    risks
}