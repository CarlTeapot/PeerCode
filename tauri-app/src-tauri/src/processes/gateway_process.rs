use crate::processes::types::GatewayWorkflowResult;
use crate::state::appstate::{AppRole, AppState};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;

#[derive(serde::Deserialize)]
struct RoomResponse {
    room_id: String,
}

pub async fn run_gateway(app: &AppHandle) -> Result<Option<GatewayWorkflowResult>, String> {
    let (mut rx, child) = app
        .shell()
        .sidecar("peercode-gateway")
        .map_err(|e| format!("Gateway sidecar not found: {e}"))?
        .spawn()
        .map_err(|e| format!("Failed to spawn gateway: {e}"))?;

    {
        let state = app.state::<AppState>();
        let role = state.role.lock().unwrap();
        println!("{}", role.status());
        if !matches!(*role, AppRole::Starting) {
            let _ = child.kill();
            return Ok(None);
        }
        drop(role);
        state.processes.lock().unwrap().gateway = Some(child);
    }

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(bytes) = event {
            let line = String::from_utf8_lossy(&bytes);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if let Some(port) = json.get("port").and_then(|v| v.as_u64()).map(|v| v as u16) {
                    let room_id = fetch_room_id(port).await?;
                    let result = on_gateway_ready(app, port, &room_id, rx).await;
                    println!("Gateway ready");
                    return Ok(result);
                }
            }
        }
    }

    let still_starting = matches!(
        *app.state::<AppState>().role.lock().unwrap(),
        AppRole::Starting
    );
    if still_starting {
        Err("Gateway exited before reporting its port".into())
    } else {
        Ok(None)
    }
}

async fn on_gateway_ready(
    app: &AppHandle,
    port: u16,
    room_id: &str,
    log_rx: Receiver<CommandEvent>,
) -> Option<GatewayWorkflowResult> {
    let lan_url = get_lan_url(port, room_id).await;

    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Starting) {
            return None;
        }
        *role = AppRole::Host {
            room_id: room_id.to_string(),
            lan_url: lan_url.clone(),
            public_url: None,
        };
    }

    Some(GatewayWorkflowResult {
        lan_url,
        port,
        room_id: room_id.to_string(),
        log_rx,
    })
}

async fn get_lan_url(port: u16, room_id: &str) -> Option<String> {
    let room_id = room_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        local_ip_address::local_ip()
            .ok()
            .map(|ip| format!("ws://{}:{}/ws?room={}", ip, port, room_id))
    })
    .await
    .ok()
    .flatten()
}

const FETCH_ROOM_TIMEOUT: Duration = Duration::from_secs(5);

async fn fetch_room_id(port: u16) -> Result<String, String> {
    tokio::time::timeout(FETCH_ROOM_TIMEOUT, fetch_room_id_inner(port))
        .await
        .map_err(|_| {
            format!(
                "gateway /rooms: timed out after {}s",
                FETCH_ROOM_TIMEOUT.as_secs()
            )
        })?
}

async fn fetch_room_id_inner(port: u16) -> Result<String, String> {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/rooms"))
        .send()
        .await
        .map_err(|e| format!("gateway /rooms: {e}"))?
        .error_for_status()
        .map_err(|e| format!("gateway /rooms: {e}"))?
        .json::<RoomResponse>()
        .await
        .map(|r| r.room_id)
        .map_err(|e| format!("gateway /rooms: {e}"))
}
