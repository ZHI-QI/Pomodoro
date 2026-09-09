/// 休息时长：完成番茄后黑屏休息的秒数（硬性，不可打断）
pub const REST_SECS: u64 = 60;

/// 关闭显示器电源，secs 秒后自动唤醒
pub fn rest_screen(secs: u64) {
    imp::monitor_power(true);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        imp::monitor_power(false);
    });
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, PostMessageW, WM_SYSCOMMAND};

    const SC_MONITORPOWER: usize = 0xF170;
    const MONITOR_OFF: isize = 2;
    const MONITOR_ON: isize = -1;

    pub(super) fn monitor_power(off: bool) {
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
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::Command;

    pub(super) fn monitor_power(off: bool) {
        if off {
            // 立即让显示器进入睡眠
            let _ = Command::new("pmset").arg("displaysleepnow").status();
        } else {
            // 声明用户活动以唤醒显示器（assert 持续 2 秒）
            let _ = Command::new("caffeinate").args(["-u", "-t", "2"]).status();
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub(super) fn monitor_power(_off: bool) {
        // 其他平台暂不支持息屏休息
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_secs_is_one_minute() {
        assert_eq!(REST_SECS, 60);
    }
}
