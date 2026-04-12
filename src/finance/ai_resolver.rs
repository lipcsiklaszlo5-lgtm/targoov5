use crate::ai_client::AiBridgeClient;
use super::pcaf_attribution::AssetClass;

pub struct AiAssetResolver {
    client: AiBridgeClient,
}

impl AiAssetResolver {
    pub fn new() -> Self {
        Self {
            client: AiBridgeClient::new(),
        }
    }

    pub async fn detect_asset_class(&self, description: &str) -> Option<AssetClass> {
        // In a real implementation, this would use a specific prompt or a specialized endpoint.
        // For now, we reuse the classify endpoint which returns ghg_category etc.
        // The bridge would need to be updated to support AssetClass mapping.
        
        let response = self.client.classify(description).await.ok()?;
        
        if !response.matched {
            return None;
        }

        // Mapping logic (simplified placeholder)
        match response.scope3_id {
            Some(15) => {
                let desc_lower = description.to_lowercase();
                if desc_lower.contains("equity") || desc_lower.contains("stock") {
                    Some(AssetClass::ListedEquity)
                } else if desc_lower.contains("loan") || desc_lower.contains("credit") {
                    Some(AssetClass::BusinessLoans)
                } else if desc_lower.contains("mortgage") {
                    Some(AssetClass::Mortgages)
                } else if desc_lower.contains("project") {
                    Some(AssetClass::ProjectFinance)
                } else {
                    Some(AssetClass::BusinessLoans)
                }
            },
            _ => None,
        }
    }
    
    pub async fn extract_amount(&self, _text: &str) -> Option<f64> {
        // Placeholder for regex or AI extraction
        None
    }
    
    pub async fn extract_currency(&self, text: &str) -> Option<String> {
        let text_lower = text.to_lowercase();
        if text_lower.contains("usd") || text_lower.contains("$") {
            Some("USD".to_string())
        } else if text_lower.contains("eur") || text_lower.contains("€") {
            Some("EUR".to_string())
        } else if text_lower.contains("huf") || text_lower.contains("ft") {
            Some("HUF".to_string())
        } else {
            None
        }
    }
}

impl Default for AiAssetResolver {
    fn default() -> Self {
        Self::new()
    }
}
