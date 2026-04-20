use std::sync::{Arc, RwLock};

use futures_util::StreamExt;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::ws_management::ws_types::{Stream, WsConnection};

//TODO: handle messages and integrate with crdt
pub async fn receive_loop(
    mut stream: Stream,
    connection: Arc<Mutex<WsConnection>>,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
) {
    while let Some(result) = stream.next().await {
        match result {
            Ok(Message::Text(text)) => {
                eprintln!("[ws recv] text: {text}");
            }
            Ok(Message::Binary(bytes)) => {
                eprintln!("[ws recv] binary ({} bytes): {:?}", bytes.len(), bytes);
            }
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Close(_)) => {
                eprintln!("[ws recv] server closed connection");
                break;
            }
            Err(e) => {
                eprintln!("[ws recv] error: {e}");
                break;
            }
        }
    }

    let mut guard = connection.lock().await;
    if matches!(*guard, WsConnection::Connected { .. }) {
        *write_tx.write().unwrap() = None;
        *guard = WsConnection::Disconnected;
        eprintln!("[ws recv] connection lost — state reset to Disconnected");
    }
}
