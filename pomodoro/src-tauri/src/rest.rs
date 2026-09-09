use std::io::Write;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// 休息时长：完成番茄后黑洞动画休息的秒数（硬性，不可打断）
pub const REST_SECS: u64 = 60;

/// 诊断日志：写 %LOCALAPPDATA%\Pomodoro\debug.log（release 无控制台，用文件取证）
fn log_line(msg: &str) {
    let Ok(base) = std::env::var("LOCALAPPDATA") else { return };
    let p = std::path::PathBuf::from(base).join("Pomodoro").join("debug.log");
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let ts = chrono::Local::now().format("%H:%M:%S");
        let _ = writeln!(f, "[{ts}] {msg}");
    }
}

#[tauri::command]
pub fn debug_log(msg: String) {
    log_line(&msg);
}

/// 显示全屏黑洞休息窗，REST_SECS 秒后自动淡出隐藏
pub fn start_rest(app: &AppHandle) {
    log_line("== start_rest called ==");
    let Some(win) = app.get_webview_window("rest") else {
        log_line("ERROR: rest window not found");
        return;
    };
    let vis = win.is_visible().unwrap_or(false);
    log_line(&format!("rest window is_visible={vis}"));
    if vis {
        log_line("already resting, ignore duplicate");
        return;
    }
    if let Err(e) = app.emit("rest_start", ()) {
        log_line(&format!("ERROR: emit rest_start failed: {e}"));
    } else {
        log_line("emit rest_start ok");
    }
    if let Err(e) = win.show() {
        log_line(&format!("ERROR: show failed: {e}"));
    } else {
        log_line("show ok");
    }
    let _ = win.set_focus();

    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        // 提前 2 秒发淡出信号，前端过渡后隐藏
        tokio::time::sleep(Duration::from_secs(REST_SECS.saturating_sub(2))).await;
        log_line("emit rest_fade");
        let _ = h.emit("rest_fade", ());
        tokio::time::sleep(Duration::from_secs(2)).await;
        if let Some(w) = h.get_webview_window("rest") {
            let _ = w.hide();
            log_line("rest hidden");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_secs_is_one_minute() {
        assert_eq!(REST_SECS, 60);
    }
}
