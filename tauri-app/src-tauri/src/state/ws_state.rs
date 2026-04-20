use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::ws_management::ws_receiver::receive_loop;
use crate::ws_management::ws_types::{WsConnection, WsError};
use crate::ws_management::ws_writer::write_loop;

pub struct WsState {
    connection: Arc<Mutex<WsConnection>>,
    connect_timeout: Duration,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
}

impl WsState {
    pub fn new(connect_timeout: Duration) -> Self {
        Self {
            connection: Arc::new(Mutex::new(WsConnection::Disconnected)),
            connect_timeout,
            write_tx: Arc::new(RwLock::new(None)),
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
        let (write_tx, write_rx) = mpsc::channel::<Message>(64);
        let sender = tokio::task::spawn(write_loop(sink, write_rx));
        let receiver = tokio::task::spawn(receive_loop(
            stream,
            Arc::clone(&self.connection),
            Arc::clone(&self.write_tx),
        ));

        let mut guard = self.connection.lock().await;
        if !matches!(*guard, WsConnection::Connecting) {
            receiver.abort();
            sender.abort();
            return Err(WsError::Cancelled);
        }
        *self.write_tx.write().unwrap() = Some(Arc::new(write_tx));
        *guard = WsConnection::Connected {
            session_id: session_id.clone(),
            receiver,
            sender,
        };

        eprintln!("[ws] connected  url={url}  room={session_id}");
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), WsError> {
        let mut guard = self.connection.lock().await;
        match &mut *guard {
            WsConnection::Connected {
                receiver, sender, ..
            } => {
                receiver.abort();
                sender.abort();
                *self.write_tx.write().unwrap() = None;
                *guard = WsConnection::Disconnected;
                eprintln!("[ws] disconnected");
                Ok(())
            }
            _ => Err(WsError::NotConnected),
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), WsError> {
        let arc = {
            let guard = self.write_tx.read().unwrap();
            guard.as_ref().ok_or(WsError::NotConnected)?.clone()
        };

        arc.send(msg)
            .await
            .map_err(|_| WsError::SendFailed("writer task has exited".into()))
    }
}
