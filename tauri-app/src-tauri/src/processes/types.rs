use crate::session::{SessionErrorPayload, SESSION_ERROR};
use crate::state::appstate::AppState;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;

pub struct GatewayWorkflowResult {
    pub lan_url: String,
    pub port: u16,
    pub room_id: String,
    pub log_rx: Receiver<CommandEvent>,
}

#[derive(serde::Deserialize)]
pub struct RoomResponse {
    pub(crate) room_id: String,
}
pub struct TunnelWorkflowResult {
    pub public_url: String,
    pub room_id: String,
    pub log_rx: Receiver<CommandEvent>,
}

pub fn emit_error(app: &AppHandle, message: String) {
    app.state::<AppState>().teardown_host();
    let _ = app.emit(SESSION_ERROR, SessionErrorPayload { message });
}
