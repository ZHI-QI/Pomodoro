pub mod commands;
pub mod models;
pub mod notify;
pub mod settings;
pub mod store;
pub mod timer;

use commands::*;
use serde::Serialize;
use std::sync::Mutex;
use store::{OpenOutcome, Store};
use tauri::{Emitter, Manager};

pub struct AppState {
    pub store: Mutex<Store>,
    pub timer: Mutex<Option<timer::ActiveTimer>>,
    /// 进程生命周期内不变：本次启动的数据库打开结果（R3：前端主动拉取展示横幅）
    pub outcome: OpenOutcome,
}

fn db_path() -> std::path::PathBuf {
    std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Pomodoro")
        .join("data.db")
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StoreStatePayload {
    pub outcome: &'static str,
}

pub(crate) fn outcome_label(o: OpenOutcome) -> &'static str {
    match o {
        OpenOutcome::Opened => "opened",
        OpenOutcome::Recovered => "recovered",
        OpenOutcome::Reset => "reset",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let path = db_path();
    let (store, outcome) = match Store::open(&path) {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("fatal: db unrecoverable: {err}");
            std::process::exit(1);
        }
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            store: Mutex::new(store),
            timer: Mutex::new(None),
            outcome,
        })
        .setup({
            let outcome = outcome;
            move |app| {
                let payload = StoreStatePayload {
                    outcome: outcome_label(outcome),
                };
                if outcome == OpenOutcome::Recovered {
                    let _ = app.emit("store_recovered", payload.clone());
                } else if outcome == OpenOutcome::Reset {
                    let _ = app.emit("store_reset", payload);
                }
                recover_running_session(app.handle().clone());
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            abort_session,
            get_active_session,
            list_today,
            list_recent,
            get_stats,
            get_settings,
            set_settings,
            commands::get_store_outcome
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 崩溃恢复：running 记录已过期→补完成并提醒；未过期→续跑（规格 §6）
fn recover_running_session(app: tauri::AppHandle) {    let state = app.state::<AppState>();
    let Some(s) = state.store.lock().unwrap().get_active().ok().flatten() else {
        return;
    };
    let started = chrono::DateTime::parse_from_rfc3339(&s.started_at)
        .map(|d| d.with_timezone(&chrono::Local))
        .unwrap_or_else(|_| chrono::Local::now());
    let t = timer::make_timer(s.id, s.planned_sec, started);
    match timer::poll(&t, chrono::Local::now()) {
        timer::Tick::Finished => {
            let ended = chrono::Local::now().to_rfc3339();
            let settings = state.store.lock().unwrap().get_settings().unwrap_or_default();
            let _ = state
                .store
                .lock()
                .unwrap()
                .finish(s.id, "completed", s.planned_sec, &ended);
            if settings.notification {
                notify::notify_done(&app, &s.note, s.planned_sec);
            }
        }
        timer::Tick::Running(_) => {
            *state.timer.lock().unwrap() = Some(t.clone());
            commands::spawn_tick(app, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_label_maps_all_variants() {
        assert_eq!(outcome_label(OpenOutcome::Opened), "opened");
        assert_eq!(outcome_label(OpenOutcome::Recovered), "recovered");
        assert_eq!(outcome_label(OpenOutcome::Reset), "reset");
    }

    #[test]
    fn store_outcome_payload_serializes_snake_case() {
        let json = serde_json::to_string(&StoreStatePayload { outcome: "reset" }).unwrap();
        assert_eq!(json, r#"{"outcome":"reset"}"#);
    }
}
