use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use crdt_core::wire::{encode_control_json, PermissionChange, CONTROL_PERMISSION_CHANGE};
use log::{info, warn};
use tauri::State;

#[tauri::command]
pub async fn set_peer_permission(
    target_client_id: String,
    can_write: bool,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<(), String> {
    info!(
        "set_peer_permission requested: target={}, can_write={}",
        target_client_id, can_write
    );

    {
        let role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Host { .. }) {
            warn!("set_peer_permission rejected: caller is not host");
            return Err("Only the host can change permissions".into());
        }
    }

    let payload = PermissionChange {
        target_client_id,
        can_write,
    };
    let frame = encode_control_json(CONTROL_PERMISSION_CHANGE, &payload);
    ws.send_raw(frame).await;
    info!("permission change frame sent to gateway");
    Ok(())
}
