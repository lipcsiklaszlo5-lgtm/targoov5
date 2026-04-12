use rust_xlsxwriter::*;

fn main() -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Headers
    let headers = [
        "test_id", "header_original", "value", "unit", "expected_scope", 
        "expected_cat_id", "expected_calc_path", "expected_unit_category", 
        "language", "jurisdiction"
    ];

    for (col, header) in headers.iter().enumerate() {
        worksheet.write(0, col as u16, *header)?;
    }

    let test_cases = vec![
        // SCOPE 1
        ("S1-01", "Natural Gas", "50000", "kWh", "SCOPE1", "", "ActivityBased", "energy", "EN", "US"),
        ("S1-02", "Diesel fuel", "1000", "liters", "SCOPE1", "", "ActivityBased", "volume", "EN", "UK"),
        ("S1-03", "Gasoline consumption", "500", "gallons", "SCOPE1", "", "ActivityBased", "volume", "EN", "US"),
        ("S1-04", "LPG heating", "2000", "kg", "SCOPE1", "", "ActivityBased", "mass", "EN", "EU"),
        ("S1-05", "Erdgas", "15000", "kWh", "SCOPE1", "", "ActivityBased", "energy", "DE", "EU"),
        ("S1-06", "Diesel Kraftstoff", "800", "liter", "SCOPE1", "", "ActivityBased", "volume", "DE", "EU"),
        ("S1-07", "Földgáz", "12000", "kWh", "SCOPE1", "", "ActivityBased", "energy", "HU", "EU"),
        ("S1-08", "Dízel üzemanyag", "600", "liter", "SCOPE1", "", "ActivityBased", "volume", "HU", "EU"),
        ("S1-09", "R410A refrigerant", "5", "kg", "SCOPE1", "", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S1-10", "SF6 leakage", "0.5", "kg", "SCOPE1", "", "ActivityBased", "mass", "EN", "GLOBAL"),

        // SCOPE 2
        ("S2-01", "Electricity usage", "10000", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "EN", "US"),
        ("S2-02", "Purchased Steam", "500", "GJ", "SCOPE2_LB", "", "ActivityBased", "energy", "EN", "US"),
        ("S2-03", "District Heating", "2000", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "EN", "EU"),
        ("S2-04", "Stromverbrauch", "8000", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "DE", "EU"),
        ("S2-05", "Fernwärme", "1500", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "DE", "EU"),
        ("S2-06", "Áramfogyasztás", "7000", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "HU", "EU"),
        ("S2-07", "Vásárolt gőz", "300", "GJ", "SCOPE2_LB", "", "ActivityBased", "energy", "HU", "EU"),
        ("S2-08", "Grid electricity", "50", "MWh", "SCOPE2_LB", "", "ActivityBased", "energy", "EN", "UK"),

        // SCOPE 3
        // CAT 1
        ("S3-C1-01", "Procurement spend", "150000", "USD", "SCOPE3", "1", "SpendBased", "currency", "EN", "US"),
        ("S3-C1-02", "Raw materials", "5000", "kg", "SCOPE3", "1", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S3-C1-03", "Anyagköltség", "2500000", "HUF", "SCOPE3", "1", "SpendBased", "currency", "HU", "EU"),
        // CAT 2
        ("S3-C2-01", "CAPEX machinery", "75000", "USD", "SCOPE3", "2", "SpendBased", "currency", "EN", "US"),
        ("S3-C2-02", "Investitionsgüter", "45000", "EUR", "SCOPE3", "2", "SpendBased", "currency", "DE", "EU"),
        ("S3-C2-03", "Gépbeszerzés", "12000000", "HUF", "SCOPE3", "2", "SpendBased", "currency", "HU", "EU"),
        // CAT 3
        ("S3-C3-01", "Well-to-tank", "25000", "kWh", "SCOPE3", "3", "ActivityBased", "energy", "EN", "GLOBAL"),
        ("S3-C3-02", "Upstream natural gas", "10000", "kWh", "SCOPE3", "3", "ActivityBased", "energy", "EN", "EU"),
        ("S3-C3-03", "Hálózati veszteség", "5000", "kWh", "SCOPE3", "3", "ActivityBased", "energy", "HU", "EU"),
        // CAT 4
        ("S3-C4-01", "Inbound freight HGV", "12000", "tkm", "SCOPE3", "4", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C4-02", "Sea freight import", "8000", "tkm", "SCOPE3", "4", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C4-03", "Bejövő fuvar", "1500", "tkm", "SCOPE3", "4", "ActivityBased", "distance", "HU", "EU"),
        // CAT 5
        ("S3-C5-01", "Landfill waste", "5000", "kg", "SCOPE3", "5", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S3-C5-02", "Recycling paper", "2000", "kg", "SCOPE3", "5", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S3-C5-03", "Sondermüll", "500", "kg", "SCOPE3", "5", "ActivityBased", "mass", "DE", "EU"),
        // CAT 6
        ("S3-C6-01", "Flights business class", "25000", "km", "SCOPE3", "6", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C6-02", "Hotel nights", "120", "nights", "SCOPE3", "6", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C6-03", "Repülőjegy", "5000", "km", "SCOPE3", "6", "ActivityBased", "distance", "HU", "GLOBAL"),
        // CAT 7
        ("S3-C7-01", "Commuting car", "18000", "km", "SCOPE3", "7", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C7-02", "Work from home", "5000", "hours", "SCOPE3", "7", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C7-03", "Ingázás", "12000", "km", "SCOPE3", "7", "ActivityBased", "distance", "HU", "EU"),
        // CAT 8
        ("S3-C8-01", "Leased office", "45000", "USD", "SCOPE3", "8", "SpendBased", "currency", "EN", "US"),
        ("S3-C8-02", "Gemietete Bürofläche", "30000", "EUR", "SCOPE3", "8", "SpendBased", "currency", "DE", "EU"),
        ("S3-C8-03", "Bérelt iroda", "5000000", "HUF", "SCOPE3", "8", "SpendBased", "currency", "HU", "EU"),
        // CAT 9
        ("S3-C9-01", "Outbound delivery", "9000", "tkm", "SCOPE3", "9", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C9-02", "Last mile shipping", "2000", "tkm", "SCOPE3", "9", "ActivityBased", "distance", "EN", "GLOBAL"),
        ("S3-C9-03", "Kiszállítás", "4500", "tkm", "SCOPE3", "9", "ActivityBased", "distance", "HU", "EU"),
        // CAT 10
        ("S3-C10-01", "Downstream processing", "30000", "USD", "SCOPE3", "10", "SpendBased", "currency", "EN", "US"),
        ("S3-C10-02", "Customer manufacturing", "20000", "EUR", "SCOPE3", "10", "SpendBased", "currency", "DE", "EU"),
        ("S3-C10-03", "Továbbfeldolgozás", "8000000", "HUF", "SCOPE3", "10", "SpendBased", "currency", "HU", "EU"),
        // CAT 11
        ("S3-C11-01", "Product energy use", "120000", "kWh", "SCOPE3", "11", "ActivityBased", "energy", "EN", "GLOBAL"),
        ("S3-C11-02", "Sold appliances", "50000", "kWh", "SCOPE3", "11", "ActivityBased", "energy", "EN", "GLOBAL"),
        ("S3-C11-03", "Termékhasználat", "15000", "kWh", "SCOPE3", "11", "ActivityBased", "energy", "HU", "EU"),
        // CAT 12
        ("S3-C12-01", "Product disposal", "3500", "kg", "SCOPE3", "12", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S3-C12-02", "EoL recycling", "1500", "kg", "SCOPE3", "12", "ActivityBased", "mass", "EN", "GLOBAL"),
        ("S3-C12-03", "Életciklus végi kezelés", "2000", "kg", "SCOPE3", "12", "ActivityBased", "mass", "HU", "EU"),
        // CAT 13
        ("S3-C13-01", "Assets leased to customers", "50000", "kWh", "SCOPE3", "13", "ActivityBased", "energy", "EN", "GLOBAL"),
        ("S3-C13-02", "Vermietete Anlagen", "30000", "kWh", "SCOPE3", "13", "ActivityBased", "energy", "DE", "EU"),
        ("S3-C13-03", "Bérbe adott eszközök", "10000", "kWh", "SCOPE3", "13", "ActivityBased", "energy", "HU", "EU"),
        // CAT 14
        ("S3-C14-01", "Franchise operations", "60000", "USD", "SCOPE3", "14", "SpendBased", "currency", "EN", "US"),
        ("S3-C14-02", "Franchise fee", "25000", "EUR", "SCOPE3", "14", "SpendBased", "currency", "DE", "EU"),
        ("S3-C14-03", "Franchise hálózat", "10000000", "HUF", "SCOPE3", "14", "SpendBased", "currency", "HU", "EU"),
        // CAT 15
        ("S3-C15-01", "Listed equity", "5000000", "USD", "SCOPE3", "15", "Pcaf", "currency", "EN", "US"),
        ("S3-C15-02", "Corporate bonds", "2000000", "EUR", "SCOPE3", "15", "Pcaf", "currency", "DE", "EU"),
        ("S3-C15-03", "Finanszírozott kibocsátás", "100000000", "HUF", "SCOPE3", "15", "Pcaf", "currency", "HU", "EU"),

        // PROBLEM CASES
        ("P-01", "Natural Gas", "1,234.56", "kWh", "SCOPE1", "", "ActivityBased", "energy", "EN", "US"),
        ("P-02", "Erdgas", "1.234,56", "kWh", "SCOPE1", "", "ActivityBased", "energy", "DE", "EU"),
        ("P-03", "Electricity", "~50000", "kWh", "SCOPE2_LB", "", "ActivityBased", "energy", "EN", "US"),
        ("P-04", "Diesel", "48k", "liters", "SCOPE1", "", "ActivityBased", "volume", "EN", "EU"),
        ("P-05", "Spend", "2.5M", "USD", "SCOPE3", "1", "SpendBased", "currency", "EN", "US"),
        ("P-06", "Fuel", "1 234,56 €", "", "SCOPE1", "", "ActivityBased", "volume", "DE", "EU"),
        ("M-01", "Company ID", "CO-12345", "", "QUARANTINE", "", "", "", "", ""),
        ("M-02", "Year", "2023", "", "QUARANTINE", "", "", "", "", ""),
        ("E-01", "Negative Value", "-100", "kg", "QUARANTINE", "", "", "", "", ""),
        ("E-02", "Huge Value", "1000000000", "t", "QUARANTINE", "", "", "", "", ""),
    ];

    for (row_idx, (tid, header, val, unit, scope, cid, path, ucat, lang, jur)) in test_cases.iter().enumerate() {
        let r = (row_idx + 1) as u32;
        worksheet.write(r, 0, *tid)?;
        worksheet.write(r, 1, *header)?;
        worksheet.write(r, 2, *val)?;
        worksheet.write(r, 3, *unit)?;
        worksheet.write(r, 4, *scope)?;
        worksheet.write(r, 5, *cid)?;
        worksheet.write(r, 6, *path)?;
        worksheet.write(r, 7, *ucat)?;
        worksheet.write(r, 8, *lang)?;
        worksheet.write(r, 9, *jur)?;
    }

    workbook.save("stress_test_master.xlsx")?;
    Ok(())
}
