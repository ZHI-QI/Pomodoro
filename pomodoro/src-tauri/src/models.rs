use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: i64,
    pub note: String,
    pub note_len: i64,
    pub planned_sec: i64,
    pub actual_sec: Option<i64>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
}

impl Session {
    /// 备注字数：按 Unicode 字符计（中文 1 字 = 1）
    pub fn note_len_of(note: &str) -> i64 {
        note.chars().count() as i64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Minute,
    Hour,
    Day,
}

impl Unit {
    pub fn seconds(self, amount: i64) -> i64 {
        match self {
            Unit::Minute => amount * 60,
            Unit::Hour => amount * 3600,
            Unit::Day => amount * 86400,
        }
    }
    /// 防御性钳制（前端已钳制，后端再钳一次）
    pub fn clamp_amount(self, amount: i64) -> i64 {
        let (lo, hi) = match self {
            Unit::Minute => (1, 59),
            Unit::Hour => (1, 23),
            Unit::Day => (1, 30),
        };
        amount.clamp(lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_len_counts_chars_not_bytes() {
        assert_eq!(Session::note_len_of(""), 0);
        assert_eq!(Session::note_len_of("abc"), 3);
        assert_eq!(Session::note_len_of("番茄钟"), 3);
        assert_eq!(Session::note_len_of("😀"), 1);
    }

    #[test]
    fn unit_seconds_conversion() {
        assert_eq!(Unit::Minute.seconds(25), 1500);
        assert_eq!(Unit::Hour.seconds(2), 7200);
        assert_eq!(Unit::Day.seconds(1), 86400);
    }

    #[test]
    fn unit_clamp_domains() {
        assert_eq!(Unit::Minute.clamp_amount(0), 1);
        assert_eq!(Unit::Minute.clamp_amount(60), 59);
        assert_eq!(Unit::Hour.clamp_amount(24), 23);
        assert_eq!(Unit::Day.clamp_amount(31), 30);
        assert_eq!(Unit::Day.clamp_amount(1), 1);
    }
}
