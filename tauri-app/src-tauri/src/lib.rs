mod appstate;
mod crdt_handler;

use std::thread;
use std::time::Duration;

use crate::appstate::AppState;
use crdt_core::types::ClientId;
use rand::random;
use tauri::Manager;

fn spawn_linked_list_logger(app_handle: tauri::AppHandle) {
    thread::spawn::<_, ()>(move || loop {
        let state = app_handle.state::<AppState>();
        match state.document.lock() {
            Ok(document) => {
                println!("CRDT linked list: {}", document.debug_linked_list());
            }
            Err(_) => {
                eprintln!("CRDT linked list logger: failed to lock document");
            }
        }

        thread::sleep(Duration::from_secs(1));
    });
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::new(ClientId::new(random::<u64>())));
            spawn_linked_list_logger(app.handle().clone());
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            crdt_handler::insert,
            crdt_handler::delete
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
