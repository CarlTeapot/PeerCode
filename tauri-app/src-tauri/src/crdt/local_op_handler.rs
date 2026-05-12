use crdt_core::wire::{encode_op, OpMessage};
use log::{debug, error, info};
use tauri::State;

use crate::state::appstate::AppState;
use crate::state::ws_state::WsState;
use std::sync::atomic::Ordering;

const SNAPSHOT_REFRESH_INTERVAL: u32 = 100;

#[tauri::command]
pub async fn insert(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    content: String,
) -> Result<(), String> {
    debug!(
        "crdt insert request: position={}, content_len={}",
        position,
        content.chars().count()
    );

    let wire_block_opt = {
        let mut document = state.document.lock().map_err(|_| {
            error!("crdt insert failed: could not lock document state");
            "failed to lock document state".to_string()
        })?;
        document.local_insert(position, &content).map_err(|err| {
            error!(
                "crdt insert failed: position={}, content_len={}, error={:?}",
                position,
                content.chars().count(),
                err
            );
            format!("insert failed: {err:?}")
        })?
    };

    debug!(
        "crdt insert succeeded: position={}, content_len={}",
        position,
        content.chars().count()
    );

    if let Some(wire_block) = wire_block_opt {
        let frame = encode_op(&OpMessage::Insert(wire_block));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    length: u64,
) -> Result<(), String> {
    debug!(
        "crdt delete request: position={}, length={}",
        position, length
    );

    let delete_set = {
        let mut document = state.document.lock().map_err(|_| {
            error!("crdt delete failed: could not lock document state");
            "failed to lock document state".to_string()
        })?;
        document.delete(position, length).map_err(|err| {
            error!(
                "crdt delete failed: position={}, length={}, error={:?}",
                position, length, err
            );
            format!("delete failed: {err:?}")
        })?
    };

    debug!(
        "crdt delete succeeded: position={}, length={}",
        position, length
    );

    if !delete_set.is_empty() {
        let frame = encode_op(&OpMessage::Delete(delete_set));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }

    Ok(())
}

async fn maybe_send_snapshot(state: &State<'_, AppState>, ws: &State<'_, WsState>) {
    if !matches!(
        *state.role.lock().unwrap(),
        crate::state::appstate::AppRole::Host { .. }
    ) {
        return;
    }
    let count = state.ops_since_snapshot.fetch_add(1, Ordering::Relaxed) + 1;
    if count >= SNAPSHOT_REFRESH_INTERVAL {
        state.ops_since_snapshot.store(0, Ordering::Relaxed);
        let snapshot_frame = {
            let doc = state.document.lock().unwrap();
            crdt_core::encode_snapshot(&doc.to_snapshot())
        };
        ws.send_raw(snapshot_frame).await;
        info!("periodic snapshot sent (after {} ops)", count);
    }
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn toggle_crdt_logging(state: tauri::State<AppState>) {
    let current = state.crdt_logging_enabled.load(Ordering::Relaxed);
    debug!("toggle_crdt_logging request: current={}", current);
    state
        .crdt_logging_enabled
        .store(!current, Ordering::Relaxed);
    debug!("toggle_crdt_logging succeeded: enabled={}", !current);
}
