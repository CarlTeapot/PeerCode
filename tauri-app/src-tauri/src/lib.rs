mod appstate;
mod crdt_handler;

use std::thread;
use std::time::Duration;

use crate::appstate::AppState;
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
            app.manage(AppState::new(ClientId::new(random::<u64>())));
            #[cfg(debug_assertions)]
            spawn_linked_list_logger(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            crdt_handler::insert,
            crdt_handler::delete,
            #[cfg(debug_assertions)]
            crdt_handler::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
