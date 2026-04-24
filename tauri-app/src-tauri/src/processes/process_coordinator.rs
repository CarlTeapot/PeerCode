use crate::config::AppConfig;
use crate::processes::gateway_process::run_gateway;
use crate::processes::tunnel_process::run_cloudflared;
use crate::processes::types::emit_error;
use crate::session::{GatewayReadyPayload, TunnelReadyPayload, GATEWAY_READY, TUNNEL_READY};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;
pub async fn launch(app: AppHandle) -> Result<(u16, String), String> {
    let logging = app.state::<AppConfig>().logging.clone();

    let gateway = match run_gateway(&app).await {
        Ok(Some(r)) => r,
        Ok(None) => return Err("Gateway did not start".into()),
        Err(msg) => {
            emit_error(&app, msg.clone());
            return Err(msg);
        }
    };

    if logging.show_gateway_logs {
        tauri::async_runtime::spawn(pipe_process_logs("gateway", gateway.log_rx));
    }

    let _ = app.emit(
        GATEWAY_READY,
        GatewayReadyPayload {
            lan_url: Some(gateway.lan_url),
            room_id: gateway.room_id.clone(),
            port: gateway.port,
        },
    );

    let port = gateway.port;
    let resolved_room_id = gateway.room_id.clone();

    match run_cloudflared(&app, port, &gateway.room_id).await {
        Ok(Some(tunnel)) => {
            if logging.show_cloudflared_logs {
                tauri::async_runtime::spawn(pipe_process_logs("cloudflared", tunnel.log_rx));
            }

            let _ = app.emit(
                TUNNEL_READY,
                TunnelReadyPayload {
                    public_url: tunnel.public_url,
                    room_id: tunnel.room_id,
                },
            );
        }
        Ok(None) => {}
        Err(msg) => {
            emit_error(&app, msg.clone());
            return Err(msg);
        }
    }

    Ok((port, resolved_room_id))
}

async fn pipe_process_logs(name: &str, mut rx: Receiver<CommandEvent>) {
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(b) => print!("[{name}] {}", String::from_utf8_lossy(&b)),
            CommandEvent::Stderr(b) => eprint!("[{name}] {}", String::from_utf8_lossy(&b)),
            _ => {}
        }
    }
}
