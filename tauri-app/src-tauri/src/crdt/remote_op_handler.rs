use crdt_core::store::DeleteSet;
use crdt_core::structs::Block;
use crdt_core::wire::WireBlock;
use crdt_core::{decode_op, Document, OpMessage, RemoteChange};
use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::appstate::AppState;
use crate::ws_management::ws_types::RemoteChangeEvent;

pub const REMOTE_CHANGE_EVENT: &str = "crdt://remote-change";

pub async fn process_loop(mut rx: UnboundedReceiver<Vec<u8>>, app: AppHandle) {
    info!("op processor loop started");
    while let Some(bytes) = rx.recv().await {
        handle_remote_binary(&app, &bytes);
    }
    info!("op processor loop stopped");
}

pub fn handle_remote_binary(app: &AppHandle, bytes: &[u8]) {
    let op = match decode_op(bytes) {
        Ok(op) => op,
        Err(e) => {
            warn!("ws recv: failed to decode op frame: {e}");
            return;
        }
    };

    let state = app.state::<AppState>();
    let mut document = match state.document.lock() {
        Ok(doc) => doc,
        Err(_) => {
            error!("ws recv: failed to lock document for remote op");
            return;
        }
    };

    let changes: Vec<RemoteChange> = match op {
        OpMessage::Insert(wire_block) => match handle_insert(&mut document, wire_block) {
            Some(changes) => changes,
            None => return,
        },
        OpMessage::Delete(delete_set) => match handle_delete(&mut document, delete_set) {
            Some(changes) => changes,
            None => return,
        },
    };

    drop(document);
    emit_remote_changes(app, changes);
}

fn handle_insert(document: &mut Document, wire_block: WireBlock) -> Option<Vec<RemoteChange>> {
    let block: Block = wire_block.into();
    match document.remote_insert(block) {
        Ok(changes) => {
            debug!("ws recv: remote_insert applied, changes={}", changes.len());
            Some(changes)
        }
        Err(e) => {
            error!("ws recv: remote_insert failed: {e:?}");
            None
        }
    }
}

fn handle_delete(document: &mut Document, delete_set: DeleteSet) -> Option<Vec<RemoteChange>> {
    match document.apply_delete_set(&delete_set) {
        Ok(changes) => {
            debug!(
                "ws recv: apply_delete_set applied, changes={}",
                changes.len()
            );
            Some(changes)
        }
        Err(e) => {
            error!("ws recv: apply_delete_set failed: {e:?}");
            None
        }
    }
}

fn emit_remote_changes(app: &AppHandle, changes: Vec<RemoteChange>) {
    for change in changes {
        let event: RemoteChangeEvent = change.into();
        debug!("ws recv: emitting remote change: {:?}", event);
        if let Err(e) = app.emit(REMOTE_CHANGE_EVENT, &event) {
            warn!("ws recv: failed to emit remote change event: {e}");
        }
    }
}
