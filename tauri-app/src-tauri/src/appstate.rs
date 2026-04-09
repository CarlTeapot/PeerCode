use crdt_core::types::ClientId;
use crdt_core::Document;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
pub struct AppState {
    pub document: Mutex<Document>,
    #[cfg(debug_assertions)]
    pub crdt_logging_enabled: AtomicBool,
}

impl AppState {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            document: Mutex::new(Document::new(client_id)),
            #[cfg(debug_assertions)]
            crdt_logging_enabled: AtomicBool::new(false),
        }
    }
}
