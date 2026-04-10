use crate::appstate::{AppRole, AppState};
use crate::session::{
    GatewayReadyPayload, SessionErrorPayload, TunnelReadyPayload,
    GATEWAY_READY, SESSION_ERROR, TUNNEL_READY,
};
use rand::Rng;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;


pub fn generate_room_id() -> String {
    format!("{:08x}", rand::thread_rng().gen::<u32>())
}


pub fn launch(app: AppHandle, room_id: String) {
    tauri::async_runtime::spawn(async move {
        if let Err(msg) = run_gateway(&app, &room_id).await {
            emit_error(&app, msg);
        }
    });
}


async fn run_gateway(app: &AppHandle, room_id: &str) -> Result<(), String> {
    let (mut rx, child) = app
        .shell()
        .sidecar("peercode-gateway")
        .map_err(|e| format!("Gateway sidecar not found: {e}"))?
        .spawn()
        .map_err(|e| format!("Failed to spawn gateway: {e}"))?;

    {
        let state = app.state::<AppState>();
        let role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Starting) {
            let _ = child.kill();
            return Ok(());
        }
        drop(role);
        state.processes.lock().unwrap().gateway = Some(child);
    }

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(bytes) = event {
            let line = String::from_utf8_lossy(&bytes);
            if let Some(port_str) = line.trim().strip_prefix("PORT=") {
                if let Ok(port) = port_str.parse::<u16>() {
                    on_gateway_ready(app, port, room_id).await;
                    return Ok(());
                }
            }
        }
    }

    let still_starting =
        matches!(*app.state::<AppState>().role.lock().unwrap(), AppRole::Starting);
    if still_starting {
        Err("Gateway exited before reporting its port".into())
    } else {
        Ok(())
    }
}

async fn on_gateway_ready(app: &AppHandle, port: u16, room_id: &str) {
    let state = app.state::<AppState>();

    {
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Starting) {
            return;
        }
        *role = AppRole::Host {
            room_id: room_id.to_string(),
            lan_url: None,
            public_url: None,
        };
    }

    let lan_url = get_lan_url(port, room_id).await;

    {
        let mut role = state.role.lock().unwrap();
        match *role {
            AppRole::Host { lan_url: ref mut stored, .. } => *stored = lan_url.clone(),
            _ => return,
        }
    }

    let _ = app.emit(
        GATEWAY_READY,
        GatewayReadyPayload { lan_url, room_id: room_id.to_string(), port },
    );

    run_cloudflared(app.clone(), port, room_id.to_string());
}


fn run_cloudflared(app: AppHandle, port: u16, room_id: String) {
    let url_arg = format!("http://localhost:{port}");

    let sidecar = match app.shell().sidecar("cloudflared") {
        Ok(s) => s,
        Err(e) => return emit_error(&app, format!("cloudflared sidecar not found: {e}")),
    };

    let (mut rx, child) = match sidecar
        .args(["tunnel", "--url", &url_arg, "--no-autoupdate"])
        .spawn()
    {
        Ok(res) => res,
        Err(e) => return emit_error(&app, format!("Failed to spawn cloudflared: {e}")),
    };

    app.state::<AppState>().processes.lock().unwrap().tunnel = Some(child);

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if let CommandEvent::Stderr(bytes) = event {
                let line = String::from_utf8_lossy(&bytes);
                if let Some(raw_url) = extract_tunnel_url(&line) {
                    let public_url =
                        format!("{}/ws?room={}", raw_url.replacen("http", "ws", 1), room_id);
                    store_public_url(&app, &public_url);
                    let _ = app.emit(
                        TUNNEL_READY,
                        TunnelReadyPayload { public_url, room_id },
                    );
                    return;
                }
            }
        }

        let is_host =
            matches!(*app.state::<AppState>().role.lock().unwrap(), AppRole::Host { .. });
        if is_host {
            emit_error(&app, "cloudflared exited without producing a tunnel URL".into());
        }
    });
}

fn extract_tunnel_url(line: &str) -> Option<String> {
    let start = line.find("https://").or_else(|| line.find("http://"))?;
    let rest = &line[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '|' || c == '+' || c == '"' || c == '\x1b')
        .unwrap_or(rest.len());
    let url = rest[..end].trim().to_string();
    url.contains("trycloudflare.com").then_some(url)
}


fn emit_error(app: &AppHandle, message: String) {
    app.state::<AppState>().teardown_host();
    let _ = app.emit(SESSION_ERROR, SessionErrorPayload { message });
}

fn store_public_url(app: &AppHandle, url: &str) {
    let state = app.state::<AppState>();
    let mut role = state.role.lock().unwrap();
    if let AppRole::Host { public_url: ref mut stored, .. } = *role {
        *stored = Some(url.to_string());
    }
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
