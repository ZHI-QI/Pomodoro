use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// 休息时长：完成番茄后黑洞动画休息的秒数（硬性，不可打断）
pub const REST_SECS: u64 = 60;

/// 显示全屏黑洞休息窗，REST_SECS 秒后自动淡出隐藏
pub fn start_rest(app: &AppHandle) {
    let Some(win) = app.get_webview_window("rest") else { return };
    if win.is_visible().unwrap_or(false) {
        return; // 已在休息中，忽略重复触发
    }
    let _ = app.emit("rest_start", ());
    let _ = win.show();
    let _ = win.set_focus();

    let h = app.clone();
    tauri::async_runtime::spawn(async move {
        // 提前 2 秒发淡出信号，前端过渡后隐藏
        tokio::time::sleep(Duration::from_secs(REST_SECS.saturating_sub(2))).await;
        let _ = h.emit("rest_fade", ());
        tokio::time::sleep(Duration::from_secs(2)).await;
        if let Some(w) = h.get_webview_window("rest") {
            let _ = w.hide();
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
