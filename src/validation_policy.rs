use crate::models::{GhgScope, QuarantineReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    HardFail,
    SoftFailWarn,
    Quarantine,
}

/// Determines the severity level for a given validation issue.
pub fn get_validation_severity(issue: ValidationIssue) -> ValidationSeverity {
    match issue {
        ValidationIssue::NegativeValue => ValidationSeverity::Quarantine,
        ValidationIssue::FutureDate => ValidationSeverity::SoftFailWarn,
        ValidationIssue::UnknownUnit => ValidationSeverity::HardFail,
        ValidationIssue::NonNumericValue => ValidationSeverity::Quarantine,
        ValidationIssue::OutOfRangeValue => ValidationSeverity::Quarantine,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationIssue {
    NegativeValue,
    FutureDate,
    UnknownUnit,
    NonNumericValue,
    OutOfRangeValue,
}

/// Validates a value for common issues and returns the corresponding QuarantineReason if needed.
pub fn validate_value(value: f64, value_str: &str) -> Option<(QuarantineReason, String)> {
    if value < 0.0 {
        return Some((QuarantineReason::RangeGuardFail, format!("Negative value: {}", value_str)));
    }
    if !value.is_finite() {
        return Some((QuarantineReason::NonNumericValue, format!("Non-finite value: {}", value_str)));
    }
    None
}
