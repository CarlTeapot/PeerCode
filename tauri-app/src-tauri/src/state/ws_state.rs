use std::time::Duration;

use crate::ws_types::{Stream, WsConnection, WsError};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct WsState {
    connection: Mutex<WsConnection>,
    connect_timeout: Duration,
}

impl WsState {
    pub fn new(connect_timeout: Duration) -> Self {
        Self {
            connection: Mutex::new(WsConnection::Disconnected),
            connect_timeout,
        }
    }

    pub async fn connect(&self, url: &str, session_id: String) -> Result<(), WsError> {
        {
            let mut guard = self.connection.lock().await;
            if !matches!(*guard, WsConnection::Disconnected) {
                return Err(WsError::AlreadyConnected);
            }
            *guard = WsConnection::Connecting;
        }

        let outcome = timeout(self.connect_timeout, connect_async(url)).await;

        let (ws_stream, _response) = match outcome {
            Ok(Ok(conn)) => conn,
            Ok(Err(e)) => {
                *self.connection.lock().await = WsConnection::Disconnected;
                return Err(WsError::Handshake {
                    url: url.to_string(),
                    cause: e.to_string(),
                });
            }
            Err(_) => {
                *self.connection.lock().await = WsConnection::Disconnected;
                return Err(WsError::Timeout {
                    url: url.to_string(),
                    secs: self.connect_timeout.as_secs(),
                });
            }
        };

        let (sink, stream) = ws_stream.split();
        let reader = tokio::task::spawn(receive_loop(stream));

        let mut guard = self.connection.lock().await;
        if !matches!(*guard, WsConnection::Connecting) {
            reader.abort();
            return Err(WsError::Cancelled);
        }
        *guard = WsConnection::Connected {
            sink,
            session_id: session_id.clone(),
            reader,
        };

        eprintln!("[ws] connected  url={url}  room={session_id}");
        Ok(())
    }

    pub async fn send(&self, msg: Message) -> Result<(), WsError> {
        let mut guard = self.connection.lock().await;
        match &mut *guard {
            WsConnection::Connected { sink, .. } => sink
                .send(msg)
                .await
                .map_err(|e| WsError::SendFailed(e.to_string())),
            _ => Err(WsError::NotConnected),
        }
    }
}

async fn receive_loop(mut stream: Stream) {
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
}
