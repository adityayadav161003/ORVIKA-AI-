use std::path::{Path, PathBuf};

use crate::utils::error::{AppError, AppResult};

pub fn ensure_data_dir(base: &Path) -> AppResult<PathBuf> {
    let data_dir = base.join("data");
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

pub fn database_path(base: &Path) -> PathBuf {
    base.join("data").join("app.db")
}

pub fn resolve_app_data_dir(base: &Path) -> AppResult<PathBuf> {
    if !base.exists() {
        std::fs::create_dir_all(base).map_err(|err| {
            AppError::Config(format!("Failed to create app data directory: {err}"))
        })?;
    }
    Ok(base.to_path_buf())
}
