use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

pub fn notify_done(app: &AppHandle, note: &str, planned_sec: i64) {
    let body = if note.is_empty() {
        format!("专注 {} 分钟", planned_sec / 60)
    } else {
        format!("「{}」专注 {} 分钟", truncate(note, 24), planned_sec / 60)
    };
    let _ = app
        .notification()
        .builder()
        .title("🍅 番茄完成！")
        .body(body)
        .show();
}

#[cfg(test)]
mod tests {
    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(super::truncate("短", 10), "短");
        assert_eq!(super::truncate("一二三四五", 3), "一二三…");
    }
}
