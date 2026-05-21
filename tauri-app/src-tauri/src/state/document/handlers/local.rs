use crdt_core::store::DeleteSet;
use crdt_core::wire::WireBlock;
use crdt_core::DocumentError;
use log::{debug, warn};
use tauri::{AppHandle, Emitter};

use crate::state::document::state::DocState;
use crate::state::document::types::REMOTE_CHANGE_EVENT;
use crate::ws_management::ws_types::RemoteChangeEvent;

pub fn handle_local_insert(
    state: &mut DocState,
    app: &AppHandle,
    position: u64,
    content: &str,
    base_seq: u64,
) -> Result<Option<WireBlock>, String> {
    let transformed = state.op_log.transform(position, base_seq);
    if transformed != position {
        debug!(
            "doc actor: local_insert transformed {} -> {} (base_seq={})",
            position, transformed, base_seq
        );
    }
    let (actual_pos, wire) = insert_returning_pos(state, transformed, content)?;
    if wire.is_some() {
        emit_local_insert(state, app, actual_pos, content);
    }
    Ok(wire)
}

pub fn handle_local_delete(
    state: &mut DocState,
    app: &AppHandle,
    position: u64,
    length: u64,
    base_seq: u64,
) -> Result<DeleteSet, String> {
    let transformed = state.op_log.transform(position, base_seq);
    if transformed != position {
        debug!(
            "doc actor: local_delete transformed {} -> {} (base_seq={})",
            position, transformed, base_seq
        );
    }
    let (actual_pos, actual_len, ds) = delete_returning_pos(state, transformed, length)?;
    if !ds.is_empty() {
        emit_local_delete(state, app, actual_pos, actual_len);
    }
    Ok(ds)
}

pub fn handle_local_replace(
    state: &mut DocState,
    app: &AppHandle,
    position: u64,
    delete_length: u64,
    content: &str,
    base_seq: u64,
) -> Result<(DeleteSet, Option<WireBlock>), String> {
    let transformed = state.op_log.transform(position, base_seq);
    if transformed != position {
        debug!(
            "doc actor: local_replace transformed {} -> {} (base_seq={})",
            position, transformed, base_seq
        );
    }
    let (del_pos, del_len, delete_set) = delete_returning_pos(state, transformed, delete_length)?;
    if !delete_set.is_empty() {
        emit_local_delete(state, app, del_pos, del_len);
    }
    let (ins_pos, wire) = insert_returning_pos(state, transformed, content)?;
    if wire.is_some() {
        emit_local_insert(state, app, ins_pos, content);
    }
    Ok((delete_set, wire))
}

fn emit_local_insert(state: &mut DocState, app: &AppHandle, position: u64, content: &str) {
    let seq = state.mint_seq();
    let event = RemoteChangeEvent::Insert {
        seq,
        position,
        content: content.to_string(),
    };
    if let Err(e) = app.emit(REMOTE_CHANGE_EVENT, &event) {
        warn!("doc actor: failed to emit local change event: {e}");
    }
}

fn emit_local_delete(state: &mut DocState, app: &AppHandle, position: u64, length: u64) {
    let seq = state.mint_seq();
    let event = RemoteChangeEvent::Delete {
        seq,
        position,
        length,
    };
    if let Err(e) = app.emit(REMOTE_CHANGE_EVENT, &event) {
        warn!("doc actor: failed to emit local change event: {e}");
    }
}

fn insert_returning_pos(
    state: &mut DocState,
    position: u64,
    content: &str,
) -> Result<(u64, Option<WireBlock>), String> {
    match state.doc.local_insert(position, content) {
        Ok(wire) => Ok((position, wire)),
        Err(DocumentError::OutOfBounds(_)) => clamped_retry_insert(state, position, content),
        Err(e) => Err(format!("{e:?}")),
    }
}

fn delete_returning_pos(
    state: &mut DocState,
    position: u64,
    length: u64,
) -> Result<(u64, u64, DeleteSet), String> {
    match state.doc.delete(position, length) {
        Ok(ds) => Ok((position, length, ds)),
        Err(DocumentError::OutOfBounds(_)) => clamped_retry_delete(state, position, length),
        Err(e) => Err(format!("{e:?}")),
    }
}

fn clamped_retry_insert(
    state: &mut DocState,
    position: u64,
    content: &str,
) -> Result<(u64, Option<WireBlock>), String> {
    let visible = state.visible_length();
    let clamped = position.min(visible);
    warn!(
        "doc actor: local_insert OOB at {} (visible_len={}); clamping to {}",
        position, visible, clamped
    );
    state
        .doc
        .local_insert(clamped, content)
        .map(|wire| (clamped, wire))
        .map_err(|e| format!("{e:?}"))
}

fn clamped_retry_delete(
    state: &mut DocState,
    position: u64,
    length: u64,
) -> Result<(u64, u64, DeleteSet), String> {
    let visible = state.visible_length();
    if visible == 0 {
        warn!(
            "doc actor: local_delete OOB with empty doc (pos={}, len={})",
            position, length
        );
        return Ok((0, 0, DeleteSet::new()));
    }
    let clamped_pos = position.min(visible.saturating_sub(1));
    let clamped_len = length.min(visible.saturating_sub(clamped_pos));
    warn!(
        "doc actor: local_delete OOB at {}+{} (visible_len={}); clamping to {}+{}",
        position, length, visible, clamped_pos, clamped_len
    );
    if clamped_len == 0 {
        return Ok((clamped_pos, 0, DeleteSet::new()));
    }
    state
        .doc
        .delete(clamped_pos, clamped_len)
        .map(|ds| (clamped_pos, clamped_len, ds))
        .map_err(|e| format!("{e:?}"))
}
