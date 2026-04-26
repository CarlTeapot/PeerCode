use crate::processes::types::TunnelWorkflowResult;
use crate::state::appstate::{AppRole, AppState};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub async fn run_cloudflared(
    app: &AppHandle,
    port: u16,
    room_id: &str,
) -> Result<Option<TunnelWorkflowResult>, String> {
    let url_arg = format!("http://localhost:{port}");

    let (mut rx, child) = app
        .shell()
        .sidecar("cloudflared")
        .map_err(|e| format!("cloudflared sidecar not found: {e}"))?
        .args([
            "tunnel",
            "--url",
            &url_arg,
            "--no-autoupdate",
            "--loglevel",
            "info",
            "--http2-origin=false",
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn cloudflared: {e}"))?;

    app.state::<AppState>().processes.lock().unwrap().tunnel = Some(child);

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
                store_public_url(app, &public_url);

                return Ok(Some(TunnelWorkflowResult {
                    public_url,
                    log_rx: rx,
                }));
            }
        }
    }

    let is_host = matches!(
        *app.state::<AppState>().role.lock().unwrap(),
        AppRole::Host { .. }
    );
    if is_host {
        Err("cloudflared exited without producing a tunnel URL".into())
    } else {
        Ok(None)
    }
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
