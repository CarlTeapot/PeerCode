use crate::app_config::config::AppConfig;
use crate::processes::error::emit_error;
use crate::processes::gateway_process::run_gateway;
use crate::processes::tunnel_process::run_cloudflared;
use crate::processes::types::CombinedWorkflowResult;
use crate::session::session_types::{
    SessionReadyPayload, SESSION_READY,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;

pub async fn launch(app: AppHandle) -> Result<CombinedWorkflowResult, String> {
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

    let lan_url = gateway.lan_url;
    let port = gateway.port;
    let resolved_room_id = gateway.room_id.clone();
    let mut public_url: Option<String> = None;

    match run_cloudflared(&app, port, &gateway.room_id).await {
        Ok(Some(tunnel)) => {
            if logging.show_cloudflared_logs {
                tauri::async_runtime::spawn(pipe_process_logs("cloudflared", tunnel.log_rx));
            }
            public_url = Some(tunnel.public_url);
        }
        Ok(None) => {}
        Err(msg) => {
            emit_error(&app, msg.clone());
            return Err(msg);
        }
    }

    let _ = app.emit(
        SESSION_READY,
        SessionReadyPayload {
            lan_url,
            public_url,
            room_id: resolved_room_id.clone(),
            port,
        },
    );

    Ok(CombinedWorkflowResult {
        port,
        room_id: resolved_room_id,
    })
}

async fn pipe_process_logs(name: &str, mut rx: Receiver<CommandEvent>) {
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(b) => print!("[{name}] {}", String::from_utf8_lossy(&b)),
            CommandEvent::Stderr(b) => eprint!("[{name}] {}", String::from_utf8_lossy(&b)),
            CommandEvent::Terminated(status) => {
                eprintln!("[{name}] terminated: {status:?}");
                break;
            }
            _ => {}
        }
    }
}
