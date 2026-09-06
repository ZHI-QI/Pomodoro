use crate::models::{Session, Unit};
use crate::notify;
use crate::settings::Settings;
use crate::timer::{self, ActiveTimer, Tick};
use crate::AppState;
use chrono::{Datelike, Local};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn now_iso() -> String {
    Local::now().to_rfc3339()
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TickPayload {
    pub session_id: i64,
    pub remaining_sec: i64,
    pub planned_sec: i64,
    pub note_short: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DonePayload {
    pub session_id: i64,
    pub note: String,
    pub planned_sec: i64,
    pub note_len: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayFocus {
    pub day: String,
    pub focus_sec: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsPayload {
    pub today_sec: i64,
    pub week_sec: i64,
    pub completed_count: i64,
    pub by_day: Vec<DayFocus>,
}

/// 每秒 tick 循环：以 deadline 墙钟比对，休眠唤醒自动校准
pub fn spawn_tick(app: AppHandle, t: ActiveTimer) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let state = app.state::<AppState>();
            let now = Local::now();
            let current = state.timer.lock().unwrap().clone();
            let Some(active) = current else { break };
            if active.session_id != t.session_id {
                break; // 被新任务替换或已放弃
            }
            match timer::poll(&active, now) {
                Tick::Running(rem) => {
                    let note_short = state
                        .store
                        .lock()
                        .unwrap()
                        .get(active.session_id)
                        .ok()
                        .flatten()
                        .map(|s| s.note.chars().take(8).collect::<String>())
                        .unwrap_or_default();
                    let _ = app.emit(
                        "tick",
                        TickPayload {
                            session_id: active.session_id,
                            remaining_sec: rem,
                            planned_sec: active.planned_sec,
                            note_short,
                        },
                    );
                }
                Tick::Finished => {
                    let ended = now_iso();
                    let session = {
                        let store = state.store.lock().unwrap();
                        let _ = store.finish(active.session_id, "completed", active.planned_sec, &ended);
                        store.get(active.session_id).ok().flatten()
                    };
                    *state.timer.lock().unwrap() = None;
                    let payload = DonePayload {
                        session_id: active.session_id,
                        note: session.as_ref().map(|s| s.note.clone()).unwrap_or_default(),
                        planned_sec: active.planned_sec,
                        note_len: session.as_ref().map(|s| s.note_len).unwrap_or(0),
                    };
                    let settings = state.store.lock().unwrap().get_settings().unwrap_or_default();
                    if settings.notification {
                        notify::notify_done(&app, &payload.note, payload.planned_sec);
                    }
                    let _ = app.emit("session_done", payload);
                    break;
                }
            }
        }
    });
}

#[tauri::command]
pub fn start_session(
    app: AppHandle,
    state: State<AppState>,
    note: String,
    unit: Unit,
    amount: i64,
) -> Result<Session, String> {
    // 硬上限 5000 字，超出静默截断（规格 §6）
    let note: String = note.chars().take(5000).collect();
    let amount = unit.clamp_amount(amount);
    let planned = unit.seconds(amount);
    let started = now_iso();
    let session = state
        .store
        .lock()
        .unwrap()
        .insert_running(&note, planned, &started)
        .map_err(e)?;
    let t = timer::make_timer(session.id, planned, Local::now());
    *state.timer.lock().unwrap() = Some(t.clone());
    spawn_tick(app, t);
    Ok(session)
}

#[tauri::command]
pub fn abort_session(state: State<AppState>, id: i64) -> Result<Session, String> {
    let now = Local::now();
    let actual = {
        let guard = state.timer.lock().unwrap();
        match guard.as_ref().filter(|t| t.session_id == id) {
            Some(t) => (t.planned_sec - timer::remaining_sec(t, now)).max(0),
            None => 0,
        }
    };
    *state.timer.lock().unwrap() = None;
    let ended = now_iso();
    {
        let store = state.store.lock().unwrap();
        store.finish(id, "aborted", actual, &ended).map_err(e)?;
        store.get(id).map_err(e)?.ok_or_else(|| "session not found".into())
    }
}

#[tauri::command]
pub fn get_active_session(state: State<AppState>) -> Result<Option<Session>, String> {
    state.store.lock().unwrap().get_active().map_err(e)
}

#[tauri::command]
pub fn list_today(state: State<AppState>) -> Result<Vec<Session>, String> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    state.store.lock().unwrap().list_today(&today).map_err(e)
}

#[tauri::command]
pub fn list_recent(state: State<AppState>, limit: i64) -> Result<Vec<Session>, String> {
    state.store.lock().unwrap().list_recent(limit).map_err(e)
}

#[tauri::command]
pub fn get_stats(state: State<AppState>) -> Result<StatsPayload, String> {
    let today = Local::now();
    let today_str = today.format("%Y-%m-%d").to_string();
    let store = state.store.lock().unwrap();
    let mut by_day = Vec::new();
    for i in (0..7).rev() {
        let day = (today - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let focus_sec = store.day_focus(&day).map_err(e)?;
        by_day.push(DayFocus { day, focus_sec });
    }
    // 本周 = 本周一 00:00 起（NaiveDate 逐日累加）
    let mut cursor = today.date_naive() - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let today_date = today.date_naive();
    let mut week_sec = 0i64;
    loop {
        week_sec += store.day_focus(&cursor.format("%Y-%m-%d").to_string()).map_err(e)?;
        if cursor == today_date {
            break;
        }
        cursor += chrono::Duration::days(1);
    }
    Ok(StatsPayload {
        today_sec: store.day_focus(&today_str).map_err(e)?,
        week_sec,
        completed_count: store.completed_count().map_err(e)?,
        by_day,
    })
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    state.store.lock().unwrap().get_settings().map_err(e)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    {
        let store = state.store.lock().unwrap();
        store.save_settings(&settings).map_err(e)?;
    }
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let _ = if settings.autostart { autolaunch.enable() } else { autolaunch.disable() };
    let _ = app.emit("theme_changed", settings.theme.clone());
    Ok(settings)
}
