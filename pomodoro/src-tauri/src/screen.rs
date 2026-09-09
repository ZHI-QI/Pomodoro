use std::time::Duration;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, PostMessageW, WM_SYSCOMMAND};

/// 休息时长：完成番茄后黑屏休息的秒数（硬性，不可打断）
pub const REST_SECS: u64 = 60;

const SC_MONITORPOWER: usize = 0xF170;
const MONITOR_OFF: isize = 2;
const MONITOR_ON: isize = -1;

fn monitor_power(off: bool) {
    unsafe {
        // 必须广播到所有顶层窗口(HWND_BROADCAST)；NULL hwnd 仅投递到
        // 自身线程队列，显示器驱动不会响应——此前息屏无效的根因
        let _ = PostMessageW(
            Some(HWND_BROADCAST),
            WM_SYSCOMMAND,
            WPARAM(SC_MONITORPOWER),
            LPARAM(if off { MONITOR_OFF } else { MONITOR_ON }),
        );
    }
}

/// 关闭显示器电源，secs 秒后自动唤醒
pub fn rest_screen(secs: u64) {
    monitor_power(true);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(secs)).await;
        monitor_power(false);
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
