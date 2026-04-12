use crate::ingest::RawRow;
use crate::ai_client::AiBridgeClient;
use std::sync::Arc;

pub async fn infer_activity_from_row(
    row: &RawRow,
    ai_client: &Arc<AiBridgeClient>
) -> Option<(String, f32)> {
    // a) Iterate through all non-numeric values in the row
    // other_columns already contains these filtered values
    
    let mut best_activity: Option<(String, f32)> = None;

    for val in &row.other_columns {
        let trimmed = val.trim();
        if trimmed.is_empty() || trimmed.len() < 3 {
            continue;
        }

        // b) Call AI Bridge for each value
        if let Ok(resp) = ai_client.classify(trimmed).await {
            // c) If a value strongly correlates with an ESG activity
            if resp.matched && resp.confidence > 0.6 {
                if best_activity.is_none() || resp.confidence > best_activity.as_ref().unwrap().1 {
                    best_activity = Some((trimmed.to_string(), resp.confidence));
                }
            }
        }
        
        // Short-circuit if we found a very strong match
        if let Some((_, conf)) = &best_activity {
            if *conf > 0.9 {
                break;
            }
        }
    }

    best_activity
}
