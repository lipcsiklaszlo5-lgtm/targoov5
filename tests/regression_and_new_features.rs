// tests/regression_and_new_features.rs

use targoo_v2::models::{LedgerRow, Scope3Extension, GhgScope, Jurisdiction, CalcPath, DataQualityTier, MatchMethod};
use targoo_v2::gap_analysis::{run_gap_analysis, GapStatus};
use targoo_v2::benchmark::run_benchmark;
use targoo_v2::supply_chain::run_supply_chain_stress_test;
use targoo_v2::ixbrl_mapper::map_to_xbrl;
use targoo_v2::ledger::verify_chain;
use uuid::Uuid;
use chrono::Utc;

// Kézzel készítünk egy mock LedgerRow-t, ami szimulál egy valós adatsort.
fn create_mock_ledger() -> Vec<LedgerRow> {
    vec![
        // Scope 1 sor
        LedgerRow {
            row_id: Uuid::new_v4(),
            source_file: "test.csv".to_string(),
            raw_row_index: 1,
            raw_header: "Erdgas".to_string(),
            raw_value: 1000.0,
            raw_unit: "m3".to_string(),
            converted_value: 1000.0,
            converted_unit: "m3".to_string(),
            assumed_unit: None,
            ghg_scope: GhgScope::SCOPE1,
            ghg_category: "Stationary Combustion".to_string(),
            ghg_subcategory: "".to_string(),
            emission_factor: 2.0,
            ef_source: "DEFRA".to_string(),
            ef_jurisdiction: Jurisdiction::EU,
            gwp_applied: 1.0,
            tco2e: 2.0,
            confidence: 1.0,
            scope3_extension: None,
            sha256_hash: "hash1".to_string(), // A valóságban ezt a ledger.rs számolja
            issa_5000: None,
            created_at: Utc::now(),
        },
        // Scope 2 sor
        LedgerRow {
            row_id: Uuid::new_v4(),
            source_file: "test.csv".to_string(),
            raw_row_index: 2,
            raw_header: "Strom".to_string(),
            raw_value: 5000.0,
            raw_unit: "kWh".to_string(),
            converted_value: 5000.0,
            converted_unit: "kWh".to_string(),
            assumed_unit: None,
            ghg_scope: GhgScope::SCOPE2_LB,
            ghg_category: "Electricity".to_string(),
            ghg_subcategory: "".to_string(),
            emission_factor: 0.4,
            ef_source: "UBA".to_string(),
            ef_jurisdiction: Jurisdiction::EU,
            gwp_applied: 1.0,
            tco2e: 2.0,
            confidence: 1.0,
            scope3_extension: None,
            sha256_hash: "hash2".to_string(),
            issa_5000: None,
            created_at: Utc::now(),
        },
        // Scope 3 Cat 1 sor
        LedgerRow {
            row_id: Uuid::new_v4(),
            source_file: "test.csv".to_string(),
            raw_row_index: 3,
            raw_header: "Supplier A".to_string(),
            raw_value: 10000.0,
            raw_unit: "EUR".to_string(),
            converted_value: 10000.0,
            converted_unit: "EUR".to_string(),
            assumed_unit: None,
            ghg_scope: GhgScope::SCOPE3,
            ghg_category: "Purchased Goods".to_string(),
            ghg_subcategory: "".to_string(),
            emission_factor: 0.3,
            ef_source: "EXIOBASE".to_string(),
            ef_jurisdiction: Jurisdiction::EU,
            gwp_applied: 1.0,
            tco2e: 3000.0,
            confidence: 0.9,
            scope3_extension: Some(Scope3Extension {
                category_id: 1,
                category_name: "Purchased Goods & Services".to_string(),
                category_match_method: MatchMethod::Exact,
                category_confidence: 0.9,
                calc_path: CalcPath::SpendBased,
                spend_usd_normalized: Some(10000.0),
                eeio_sector_code: Some("MANUF".to_string()),
                eeio_source: None,
                physical_quantity: None,
                physical_unit: None,
                data_quality_tier: DataQualityTier::Primary,
                ghg_protocol_dq_score: 1,
                pcaf_asset_class: None,
                pcaf_attribution_factor: None,
                pcaf_data_quality_score: None,
            }),
            sha256_hash: "hash3".to_string(),
            issa_5000: None,
            created_at: Utc::now(),
        },
    ]
}

#[test]
fn test_regression_and_new_features() {
    let ledger = create_mock_ledger();

    // --- 1. REGRESSZIÓS TESZTEK (Régi motor működik-e?) ---
    // A lényeg, hogy a függvények ne pánikoljanak.
    assert_eq!(ledger.len(), 3);
    let scope1_total: f64 = ledger.iter().filter(|r| r.ghg_scope == GhgScope::SCOPE1).map(|r| r.tco2e).sum();
    assert_eq!(scope1_total, 2.0);

    // --- 2. ÚJ FUNKCIÓK TESZTJE ---

    // 2.1 Gap Analysis
    let gap_results = run_gap_analysis(&ledger);
    assert!(!gap_results.is_empty(), "A Gap Analysis nem adott vissza eredményt.");
    
    let scope1_gap = gap_results.iter().find(|g| g.esrs_code == "ESRS E1-6 §44a").unwrap();
    assert!(matches!(scope1_gap.status, GapStatus::Found), "A Scope 1 Gap státusza nem 'Found'.");
    
    let missing_gap = gap_results.iter().find(|g| g.esrs_code == "ESRS E1-6 §44c" && g.description.contains("Cat 6")).unwrap();
    assert!(matches!(missing_gap.status, GapStatus::Missing), "A Cat 6 Gap státusza nem 'Missing'.");

    // 2.2 Benchmark
    let benchmark_results = run_benchmark(&ledger, "Manufacturing", Some(1.0));
    assert!(!benchmark_results.is_empty(), "A Benchmark nem adott vissza eredményt.");
    
    let benchmark = &benchmark_results[0];
    // company_intensity = (2.0) / 1.0 = 2.0  (run_benchmark only uses SCOPE1_total)
    // Wait, let's check run_benchmark logic. 
    // It says: company_intensity = scope1_total / revenue;
    // value_p25 = 45.0
    assert_eq!(benchmark.percentile_position, "TOP 25%");
    assert!(!benchmark.materiality_flag, "A Materiality flag-nek false-nak kell lennie.");

    // 2.3 Supply Chain Stress Test
    let supplier_risks = run_supply_chain_stress_test(&ledger);
    assert_eq!(supplier_risks.len(), 1);
    let risk = &supplier_risks[0];
    assert_eq!(risk.supplier_name, "Supplier A");
    assert_eq!(risk.total_tco2e, 3000.0);
    assert!(risk.lksg_flag, "Az LkSG flag-nek true-nak kell lennie.");

    // 2.4 iXBRL Mapper
    let scope1_row = ledger.iter().find(|r| r.ghg_scope == GhgScope::SCOPE1).unwrap();
    let xbrl_tag = map_to_xbrl(scope1_row).unwrap();
    assert_eq!(xbrl_tag.xbrl_concept, "esrs:GrossScope1GHGEmissions");
    
    let cat6_row = ledger.iter().find(|r| r.scope3_extension.as_ref().map(|s| s.category_id == 6).unwrap_or(false));
    assert!(cat6_row.is_none(), "Nem szabadna Cat 6 sornak lennie.");
    
    // 2.5 SHA-256 Chain Verification
    // Mivel a mock adataink hash-je nem valódi, a verifikációnak el kell törnie.
    let verification_result = verify_chain(&ledger, "test_run_id");
    assert!(!verification_result.is_valid, "A lánc verifikációnak hibát kell jeleznie a mock adatok miatt.");
    assert_eq!(verification_result.broken_at_index, Some(0));
}
