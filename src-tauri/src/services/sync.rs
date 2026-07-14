use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;
use crate::db::Database;
use crate::db::settings_repo;
use crate::db::session_repo;

pub struct SyncService {
    database: Arc<Database>,
}

impl SyncService {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Start a background task for database replication sync sweeps.
    pub fn start_background_sync(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = self.perform_sync_sweep().await {
                    tracing::error!("Background sync sweep failed: {}", e);
                }
            }
        });
    }

    pub async fn perform_sync_sweep(&self) -> Result<(), String> {
        let (enabled, url, token) = self.database.with_connection(|conn| {
            let enabled = settings_repo::get(conn, "team_sync_enabled")
                .unwrap_or_default()
                .unwrap_or_else(|| "false".to_string()) == "true";
            let url = settings_repo::get(conn, "team_sync_url")
                .unwrap_or_default()
                .unwrap_or_else(|| "".to_string());
            let token = settings_repo::get(conn, "sso_token")
                .unwrap_or_default()
                .unwrap_or_else(|| "".to_string());
            Ok((enabled, url, token))
        }).map_err(|e: rusqlite::Error| e.to_string())?;

        if !enabled || url.is_empty() {
            return Ok(());
        }

        tracing::info!("Performing background sync sweep to coordinator URL: {}", url);

        // 1. Gather local changes (e.g. sessions)
        let local_data = self.database.with_connection(|conn| {
            let sessions = session_repo::list(conn).unwrap_or_default();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            Ok::<serde_json::Value, rusqlite::Error>(serde_json::json!({
                "sessions": sessions,
                "timestamp": timestamp
            }))
        }).map_err(|e: rusqlite::Error| e.to_string())?;

        // 2. Perform outbound HTTP POST request using reqwest Client
        let client = reqwest::Client::new();
        let mut request = client.post(format!("{}/push", url.trim_end_matches('/')))
            .json(&local_data);

        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::info!("Successfully pushed sync replication payload containing {} sessions.", 
                                 local_data["sessions"].as_array().map(|a| a.len()).unwrap_or(0));
                } else {
                    tracing::warn!("Sync push returned non-success status code: {}", response.status());
                }
            }
            Err(err) => {
                tracing::error!("Failed to perform outbound sync request: {}", err);
            }
        }

        Ok(())
}
