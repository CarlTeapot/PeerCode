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
    Starting,
    Host {
        room_id: String,
        lan_url: Option<String>,
        public_url: Option<String>,
    },
    // add guest laterr
}

impl AppRole {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Undecided => "idle",
            Self::Starting => "starting",
            Self::Host { .. } => "host",
        }
    }
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

    pub fn teardown_host(&self) {
        let mut role = self.role.lock().unwrap();
        if matches!(*role, AppRole::Starting | AppRole::Host { .. }) {
            *role = AppRole::Undecided;
        }

        let mut procs = self.processes.lock().unwrap();
        if let Some(child) = procs.tunnel.take() {
            let _ = child.kill();
        }
        if let Some(child) = procs.gateway.take() {
            let _ = child.kill();
        }
    }
}
