use crate::processes::process_coordinator;
use crate::session::session_types::{JoinInfo, SessionInfo};
use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use tauri::{AppHandle, Manager, State};
use url::Url;

#[tauri::command]
pub async fn start_host_session(app: AppHandle) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !role.can_initiate_session() {
            return Err("A session is already active".into());
        }
        *role = AppRole::Starting;
    }

    let result = process_coordinator::launch(app.clone()).await?;

    connect(app, result.port, result.room_id).await;
    Ok(())
}

#[tauri::command]
pub async fn join_session(
    url: String,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<(), String> {
    println!("{:#?}", url);
    let join_info = parse_join_url(url)?;

    {
        let mut role = state.role.lock().unwrap();
        if !role.can_initiate_session() {
            return Err("A session is already active".into());
        }
        *role = AppRole::Starting;
    }

    let guest_client_id = {
        let doc = state.document.lock().unwrap();
        doc.client_id.value
    };

    let ws_url = format!(
        "{}/ws?room={}&client_id={}",
        join_info.server_url, join_info.room_id, guest_client_id
    );

    ws.connect(&ws_url, join_info.room_id.clone())
        .await
        .map_err(|e| {
            *state.role.lock().unwrap() = AppRole::Undecided;
            e.to_string()
        })?;

    let should_disconnect = {
        let mut role = state.role.lock().unwrap();
        if matches!(*role, AppRole::Starting) {
            *role = AppRole::Guest {
                room_id: join_info.room_id.clone(),
                server_url: join_info.server_url.clone(),
            };
            false
        } else {
            true
        }
    };
    if !should_disconnect {
        return Ok(());
    }

    let _ = ws.disconnect().await;

    Err("Join session was cancelled".into())
}

#[tauri::command]
pub fn stop_host_session(state: State<'_, AppState>) -> Result<(), String> {
    state.teardown_host();
    Ok(())
}

#[tauri::command]
pub async fn disconnect_websocket(ws: State<'_, WsState>) -> Result<(), String> {
    ws.disconnect().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> SessionInfo {
    let role = state.role.lock().unwrap();
    let (lan_url, public_url, room_id) = match &*role {
        AppRole::Host {
            room_id,
            lan_url,
            public_url,
            ..
        } => (lan_url.clone(), public_url.clone(), Some(room_id.clone())),
        AppRole::Guest {
            room_id,
            server_url,
        } => (None, Some(server_url.clone()), Some(room_id.clone())),
        _ => (None, None, None),
    };
    SessionInfo {
        status: role.status().into(),
        lan_url,
        public_url,
        room_id,
    }
}

#[tauri::command]
pub fn parse_join_url(url: String) -> Result<JoinInfo, String> {
    let parsed = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

    let scheme = parsed.scheme();
    if scheme != "ws" && scheme != "wss" {
        return Err("Invalid URL: must begin with ws:// or wss://".to_string());
    }

    if parsed
        .host_str()
        .map(|h| h.trim().is_empty())
        .unwrap_or(true)
    {
        return Err("Invalid URL: missing host".to_string());
    }

    let room_id = parsed
        .query_pairs()
        .find(|(k, _)| k == "room")
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| "URL is missing the ?room= parameter".to_string())?;

    let mut base_path = parsed.path().trim_end_matches('/').to_string();
    if base_path.ends_with("/ws") {
        base_path.truncate(base_path.len() - 3);
    }
    if base_path.is_empty() {
        base_path.push('/');
    }

    let mut server_url = format!("{}://{}", scheme, parsed.host_str().unwrap());
    if let Some(port) = parsed.port() {
        server_url.push(':');
        server_url.push_str(&port.to_string());
    }
    if base_path != "/" {
        server_url.push_str(&base_path);
    }

    Ok(JoinInfo {
        server_url,
        room_id,
    })
}

async fn connect(app: AppHandle, port: u16, room_id: String) {
    let host_client_id = {
        let state = app.state::<AppState>();
        let doc = state.document.lock().unwrap();
        doc.client_id.value
    };

    let local_ws_url =
        format!("ws://127.0.0.1:{port}/ws?room={room_id}&client_id={host_client_id}");
    let ws = app.state::<WsState>();
    if let Err(e) = ws.connect(&local_ws_url, room_id.clone()).await {
        eprintln!("[ws] local connection failed (session still running): {e}");
    }
}
