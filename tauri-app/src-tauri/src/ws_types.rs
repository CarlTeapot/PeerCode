use futures_util::stream::{SplitSink, SplitStream};
use std::fmt::Display;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type Sink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type Stream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub enum WsConnection {
    Disconnected,
    Connecting,
    Connected {
        sink: Sink,
        #[allow(dead_code)]
        session_id: String,
        #[allow(dead_code)]
        reader: JoinHandle<()>,
    },
}

#[derive(Debug)]
pub enum WsError {
    AlreadyConnected,
    Timeout { url: String, secs: u64 },
    Handshake { url: String, cause: String },
    NotConnected,
    SendFailed(String),
    Cancelled,
}

impl Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::AlreadyConnected => {
                write!(f, "WebSocket is already connected or connecting")
            }
            WsError::Timeout { url, secs } => {
                write!(f, "WebSocket connect to {url} timed out after {secs}s")
            }
            WsError::Handshake { url, cause } => {
                write!(f, "WebSocket connect to {url} failed: {cause}")
            }
            WsError::NotConnected => write!(f, "WebSocket is not connected"),
            WsError::SendFailed(e) => write!(f, "WebSocket send failed: {e}"),
            WsError::Cancelled => write!(f, "WebSocket connection cancelled before handshake"),
        }
    }
}
