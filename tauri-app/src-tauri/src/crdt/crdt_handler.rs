use tauri::State;

use crate::state::appstate::AppState;
use std::sync::atomic::Ordering;
#[tauri::command]
pub fn insert(state: State<AppState>, position: u64, content: String) -> Result<(), String> {
    let mut document = state
        .document
        .lock()
        .map_err(|_| "failed to lock document state".to_string())?;

    // TODO(T10): forward the returned `Option<WireBlock>` to the ws writer
    // as an encoded `OpMessage::Insert` frame.
    document
        .local_insert(position, &content)
        .map(|_wire_block| ())
        .map_err(|err| format!("insert failed: {err:?}"))
}

#[tauri::command]
pub fn delete(state: State<AppState>, position: u64, length: u64) -> Result<(), String> {
    let mut document = state
        .document
        .lock()
        .map_err(|_| "failed to lock document state".to_string())?;

    // TODO(T10): forward the returned `DeleteSet` diff to the ws writer
    // as an encoded `OpMessage::Delete` frame.
    document
        .delete(position, length)
        .map(|_delete_set_diff| ())
        .map_err(|err| format!("delete failed: {err:?}"))
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn toggle_crdt_logging(state: tauri::State<AppState>) {
    let current = state.crdt_logging_enabled.load(Ordering::Relaxed);
    state
        .crdt_logging_enabled
        .store(!current, Ordering::Relaxed);
}
