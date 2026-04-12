use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LsrCategory {
    LandUseChange,        // LUC (e.g., deforestation)
    LandManagement,       // Soil carbon, fertilization, etc.
    Leakage,              // Indirect land use change / Carbon opportunity cost
    BiogenicProduct,      // Emissions from biogenic products
    CarbonRemoval,        // Removals (sequestration)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LandOwnership {
    Owned,      // Scope 1
    Supplier,   // Scope 3
}
