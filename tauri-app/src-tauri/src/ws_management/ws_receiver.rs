use crdt_core::wire::{RoomState, CONTROL_ROOM_STATE, CONTROL_SESSION_ENDED, PREFIX_CONTROL};
use futures_util::StreamExt;
use log::{debug, info, warn};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::session::session_types::{SessionEndedPayload, SESSION_ENDED};
use crate::state::appstate::AppState;
use crate::ws_management::ws_types::{Stream, WsConnection};

const ROOM_STATE_EVENT: &str = "session://room-state";
const CAN_WRITE_EVENT: &str = "session://can-write";

pub async fn receive_loop(
    mut stream: Stream,
    connection: Arc<Mutex<WsConnection>>,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
    op_tx: mpsc::UnboundedSender<Vec<u8>>,
    app: AppHandle,
) {
    info!("ws receiver loop started");
    while let Some(result) = stream.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("ws recv text (len={}): {text}", text.len());
            }
            Ok(Message::Binary(bytes)) => {
                if bytes.first().copied() == Some(PREFIX_CONTROL) {
                    match bytes.get(1).copied() {
                        Some(CONTROL_SESSION_ENDED) => {
                            info!("ws recv: session ended by host");
                            let _ = app.emit(SESSION_ENDED, SessionEndedPayload {});
                            break;
                        }
                        Some(CONTROL_ROOM_STATE) => {
                            debug!("ws recv: room state update");
                            if let Ok(state) = serde_json::from_slice::<RoomState>(&bytes[2..]) {
                                update_local_permission(&app, &state).await;
                                let _ = app.emit(ROOM_STATE_EVENT, &state);
                            } else {
                                warn!("ws recv: failed to decode room state");
                            }
                        }
                        other => {
                            warn!("ws recv: unknown control frame type={:?}; ignoring", other);
                        }
                    }
                    continue;
                }
                debug!("ws receiver binary message (bytes={})", bytes.len());
                if op_tx.send(bytes.into()).is_err() {
                    warn!("ws receiver: op processor channel closed; dropping frame");
                }
            }
            Ok(Message::Ping(_)) => {
                debug!("ws receiver ping");
            }
            Ok(Message::Pong(_)) => {
                debug!("ws receiver pong");
            }
            Ok(Message::Frame(_)) => {
                debug!("ws receiver raw frame");
            }
            Ok(Message::Close(_)) => {
                info!("ws recv: server closed connection");
                break;
            }
            Err(e) => {
                warn!("ws recv error: {e}");
                break;
            }
        }
    }

    let mut guard = connection.lock().await;
    if matches!(*guard, WsConnection::Connected { .. }) {
        *write_tx.write().unwrap() = None;
        *guard = WsConnection::Disconnected;
        warn!("ws recv connection lost; state reset to Disconnected");
    }
    info!("ws recv loop stopped");
}

async fn update_local_permission(app: &AppHandle, room_state: &RoomState) {
    let state = app.state::<AppState>();
    let local_client_id = {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if state
            .doc_tx
            .send(crate::state::document::DocOp::GetClientId { reply: tx })
            .await
            .is_err()
        {
            warn!("update_local_permission: doc actor channel closed");
            return;
        }
        match rx.await {
            Ok(id) => id.value.to_string(),
            Err(_) => {
                warn!("update_local_permission: doc actor reply dropped");
                return;
            }
        }
    };
    if let Some(me) = room_state
        .peers
        .iter()
        .find(|p| p.client_id == local_client_id)
    {
        state.can_write.store(me.can_write, Ordering::Relaxed);
        let _ = app.emit(CAN_WRITE_EVENT, me.can_write);
        debug!("local write permission updated: can_write={}", me.can_write);
    }
}
