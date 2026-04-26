use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;

pub struct GatewayWorkflowResult {
    pub lan_url: Option<String>,
    pub port: u16,
    pub room_id: String,
    pub log_rx: Receiver<CommandEvent>,
}

pub struct TunnelWorkflowResult {
    pub public_url: String,
    pub log_rx: Receiver<CommandEvent>,
}

pub struct CombinedWorkflowResult {
    pub port: u16,
    pub room_id: String,
}
