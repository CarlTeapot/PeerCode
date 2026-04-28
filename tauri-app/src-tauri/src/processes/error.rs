use crate::session::session_types::{SessionErrorPayload, SESSION_ERROR};
use crate::state::appstate::AppState;
use tauri::{AppHandle, Emitter, Manager};

pub fn emit_error(app: &AppHandle, message: String) {
    app.state::<AppState>().teardown_host();
    let _ = app.emit(SESSION_ERROR, SessionErrorPayload { message });
}
