use crate::db::audit_repo;
use crate::utils::error::AppResult;
use rusqlite::Connection;

/// Log an outgoing cloud API call.
pub fn log_cloud_call(
    conn: &Connection,
    session_id: Option<&str>,
    destination: &str,
    raw_query: &str,
    sanitized_query: &str,
    response: &str,
    risk: &str,
) -> AppResult<()> {
    audit_repo::log_event(
        conn,
        "cloud_call",
        session_id,
        &format!("Sent research query for topic to {}", destination),
        Some(raw_query),
        Some(destination),
        Some(response),
        Some(sanitized_query),
        Some(risk),
    )
}

/// Log a request blocked due to PII violations or spending limits.
pub fn log_blocked_call(
    conn: &Connection,
    session_id: Option<&str>,
    destination: &str,
    raw_query: &str,
    details: &str,
) -> AppResult<()> {
    audit_repo::log_event(
        conn,
        "blocked",
        session_id,
        details,
        Some(raw_query),
        Some(destination),
        None,
        None,
        Some("high"),
    )
}

/// Log PII redaction event details.
pub fn log_pii_redacted(
    conn: &Connection,
    session_id: Option<&str>,
    raw_text: &str,
    sanitized_text: &str,
    risk: &str,
) -> AppResult<()> {
    audit_repo::log_event(
        conn,
        "pii_detected",
        session_id,
        "PII details redacted from user query before outbound sending.",
        Some(raw_text),
        None,
        None,
        Some(sanitized_text),
        Some(risk),
    )
}
