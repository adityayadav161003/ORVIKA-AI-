use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::utils::error::{AppError, AppResult};

/// Structure representing a record in the audit log table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub event_type: String,
    pub session_id: Option<String>,
    pub details: String,
    pub outbound_content: Option<String>,
    pub destination: Option<String>,
    pub response_summary: Option<String>,
    pub sanitization_result: Option<String>,
    pub risk_level: Option<String>,
}

/// Log an event into the audit log.
pub fn log_event(
    conn: &Connection,
    event_type: &str,
    session_id: Option<&str>,
    details: &str,
    outbound_content: Option<&str>,
    destination: Option<&str>,
    response_summary: Option<&str>,
    sanitization_result: Option<&str>,
    risk_level: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO audit_log (
            event_type, session_id, details, outbound_content, 
            destination, response_summary, sanitization_result, risk_level
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event_type,
            session_id,
            details,
            outbound_content,
            destination,
            response_summary,
            sanitization_result,
            risk_level
        ],
    )?;
    Ok(())
}

/// Query and filter audit logs based on parameters.
pub fn list_events(
    conn: &Connection,
    session_id: Option<&str>,
    event_type: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> AppResult<Vec<AuditLogEntry>> {
    let mut query = "SELECT id, timestamp, event_type, session_id, details, 
                     outbound_content, destination, response_summary, 
                     sanitization_result, risk_level 
                     FROM audit_log WHERE 1=1".to_string();
    let mut params_vec: Vec<String> = Vec::new();

    if let Some(sid) = session_id {
        query.push_str(" AND session_id = ?");
        params_vec.push(sid.to_string());
    }

    if let Some(etype) = event_type {
        if !etype.is_empty() {
            query.push_str(" AND event_type = ?");
            params_vec.push(etype.to_string());
        }
    }

    if let Some(start) = start_date {
        if !start.is_empty() {
            query.push_str(" AND timestamp >= ?");
            params_vec.push(start.to_string());
        }
    }

    if let Some(end) = end_date {
        if !end.is_empty() {
            query.push_str(" AND timestamp <= ?");
            params_vec.push(end.to_string());
        }
    }

    query.push_str(" ORDER BY timestamp DESC LIMIT 500");

    let mut stmt = conn.prepare(&query)?;
    
    // We convert parameters to dynamic array for query_map
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();

    let rows = stmt.query_map(&params_refs[..], |row| {
        Ok(AuditLogEntry {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            event_type: row.get(2)?,
            session_id: row.get(3)?,
            details: row.get(4)?,
            outbound_content: row.get(5)?,
            destination: row.get(6)?,
            response_summary: row.get(7)?,
            sanitization_result: row.get(8)?,
            risk_level: row.get(9)?,
        })
    })?;

    let mut entries = Vec::new();
    for entry in rows {
        entries.push(entry?);
    }
    Ok(entries)
}

/// Wipe all logs from the audit table.
pub fn clear_events(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM audit_log", [])?;
    Ok(())
}

/// Get privacy dashboard analytics statistics.
pub fn get_stats(conn: &Connection) -> AppResult<serde_json::Value> {
    let total_requests: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE event_type = 'cloud_call'",
        [],
        |row| row.get(0),
    )?;

    let blocked_requests: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE event_type = 'blocked'",
        [],
        |row| row.get(0),
    )?;

    let pii_detected: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE event_type = 'pii_detected'",
        [],
        |row| row.get(0),
    )?;

    // Risk levels breakdown
    let risk_low: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE risk_level = 'low'",
        [],
        |row| row.get(0),
    )?;
    
    let risk_medium: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE risk_level = 'medium'",
        [],
        |row| row.get(0),
    )?;

    let risk_high: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_log WHERE risk_level = 'high'",
        [],
        |row| row.get(0),
    )?;

    // Calculate a safety score
    // Start at 100%, deduct 5% for medium risk event, 15% for high risk event, and 20% for blocked events.
    let mut health_score = 100 - (risk_medium * 5) - (risk_high * 15) - (blocked_requests * 20);
    if health_score < 0 {
        health_score = 0;
    }

    Ok(serde_json::json!({
        "totalRequests": total_requests,
        "blockedRequests": blocked_requests,
        "piiDetected": pii_detected,
        "healthScore": health_score,
        "riskBreakdown": {
            "low": risk_low,
            "medium": risk_medium,
            "high": risk_high
        }
    }))
}
