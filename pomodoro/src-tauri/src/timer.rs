use chrono::{DateTime, Duration, Local};

/// 运行中的计时器；截止时间戳是唯一真相（休眠/改表不漂移）
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveTimer {
    pub session_id: i64,
    pub planned_sec: i64,
    pub started_at: DateTime<Local>,
    pub deadline: DateTime<Local>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tick {
    Running(i64), // 剩余秒
    Finished,
}

pub fn make_timer(session_id: i64, planned_sec: i64, started_at: DateTime<Local>) -> ActiveTimer {
    ActiveTimer {
        session_id,
        planned_sec,
        started_at,
        deadline: started_at + Duration::seconds(planned_sec),
    }
}

pub fn remaining_sec(t: &ActiveTimer, now: DateTime<Local>) -> i64 {
    (t.deadline - now).num_seconds().max(0)
}

pub fn poll(t: &ActiveTimer, now: DateTime<Local>) -> Tick {
    if now >= t.deadline {
        Tick::Finished
    } else {
        Tick::Running(remaining_sec(t, now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 9, 6, h, m, 0).unwrap()
    }

    #[test]
    fn fresh_timer_remaining_equals_planned() {
        let t = make_timer(1, 3600, at(10, 0));
        assert_eq!(remaining_sec(&t, at(10, 0)), 3600);
        assert_eq!(poll(&t, at(10, 0)), Tick::Running(3600));
    }

    #[test]
    fn halfway_and_finish() {
        let t = make_timer(1, 3600, at(10, 0));
        assert_eq!(poll(&t, at(10, 30)), Tick::Running(1800)); // 模拟休眠唤醒重算
        assert_eq!(poll(&t, at(11, 0)), Tick::Finished);
        assert_eq!(poll(&t, at(11, 5)), Tick::Finished); // 过期后再查仍是完成
    }

    #[test]
    fn remaining_never_negative() {
        let t = make_timer(1, 60, at(10, 0));
        assert_eq!(remaining_sec(&t, at(10, 10)), 0);
    }
}
