use crdt_core::structs::Block;
use crdt_core::{decode_op, OpMessage, RemoteChange};
use log::{debug, error, warn};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::appstate::AppState;
use crate::ws_management::ws_types::RemoteChangeEvent;

pub const REMOTE_CHANGE_EVENT: &str = "crdt://remote-change";

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
        OpMessage::Insert(wire_block) => {
            let block: Block = wire_block.into();
            match document.remote_insert(block) {
                Ok(changes) => {
                    debug!("ws recv: remote_insert applied, changes={}", changes.len());
                    changes
                }
                Err(e) => {
                    error!("ws recv: remote_insert failed: {e:?}");
                    return;
                }
            }
        }
        OpMessage::Delete(delete_set) => match document.apply_delete_set(&delete_set) {
            Ok(changes) => {
                debug!(
                    "ws recv: apply_delete_set applied, changes={}",
                    changes.len()
                );
                changes
            }
            Err(e) => {
                error!("ws recv: apply_delete_set failed: {e:?}");
                return;
            }
        },
    };

    drop(document);

    for change in changes {
        let event: RemoteChangeEvent = change.into();
        debug!("ws recv: emitting remote change: {:?}", event);
        if let Err(e) = app.emit(REMOTE_CHANGE_EVENT, &event) {
            warn!("ws recv: failed to emit remote change event: {e}");
        }
    }
}
