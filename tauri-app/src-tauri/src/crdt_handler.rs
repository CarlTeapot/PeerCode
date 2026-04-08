use tauri::State;

use crate::appstate::AppState;

#[tauri::command]
pub fn insert(state: State<AppState>, position: u64, content: String) -> Result<(), String> {
    let mut document = state
        .document
        .lock()
        .map_err(|_| "failed to lock document state".to_string())?;

    document
        .local_insert(position, &content)
        .map_err(|err| format!("insert failed: {err:?}"))
}

#[tauri::command]
pub fn delete(state: State<AppState>, position: u64, length: u64) -> Result<(), String> {
    let mut document = state
        .document
        .lock()
        .map_err(|_| "failed to lock document state".to_string())?;

    document
        .delete(position, length)
        .map_err(|err| format!("delete failed: {err:?}"))
}
