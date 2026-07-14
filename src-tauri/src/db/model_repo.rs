use rusqlite::{params, Connection};

use crate::llm::types::DownloadedModel;
use crate::utils::error::{AppError, AppResult};

pub fn list_downloaded(conn: &Connection) -> AppResult<Vec<DownloadedModel>> {
    let mut stmt = conn.prepare(
        "SELECT id, model_name, model_path, file_size, checksum_sha256, quantization, is_active
         FROM model_downloads ORDER BY downloaded_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(DownloadedModel {
            id: row.get(0)?,
            model_name: row.get(1)?,
            model_path: row.get(2)?,
            file_size: row.get(3)?,
            checksum_sha256: row.get(4)?,
            quantization: row.get(5)?,
            is_active: row.get::<_, i64>(6)? != 0,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn upsert_download(conn: &Connection, model: &DownloadedModel) -> AppResult<()> {
    conn.execute(
        "INSERT INTO model_downloads (id, model_name, model_path, file_size, checksum_sha256, quantization, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
           model_name = excluded.model_name,
           model_path = excluded.model_path,
           file_size = excluded.file_size,
           checksum_sha256 = excluded.checksum_sha256,
           quantization = excluded.quantization,
           is_active = excluded.is_active,
           downloaded_at = datetime('now')",
        params![
            model.id,
            model.model_name,
            model.model_path,
            model.file_size,
            model.checksum_sha256,
            model.quantization,
            if model.is_active { 1 } else { 0 },
        ],
    )?;
    Ok(())
}

pub fn set_active(conn: &Connection, model_id: &str) -> AppResult<()> {
    conn.execute("UPDATE model_downloads SET is_active = 0", [])?;
    let updated = conn.execute(
        "UPDATE model_downloads SET is_active = 1 WHERE id = ?1",
        params![model_id],
    )?;
    if updated == 0 {
        return Err(AppError::Other(format!("Model not found: {model_id}")));
    }
    Ok(())
}

pub fn active_model(conn: &Connection) -> AppResult<Option<DownloadedModel>> {
    let mut stmt = conn.prepare(
        "SELECT id, model_name, model_path, file_size, checksum_sha256, quantization, is_active
         FROM model_downloads WHERE is_active = 1 LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(DownloadedModel {
            id: row.get(0)?,
            model_name: row.get(1)?,
            model_path: row.get(2)?,
            file_size: row.get(3)?,
            checksum_sha256: row.get(4)?,
            quantization: row.get(5)?,
            is_active: true,
        }));
    }

    Ok(None)
}

pub fn delete_model(conn: &Connection, model_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM model_downloads WHERE id = ?1",
        params![model_id],
    )?;
    Ok(())
}
