mod chat;
mod documents;
mod llm;
mod research;
mod security;
mod settings;

use std::sync::Arc;

use tauri::State;

use crate::db::Database;

pub use chat::*;
pub use documents::*;
pub use llm::*;
pub use research::*;
pub use security::*;
pub use settings::*;

#[tauri::command]
pub fn get_db_status(database: State<'_, Arc<Database>>) -> Result<crate::db::DbStatus, String> {
    database.status().map_err(|err| err.to_string())
}
