use std::sync::{Arc, RwLock};

use futures_util::StreamExt;
use log::{debug, info, warn};
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::crdt::remote_op_handler::handle_remote_binary;
use crate::ws_management::ws_types::{Stream, WsConnection};

pub async fn receive_loop(
    mut stream: Stream,
    connection: Arc<Mutex<WsConnection>>,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
    app: AppHandle,
) {
    info!("ws receiver loop started");
    while let Some(result) = stream.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("ws recv text (len={}): {text}", text.len());
            }
            Ok(Message::Binary(bytes)) => {
                debug!("ws receiver binary message (bytes={})", bytes.len());
                handle_remote_binary(&app, bytes.as_ref());
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
