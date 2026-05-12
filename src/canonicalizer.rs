/// Normalizes input text to prevent adversarial inputs and inconsistencies.
pub fn canonicalize_header(header: &str) -> String {
    let mut s = header.trim().to_lowercase();
    
    // Unicode homoglyph cleanup (basic)
    s = s.replace('’', "'").replace('‘', "'");
    s = s.replace('“', "\"").replace('”', "\"");
    s = s.replace('–', "-").replace('—', "-");
    
    // Decimal separator normalization
    s = s.replace(",", ".");
    
    // Unit canonicalization
    s = s.replace("kwh", "kwh").replace("kw·h", "kwh").replace("kw h", "kwh");
    s = s.replace("mwh", "mwh").replace("mw·h", "mwh").replace("mw h", "mwh");
    s = s.replace("gj", "gj");
    s = s.replace("tkm", "tkm").replace("tonne-km", "tkm");
    s = s.replace("kgco2e", "kgco2e");
    
    // Collapse multiple whitespaces
    let mut result = String::new();
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    
    result.trim().to_string()
}

/// Normalizes a numeric value string (handles European decimal notation)
/// Normalizes a numeric value string (handles European decimal notation)
pub fn canonicalize_value(value_str: &str) -> Option<f64> {
    let s = value_str.trim();
    if s.is_empty() {
        return None;
    }
    
    // Handle European format: 1.234,56 -> 1234.56
    // First, check if the string contains both '.' and ','
    let has_dot = s.contains('.');
    let has_comma = s.contains(',');
    
    if has_dot && has_comma {
        // Determine which is the thousands separator and which is the decimal
        let last_dot = s.rfind('.').unwrap_or(0);
        let last_comma = s.rfind(',').unwrap_or(0);
        
        if last_comma > last_dot {
            // European format: 1.234,56 -> dot is thousands, comma is decimal
            let cleaned: String = s.chars()
                .filter(|c| *c != '.')
                .collect();
            let cleaned = cleaned.replace(",", ".");
            return cleaned.parse::<f64>().ok();
        } else {
            // US format with both: 1,234.56 -> comma is thousands, dot is decimal
            let cleaned: String = s.chars()
                .filter(|c| *c != ',')
                .collect();
            return cleaned.parse::<f64>().ok();
        }
    } else if has_comma {
        // Only comma present: treat as decimal
        let cleaned = s.replace(",", ".");
        return cleaned.parse::<f64>().ok();
    } else {
        // Only dot or no separator
        return s.parse::<f64>().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_header_units() {
        assert_eq!(canonicalize_header("  KWH  "), "kwh");
        assert_eq!(canonicalize_header("kW·h"), "kwh");
        assert_eq!(canonicalize_header("MWh"), "mwh");
    }

    #[test]
    fn test_canonicalize_value() {
        assert_eq!(canonicalize_value("1.234,56"), Some(1234.56));
        assert_eq!(canonicalize_value("invalid"), None);
    }
}
