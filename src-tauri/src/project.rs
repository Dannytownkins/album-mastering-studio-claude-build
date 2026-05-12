use crate::types::*;
use std::path::{Path, PathBuf};

use tauri::Manager;

const SESSION_FILENAME: &str = "session.json";
const SESSION_TMP_FILENAME: &str = "session.json.tmp";
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[tauri::command]
pub async fn save_project(path: String, state: ProjectState) -> CommandResult<()> {
    if path.is_empty() {
        return Err(CommandError::InvalidPath("empty path".to_string()));
    }
    let p = Path::new(&path);
    if crate::files::has_parent_dir_component(p) {
        return Err(CommandError::InvalidPath(format!(
            "path traversal not allowed: {path}"
        )));
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| CommandError::Io(e.to_string()))?;
    }
    write_session_atomic(p, &state)
}

#[tauri::command]
pub async fn autosave_session(
    state: ProjectState,
    app: tauri::AppHandle,
) -> CommandResult<()> {
    let path = autosave_path(&app)?;
    write_session_atomic(&path, &state)
}

#[tauri::command]
pub async fn load_recent_session(
    app: tauri::AppHandle,
) -> CommandResult<Option<ProjectState>> {
    let path = autosave_path(&app)?;
    if !path.exists() {
        return Ok(None);
    }
    match read_session(&path) {
        Ok(state) if state.schema_version == SUPPORTED_SCHEMA_VERSION => Ok(Some(state)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

fn autosave_path(app: &tauri::AppHandle) -> CommandResult<PathBuf> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| CommandError::Other(format!("app_data_dir: {e}")))?;
    std::fs::create_dir_all(&app_data).map_err(|e| CommandError::Io(e.to_string()))?;
    Ok(app_data.join(SESSION_FILENAME))
}

pub fn write_session_atomic(path: &Path, state: &ProjectState) -> CommandResult<()> {
    let json = serde_json::to_vec_pretty(state)
        .map_err(|e| CommandError::Other(format!("serialize session: {e}")))?;
    let tmp_path = path
        .parent()
        .map(|p| p.join(SESSION_TMP_FILENAME))
        .unwrap_or_else(|| PathBuf::from(SESSION_TMP_FILENAME));
    std::fs::write(&tmp_path, &json).map_err(|e| CommandError::Io(e.to_string()))?;
    std::fs::rename(&tmp_path, path).map_err(|e| CommandError::Io(e.to_string()))?;
    Ok(())
}

pub fn read_session(path: &Path) -> CommandResult<ProjectState> {
    let json = std::fs::read(path).map_err(|e| CommandError::Io(e.to_string()))?;
    serde_json::from_slice(&json)
        .map_err(|e| CommandError::Other(format!("session parse: {e}")))
}
