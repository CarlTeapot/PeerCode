use crate::appstate::{AppRole, AppState};
use rand::Rng;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

// ── Event Payloads (Sent to React) ────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct GatewayReadyPayload {
    pub lan_url: String,
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
pub struct JoinInfo {
    pub server_url: String,
    pub room_id: String,
}

// ── Tauri Commands (Called from React) ────────────────────────────────────────

#[tauri::command]
pub async fn start_host_session(app: AppHandle) -> Result<(), String> {
    {
        let state = app.state::<AppState>();
        let role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Undecided) {
            return Err("A session is already running".into());
        }
    }

    let room_id = generate_room_id();

    let (mut gateway_rx, gateway_child) = app
        .shell()
        .sidecar("peercode-gateway")
        .map_err(|e| format!("Gateway sidecar not found: {e}"))?
        .spawn()
        .map_err(|e| format!("Failed to spawn gateway: {e}"))?;

    app.state::<AppState>().processes.lock().unwrap().gateway = Some(gateway_child);

    tauri::async_runtime::spawn(async move {
        while let Some(event) = gateway_rx.recv().await {
            if let CommandEvent::Stdout(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                
                if let Some(port_str) = line.trim().strip_prefix("PORT=") {
                    if let Ok(port) = port_str.parse::<u16>() {
                        handle_gateway_started(app.clone(), port, room_id);
                        break;
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn stop_host_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut procs = state.processes.lock().unwrap();
    
    if let Some(child) = procs.tunnel.take() {
        let _ = child.kill();
    }
    
    if let Some(child) = procs.gateway.take() {
        let _ = child.kill();
    }
    
    *state.role.lock().unwrap() = AppRole::Undecided;
    Ok(())
}

#[tauri::command]
pub fn parse_join_url(url: String) -> Result<JoinInfo, String> {
    let (base, query) = url.split_once('?').unwrap_or((&url, ""));

    let room_id = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("room="))
        .map(|v| v.to_string())
        .ok_or_else(|| "Missing 'room' parameter in URL".to_string())?;

    let server_url = base.rfind("/ws")
        .map(|pos| base[..pos].to_string())
        .unwrap_or_else(|| base.trim_end_matches('/').to_string());

    Ok(JoinInfo { server_url, room_id })
}

// ── Internal Helpers ──────────────────────────────────────────────────────────

fn handle_gateway_started(app: AppHandle, port: u16, room_id: String) {
    *app.state::<AppState>().role.lock().unwrap() = AppRole::Host {
        port,
        room_id: room_id.clone(),
    };

    let lan_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());
        
    let lan_url = format!("ws://{}:{}/ws?room={}", lan_ip, port, room_id);
    
    let _ = app.emit("session://gateway-ready", GatewayReadyPayload {
        lan_url,
        room_id: room_id.clone(),
        port,
    });

    spawn_cloudflared(app, port, room_id);
}

fn spawn_cloudflared(app: AppHandle, port: u16, room_id: String) {
    let url_arg = format!("http://localhost:{}", port);

    let (mut tunnel_rx, tunnel_child) = match app.shell().sidecar("cloudflared")
        .unwrap()
        .args(["tunnel", "--url", &url_arg, "--no-autoupdate"])
        .spawn() 
    {
        Ok(res) => res,
        Err(_) => return,
    };

    app.state::<AppState>().processes.lock().unwrap().tunnel = Some(tunnel_child);

    tauri::async_runtime::spawn(async move {
        while let Some(event) = tunnel_rx.recv().await {
            if let CommandEvent::Stderr(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                
                if let Some(https_url) = extract_tunnel_url(&line) {
                    let wss_url = https_url.replacen("https://", "wss://", 1);
                    let public_url = format!("{}/ws?room={}", wss_url, room_id);
                    
                    let _ = app.emit("session://tunnel-ready", TunnelReadyPayload {
                        public_url,
                        room_id,
                    });
                    break;
                }
            }
        }
    });
}

fn extract_tunnel_url(line: &str) -> Option<String> {
    let start = line.find("https://")?;
    let rest = &line[start..];
    let end = rest.find(|c: char| c.is_whitespace() || c == '|' || c == '+' || c == '"').unwrap_or(rest.len());
    
    let url = rest[..end].trim().to_string();
    if url.contains("trycloudflare.com") { Some(url) } else { None }
}

fn generate_room_id() -> String {
    format!("{:06x}", rand::thread_rng().gen::<u32>() & 0x00FF_FFFF)
}