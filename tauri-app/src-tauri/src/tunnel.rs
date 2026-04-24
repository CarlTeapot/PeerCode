use crate::session::{
    GatewayReadyPayload, SessionErrorPayload, TunnelReadyPayload, GATEWAY_READY, SESSION_ERROR,
    TUNNEL_READY,
};
use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn launch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(msg) = run_gateway(&app).await {
            emit_error(&app, msg);
        }
    });
}

async fn run_gateway(app: &AppHandle) -> Result<(), String> {
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
            return Ok(());
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
                    on_gateway_ready(app, port, &room_id).await;
                    println!("Gateway ready");
                    tauri::async_runtime::spawn(pipe_gateway_logs(rx));
                    return Ok(());
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
        Ok(())
    }
}

async fn on_gateway_ready(app: &AppHandle, port: u16, room_id: &str) {
    let lan_url = get_lan_url(port, room_id).await;

    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Starting) {
            return;
        }
        *role = AppRole::Host {
            room_id: room_id.to_string(),
            lan_url: lan_url.clone(),
            public_url: None,
        };
    }

    let _ = app.emit(
        GATEWAY_READY,
        GatewayReadyPayload {
            lan_url,
            room_id: room_id.to_string(),
            port,
        },
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
                    let ws_url = if raw_url.starts_with("https://") {
                        raw_url.replacen("https://", "wss://", 1)
                    } else {
                        raw_url.replacen("http://", "ws://", 1)
                    };
                    let public_url = format!("{}/ws?room={}", ws_url, room_id);
                    store_public_url(&app, &public_url);
                    let _ = app.emit(
                        TUNNEL_READY,
                        TunnelReadyPayload {
                            public_url,
                            room_id: room_id.clone(),
                        },
                    );

                    let host_client_id = {
                        let state = app.state::<AppState>();
                        let doc = state.document.lock().unwrap();
                        doc.client_id.value
                    };
                    let local_ws_url = format!(
                        "ws://127.0.0.1:{port}/ws?room={room_id}&client_id={host_client_id}"
                    );
                    let ws = app.state::<WsState>();
                    if let Err(e) = ws.connect(&local_ws_url, room_id.clone()).await {
                        eprintln!("[ws] local connection failed (session still running): {e}");
                    }
                    return;
                }
            }
        }

        let is_host = matches!(
            *app.state::<AppState>().role.lock().unwrap(),
            AppRole::Host { .. }
        );
        if is_host {
            emit_error(
                &app,
                "cloudflared exited without producing a tunnel URL".into(),
            );
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
    if let AppRole::Host {
        public_url: ref mut stored,
        ..
    } = *role
    {
        *stored = Some(url.to_string());
    }
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
    let response = gateway_http_post_empty(port, "/rooms").await?;
    parse_room_id_response(&response)
}

async fn gateway_http_post_empty(port: u16, path: &str) -> Result<Vec<u8>, String> {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| format!("connect gateway {path}: {e}"))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| format!("write {path}: {e}"))?;

    let mut buf = Vec::with_capacity(256);
    stream
        .read_to_end(&mut buf)
        .await
        .map_err(|e| format!("read {path}: {e}"))?;
    Ok(buf)
}

fn parse_room_id_response(response: &[u8]) -> Result<String, String> {
    let text = String::from_utf8_lossy(response);
    let body_idx = text
        .find("\r\n\r\n")
        .ok_or_else(|| "gateway /rooms: malformed response (no body)".to_string())?
        + 4;
    let body = text[body_idx..].trim();

    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("gateway /rooms: invalid JSON: {e}"))?;
    parsed
        .get("room_id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| "gateway /rooms: response missing room_id".to_string())
}

async fn pipe_gateway_logs(mut rx: tokio::sync::mpsc::Receiver<CommandEvent>) {
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(b) => print!("[gateway] {}", String::from_utf8_lossy(&b)),
            CommandEvent::Stderr(b) => eprint!("[gateway] {}", String::from_utf8_lossy(&b)),
            _ => {}
        }
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
