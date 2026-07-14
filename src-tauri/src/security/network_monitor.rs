use regex::Regex;
use lazy_static::lazy_static;
use crate::utils::error::{AppError, AppResult};

lazy_static! {
    static ref RAW_EMAIL_RE: Regex = Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap();
    static ref RAW_SSN_RE: Regex = Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    static ref RAW_CC_RE: Regex = Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap();
    static ref RAW_PHONE_RE: Regex = Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
}

/// Analyze outbound payloads before sending them to external cloud APIs.
/// 
/// If debugging assertions are enabled, any unredacted PII will trigger an error
/// to prevent accidental leaks during development.
pub fn validate_outbound_payload(payload: &str) -> AppResult<()> {
    let mut violations = Vec::new();

    if RAW_EMAIL_RE.is_match(payload) {
        violations.push("Raw Email Address");
    }
    if RAW_SSN_RE.is_match(payload) {
        violations.push("Raw Social Security Number (SSN)");
    }
    if RAW_CC_RE.is_match(payload) {
        violations.push("Raw Credit Card Number");
    }
    if RAW_PHONE_RE.is_match(payload) {
        violations.push("Raw Phone Number");
    }

    if !violations.is_empty() {
        let error_msg = format!(
            "CRITICAL PRIVACY VIOLATION: Unsanitized PII detected in outbound payload: {:?}. Request blocked.",
            violations
        );
        
        // In dev mode, use error-level logging for visibility
        #[cfg(debug_assertions)]
        {
            tracing::error!("##################################################");
            tracing::error!("{}", error_msg);
            tracing::error!("##################################################");
        }
        
        // In release mode, still log but at warn level
        #[cfg(not(debug_assertions))]
        {
            tracing::warn!("{}", error_msg);
        }

        // Always block the request — PII must never leak regardless of build profile
        return Err(AppError::Encryption(format!("Privacy Protection: {}", error_msg)));
    }

    Ok(())
}
