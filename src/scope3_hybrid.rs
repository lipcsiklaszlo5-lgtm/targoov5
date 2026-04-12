use crate::models::{CalcPath, Scope3Category};
use crate::physics::UnitConverter;

pub struct HybridRouter {
    unit_converter: UnitConverter,
}

impl HybridRouter {
    pub fn new() -> Self {
        Self {
            unit_converter: UnitConverter::new(),
        }
    }

    /// Determines the optimal calculation path for a given Scope 3 row
    /// Returns (CalcPath, confidence, reason)
    pub fn determine_calc_path(
        &self,
        category: Scope3Category,
        header: &str,
        unit: &str,
        value: f64,
    ) -> (CalcPath, f32, String) {
        let header_lower = header.to_lowercase();
        let unit_lower = unit.to_lowercase();

        // 1. Check for explicit calculation path indicators in header
        if let Some((path, conf)) = self.check_header_indicators(category, &header_lower) {
            return (path, conf, "Header indicates preferred calculation path".to_string());
        }

        // 2. Analyze the unit type
        let unit_category = self.unit_converter.detect_category(&unit_lower);

        // 3. Category-specific routing logic
        match category {
            Scope3Category::Cat1PurchasedGoodsServices
            | Scope3Category::Cat2CapitalGoods
            | Scope3Category::Cat8UpstreamLeasedAssets
            | Scope3Category::Cat10ProcessingSoldProducts
            | Scope3Category::Cat14Franchises => {
                // These categories prefer SpendBased, but can use ActivityBased if physical unit present
                if unit_category == "currency" {
                    (CalcPath::SpendBased, 0.9, "Currency unit detected".to_string())
                } else if unit_category != "unknown" {
                    (CalcPath::ActivityBased, 0.7, "Physical unit available for spend-based category".to_string())
                } else {
                    (CalcPath::SpendBased, 0.5, "Defaulting to SpendBased for ambiguous unit".to_string())
                }
            }

            Scope3Category::Cat3FuelEnergyActivities
            | Scope3Category::Cat4UpstreamTransport
            | Scope3Category::Cat5WasteGenerated
            | Scope3Category::Cat6BusinessTravel
            | Scope3Category::Cat7EmployeeCommuting
            | Scope3Category::Cat9DownstreamTransport
            | Scope3Category::Cat11UseOfSoldProducts
            | Scope3Category::Cat12EndOfLifeTreatment
            | Scope3Category::Cat13DownstreamLeasedAssets => {
                // These categories strongly prefer ActivityBased
                if unit_category == "currency" {
                    // Check if it's a small value that might actually be spend data
                    if value < 1000.0 && self.looks_like_spend_data(&header_lower) {
                        (CalcPath::SpendBased, 0.6, "Small value with currency unit, treating as SpendBased".to_string())
                    } else {
                        (CalcPath::ActivityBased, 0.5, "Currency unit but category expects physical activity".to_string())
                    }
                } else if unit_category != "unknown" {
                    (CalcPath::ActivityBased, 0.95, "Physical unit matches category expectation".to_string())
                } else {
                    // Fallback for unknown units
                    if self.looks_like_spend_data(&header_lower) {
                        (CalcPath::SpendBased, 0.4, "Header suggests spend data".to_string())
                    } else {
                        (CalcPath::ActivityBased, 0.3, "Defaulting to ActivityBased for unknown unit".to_string())
                    }
                }
            }

            Scope3Category::Cat15Investments => {
                // PCAF is mandatory for Cat 15
                (CalcPath::Pcaf, 1.0, "PCAF mandatory for Category 15".to_string())
            }
        }
    }

    fn check_header_indicators(&self, category: Scope3Category, header: &str) -> Option<(CalcPath, f32)> {
        // Activity-based indicators
        let activity_keywords = [
            "kwh", "mwh", "gj", "tonne", "kg", "km", "mile", "liter", "gallon",
            "physical", "quantity", "consumption", "verbrauch", "menge",
        ];
        
        // Spend-based indicators
        let spend_keywords = [
            "usd", "eur", "gbp", "$", "€", "£", "cost", "spend", "price", "amount",
            "betrag", "kosten", "preis", "expense", "expenditure",
        ];

        // Check for strong activity indicators
        for kw in activity_keywords {
            if header.contains(kw) {
                // For categories that can use ActivityBased
                if !matches!(category, Scope3Category::Cat15Investments) {
                    return Some((CalcPath::ActivityBased, 0.95));
                }
            }
        }

        // Check for strong spend indicators
        for kw in spend_keywords {
            if header.contains(kw) {
                // For categories that allow SpendBased
                if matches!(
                    category,
                    Scope3Category::Cat1PurchasedGoodsServices
                        | Scope3Category::Cat2CapitalGoods
                        | Scope3Category::Cat8UpstreamLeasedAssets
                        | Scope3Category::Cat10ProcessingSoldProducts
                        | Scope3Category::Cat14Franchises
                ) {
                    return Some((CalcPath::SpendBased, 0.9));
                }
            }
        }

        None
    }

    fn looks_like_spend_data(&self, header: &str) -> bool {
        let spend_indicators = [
            "cost", "spend", "price", "expense", "amount", "invoice", "payment",
            "rechnung", "betrag", "kosten", "preis", "ausgabe",
            "költség", "ár", "számla", "összeg",
        ];
        spend_indicators.iter().any(|kw| header.contains(kw))
    }

    /// For Category 4 and 9 (Transport), suggests vehicle type based on header
    pub fn infer_transport_mode(&self, header: &str) -> Option<TransportMode> {
        let header_lower = header.to_lowercase();
        
        if header_lower.contains("air") || header_lower.contains("flight") || header_lower.contains("flug") {
            Some(TransportMode::AirFreight)
        } else if header_lower.contains("sea") || header_lower.contains("ocean") || header_lower.contains("schiff") {
            Some(TransportMode::SeaFreight)
        } else if header_lower.contains("rail") || header_lower.contains("train") || header_lower.contains("bahn") {
            Some(TransportMode::RailFreight)
        } else if header_lower.contains("hgv") || header_lower.contains("truck") || header_lower.contains("lkw") {
            // Need to infer weight class
            if header_lower.contains("32") || header_lower.contains("heavy") || header_lower.contains("schwer") {
                Some(TransportMode::RoadHGVHeavy)
            } else if header_lower.contains("16") || header_lower.contains("medium") {
                Some(TransportMode::RoadHGVMedium)
            } else {
                Some(TransportMode::RoadHGVLight)
            }
        } else if header_lower.contains("van") || header_lower.contains("transporter") {
            Some(TransportMode::RoadHGVLight)
        } else if header_lower.contains("inland water") || header_lower.contains("barge") {
            Some(TransportMode::InlandWaterway)
        } else {
            None
        }
    }

    /// For Category 6 (Business Travel), infers travel class
    pub fn infer_travel_class(&self, header: &str) -> TravelClass {
        let header_lower = header.to_lowercase();
        
        if header_lower.contains("first") || header_lower.contains("erste") {
            TravelClass::First
        } else if header_lower.contains("business") || header_lower.contains("geschäft") {
            TravelClass::Business
        } else {
            TravelClass::Economy
        }
    }

    /// For Category 5 (Waste), infers disposal method
    pub fn infer_waste_method(&self, header: &str) -> Option<WasteMethod> {
        let header_lower = header.to_lowercase();
        
        if header_lower.contains("landfill") || header_lower.contains("deponie") || header_lower.contains("lerak") {
            if header_lower.contains("organic") || header_lower.contains("bio") {
                Some(WasteMethod::LandfillOrganic)
            } else {
                Some(WasteMethod::LandfillMixed)
            }
        } else if header_lower.contains("incineration") || header_lower.contains("verbrennung") {
            if header_lower.contains("recovery") || header_lower.contains("energie") {
                Some(WasteMethod::IncinerationWithRecovery)
            } else {
                Some(WasteMethod::Incineration)
            }
        } else if header_lower.contains("recycling") || header_lower.contains("újrahaszn") {
            Some(WasteMethod::Recycling)
        } else if header_lower.contains("compost") || header_lower.contains("kompost") {
            Some(WasteMethod::Composting)
        } else if header_lower.contains("anaerobic") || header_lower.contains("biogas") {
            Some(WasteMethod::AnaerobicDigestion)
        } else if header_lower.contains("wastewater") || header_lower.contains("abwasser") || header_lower.contains("szennyvíz") {
            Some(WasteMethod::Wastewater)
        } else {
            None
        }
    }
}

impl Default for HybridRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TransportMode {
    RoadHGVHeavy,   // >32t
    RoadHGVMedium,  // 16-32t
    RoadHGVLight,   // <16t
    RailFreight,
    SeaFreight,
    AirFreight,
    InlandWaterway,
}

#[derive(Debug, Clone, Copy)]
pub enum TravelClass {
    Economy,
    Business,
    First,
}

#[derive(Debug, Clone, Copy)]
pub enum WasteMethod {
    LandfillMixed,
    LandfillOrganic,
    Incineration,
    IncinerationWithRecovery,
    Recycling,
    Composting,
    AnaerobicDigestion,
    Wastewater,
}
