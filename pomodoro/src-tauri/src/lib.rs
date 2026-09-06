pub mod commands;
pub mod models;
pub mod notify;
pub mod settings;
pub mod store;
pub mod timer;

use commands::*;
use std::sync::Mutex;
use store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub timer: Mutex<Option<timer::ActiveTimer>>,
}

fn db_path() -> std::path::PathBuf {
    std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Pomodoro")
        .join("data.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::open(&db_path()).expect("init sqlite");
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            store: Mutex::new(store),
            timer: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            abort_session,
            get_active_session,
            list_today,
            list_recent,
            get_stats,
            get_settings,
            set_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
