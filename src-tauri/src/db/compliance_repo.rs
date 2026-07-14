use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection};
use crate::utils::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub checkpoints: String, // Stringified JSON array of rules
    pub created_at: String,
    pub updated_at: String,
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Option<ComplianceTemplate>> {
    let mut stmt = conn.prepare("SELECT id, name, description, checkpoints, created_at, updated_at FROM compliance_templates WHERE id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(ComplianceTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            checkpoints: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }));
    }
    Ok(None)
}

pub fn create(conn: &Connection, template: &ComplianceTemplate) -> AppResult<()> {
    conn.execute(
        "INSERT INTO compliance_templates (id, name, description, checkpoints) VALUES (?1, ?2, ?3, ?4)",
        params![template.id, template.name, template.description, template.checkpoints],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> AppResult<Vec<ComplianceTemplate>> {
    let mut stmt = conn.prepare("SELECT id, name, description, checkpoints, created_at, updated_at FROM compliance_templates ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok(ComplianceTemplate {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            checkpoints: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    
    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }
    Ok(list)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM compliance_templates WHERE id = ?1", params![id])?;
    Ok(())
}
