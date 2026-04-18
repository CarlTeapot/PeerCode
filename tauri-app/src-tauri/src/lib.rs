mod config;
mod crdt_handler;
mod session;
mod state;
mod tunnel;
mod ws_types;

use std::thread;
use std::time::Duration;

use crate::config::AppConfig;
use crate::state::appstate::AppState;
use crate::state::ws_state::WsState;
use crdt_core::types::ClientId;
use rand::random;
use std::sync::atomic::Ordering;
use tauri::Manager;

#[cfg(debug_assertions)]
fn spawn_linked_list_logger(app_handle: tauri::AppHandle) {
    thread::spawn::<_, ()>(move || loop {
        let state = app_handle.state::<AppState>();

        if state.crdt_logging_enabled.load(Ordering::Relaxed) {
            let text = {
                let document = state.document.lock().unwrap();
                document.debug_linked_list()
            };
            println!("CRDT linked list: {}", text);
        }
        thread::sleep(Duration::from_secs(1));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_config = AppConfig::load();

            app.manage(AppState::new(ClientId::new(random::<u64>())));
            app.manage(WsState::new(app_config.websocket.connect_timeout()));
            app.manage(app_config);

            #[cfg(debug_assertions)]
            spawn_linked_list_logger(app.handle().clone());

            let _ = session::start_host_session(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.state::<AppState>().teardown_host();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            crdt_handler::insert,
            crdt_handler::delete,
            session::start_host_session,
            session::stop_host_session,
            session::parse_join_url,
            session::get_session_info,
            #[cfg(debug_assertions)]
            crdt_handler::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
