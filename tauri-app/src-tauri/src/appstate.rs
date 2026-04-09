use crdt_core::types::ClientId;
use crdt_core::Document;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri_plugin_shell::process::CommandChild;

pub struct AppState {
    pub document: Mutex<Document>,
    #[cfg(debug_assertions)]
    pub crdt_logging_enabled: AtomicBool,
    pub role: Mutex<AppRole>,
    pub processes: Mutex<HostProcesses>,
}

pub enum AppRole {
    Undecided,
    Host { port: u16, room_id: String },
    Guest { server_url: String, room_id: String },
}

pub struct HostProcesses {
    pub gateway: Option<CommandChild>,
    pub tunnel: Option<CommandChild>,
}


impl AppState {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            document: Mutex::new(Document::new(client_id)),
            role: Mutex::new(AppRole::Undecided),
            processes: Mutex::new(HostProcesses {
                gateway: None,
                tunnel: None,
            }),
            #[cfg(debug_assertions)]
            crdt_logging_enabled: AtomicBool::new(false),
        }
    }
}
