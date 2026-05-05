/// Alapvető emissziós képlet: tCO2e = (érték * faktor * GWP) / 1000
/// Minden bemenet f64 a pontosság érdekében.
pub fn calculate_tco2e(converted_value: f64, ef_kg_co2e: f64, gwp: f64) -> f64 {
    (converted_value * ef_kg_co2e * gwp) / 1000.0
}
