use crate::appstate::{AppRole, AppState};
use crate::tunnel;
use tauri::{AppHandle, Manager, State};


pub const GATEWAY_READY: &str = "session://gateway-ready";
pub const TUNNEL_READY: &str = "session://tunnel-ready";
pub const SESSION_ERROR: &str = "session://session-error";


#[derive(Clone, serde::Serialize)]
pub struct GatewayReadyPayload {
    pub lan_url: Option<String>,
    pub room_id: String,
    pub port: u16,
}

#[derive(Clone, serde::Serialize)]
pub struct TunnelReadyPayload {
    pub public_url: String,
    pub room_id: String,
}

#[derive(Clone, serde::Serialize)]
pub struct SessionErrorPayload {
    pub message: String,
}


#[derive(serde::Serialize)]
pub struct SessionInfo {
    pub status: String,
    pub lan_url: Option<String>,
    pub public_url: Option<String>,
    pub room_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct JoinInfo {
    pub server_url: String,
    pub room_id: String,
}


#[tauri::command]
pub fn start_host_session(app: AppHandle) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Undecided) {
            return Err("A session is already running".into());
        }
        *role = AppRole::Starting;
    }

    tunnel::launch(app, tunnel::generate_room_id());
    Ok(())
}

#[tauri::command]
pub fn stop_host_session(state: State<'_, AppState>) -> Result<(), String> {
    state.teardown_host();
    Ok(())
}

#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> SessionInfo {
    let role = state.role.lock().unwrap();
    match &*role {
        AppRole::Undecided => SessionInfo {
            status: "idle".into(),
            lan_url: None,
            public_url: None,
            room_id: None,
        },
        AppRole::Starting => SessionInfo {
            status: "starting".into(),
            lan_url: None,
            public_url: None,
            room_id: None,
        },
        AppRole::Host { room_id, lan_url, public_url, .. } => SessionInfo {
            status: "host".into(),
            lan_url: lan_url.clone(),
            public_url: public_url.clone(),
            room_id: Some(room_id.clone()),
        },
    }
}

#[tauri::command]
pub fn parse_join_url(url: String) -> Result<JoinInfo, String> {
    if !url.starts_with("ws://") && !url.starts_with("wss://") {
        return Err("Invalid URL: must begin with ws:// or wss://".to_string());
    }

    let (base, query) = url.split_once('?').unwrap_or((&url, ""));

    let room_id = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("room="))
        .map(|v| v.to_string())
        .ok_or_else(|| "URL is missing the ?room= parameter".to_string())?;

    let server_url = base
        .strip_suffix("/ws")
        .unwrap_or(base)
        .trim_end_matches('/')
        .to_string();

    Ok(JoinInfo { server_url, room_id })
}
