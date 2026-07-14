mod chat;
mod llm;
mod settings;
mod documents;
mod research;
mod security;

use std::sync::Arc;

use tauri::State;

use crate::db::Database;

pub use chat::*;
pub use llm::*;
pub use settings::*;
pub use documents::*;
pub use research::*;
pub use security::*;


#[tauri::command]
pub fn get_db_status(database: State<'_, Arc<Database>>) -> Result<crate::db::DbStatus, String> {
    database.status().map_err(|err| err.to_string())
}
