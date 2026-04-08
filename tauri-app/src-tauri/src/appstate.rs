use crdt_core::types::ClientId;
use crdt_core::Document;
use std::sync::Mutex;

pub struct AppState {
    pub document: Mutex<Document>,
}

impl AppState {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            document: Mutex::new(Document::new(client_id)),
        }
    }
}
