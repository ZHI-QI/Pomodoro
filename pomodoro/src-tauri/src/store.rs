use crate::models::Session;
use rusqlite::{params, Connection, OptionalExtension};

pub struct Store {
    pub(crate) conn: Connection,
}

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE IF NOT EXISTS sessions (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        note        TEXT NOT NULL DEFAULT '',
        note_len    INTEGER NOT NULL DEFAULT 0,
        planned_sec INTEGER NOT NULL,
        actual_sec  INTEGER,
        started_at  TEXT NOT NULL,
        ended_at    TEXT,
        status      TEXT NOT NULL CHECK(status IN ('running','completed','aborted'))
    );
    CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at);
    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
];

/// Store::open 的打开结果：
/// - Opened   首次打开既有文件成功
/// - Recovered 文件原本不存在/为空 → 新建空库（首次启动或丢失）
/// - Reset    文件存在但 SQLite 拒绝 → 备份为 .corrupt-<ts> 后重建
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenOutcome {
    Opened,
    Recovered,
    Reset,
}

fn row_to_session(r: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: r.get(0)?,
        note: r.get(1)?,
        note_len: r.get(2)?,
        planned_sec: r.get(3)?,
        actual_sec: r.get(4)?,
        started_at: r.get(5)?,
        ended_at: r.get(6)?,
        status: r.get(7)?,
    })
}

impl Store {
    /// 打开数据库；损坏时备份原文件并重建。
    /// 返回 (Store, OpenOutcome) 以便上层 emit 对应事件给前端。
    pub fn open(path: &std::path::Path) -> rusqlite::Result<(Self, OpenOutcome)> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let existed_before = path.exists();
        match Self::try_open(path) {
            Ok(store) => Ok((
                store,
                if existed_before {
                    OpenOutcome::Opened
                } else {
                    OpenOutcome::Recovered
                },
            )),
            Err(err) => Self::reset_after_corruption(path, &err),
        }
    }

    fn try_open(path: &std::path::Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    /// 损坏处理：rename 原文件为 data.db.corrupt-<ts>，再以新文件名打开
    fn reset_after_corruption(
        path: &std::path::Path,
        err: &rusqlite::Error,
    ) -> rusqlite::Result<(Self, OpenOutcome)> {
        eprintln!("db open failed: {err}; backing up and resetting");
        let ts = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let backup_name = format!(
            "{}.corrupt-{ts}",
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("data.db")
        );
        let backup_path = path.with_file_name(backup_name);
        let _ = std::fs::rename(path, &backup_path);
        let store = Self::try_open(path)?;
        Ok((store, OpenOutcome::Reset))
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        for m in MIGRATIONS {
            conn.execute_batch(m)?;
        }
        Ok(Self { conn })
    }

    pub fn insert_running(
        &self,
        note: &str,
        planned_sec: i64,
        started_at: &str,
    ) -> rusqlite::Result<Session> {
        let note_len = Session::note_len_of(note);
        self.conn.execute(
            "INSERT INTO sessions(note, note_len, planned_sec, started_at, status)
             VALUES (?1, ?2, ?3, ?4, 'running')",
            params![note, note_len, planned_sec, started_at],
        )?;
        Ok(Session {
            id: self.conn.last_insert_rowid(),
            note: note.to_string(),
            note_len,
            planned_sec,
            actual_sec: None,
            started_at: started_at.to_string(),
            ended_at: None,
            status: "running".into(),
        })
    }

    pub fn get(&self, id: i64) -> rusqlite::Result<Option<Session>> {
        self.conn
            .query_row(
                "SELECT id, note, note_len, planned_sec, actual_sec, started_at, ended_at, status
                 FROM sessions WHERE id = ?1",
                params![id],
                row_to_session,
            )
            .optional()
    }

    /// 最新一条 running（崩溃恢复用）
    pub fn get_active(&self) -> rusqlite::Result<Option<Session>> {
        self.conn
            .query_row(
                "SELECT id, note, note_len, planned_sec, actual_sec, started_at, ended_at, status
                 FROM sessions WHERE status = 'running' ORDER BY id DESC LIMIT 1",
                [],
                row_to_session,
            )
            .optional()
    }

    pub fn finish(&self, id: i64, status: &str, actual_sec: i64, ended_at: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET status = ?2, actual_sec = ?3, ended_at = ?4 WHERE id = ?1",
            params![id, status, actual_sec, ended_at],
        )?;
        Ok(())
    }

    /// 某自然日（本地日期前缀）completed 的 actual_sec 合计
    pub fn day_focus(&self, day: &str) -> rusqlite::Result<i64> {
        self.conn.query_row(
            "SELECT COALESCE(SUM(actual_sec), 0) FROM sessions
             WHERE status = 'completed' AND substr(started_at, 1, 10) = ?1",
            params![day],
            |r| r.get(0),
        )
    }

    pub fn list_today(&self, today: &str) -> rusqlite::Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note, note_len, planned_sec, actual_sec, started_at, ended_at, status
             FROM sessions WHERE substr(started_at, 1, 10) = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map(params![today], row_to_session)?;
        rows.collect()
    }

    pub fn list_recent(&self, limit: i64) -> rusqlite::Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note, note_len, planned_sec, actual_sec, started_at, ended_at, status
             FROM sessions ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], row_to_session)?;
        rows.collect()
    }

    pub fn completed_count(&self) -> rusqlite::Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'completed'",
            [],
            |r| r.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    fn tmp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pomodoro-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn migrations_are_idempotent() {
        let s = store();
        s.get_active().unwrap();
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let s = store();
        let session = s.insert_running("写周报初稿", 2700, "2026-09-06T14:02:00+08:00").unwrap();
        assert_eq!(session.note_len, 5);
        assert_eq!(session.status, "running");
        assert_eq!(session.actual_sec, None);
        let loaded = s.get(session.id).unwrap().unwrap();
        assert_eq!(loaded, session);
        assert!(s.get(999).unwrap().is_none());
    }

    #[test]
    fn get_active_returns_latest_running_only() {
        let s = store();
        assert!(s.get_active().unwrap().is_none());
        let a = s.insert_running("任务A", 60, "2026-09-06T09:00:00+08:00").unwrap();
        assert_eq!(s.get_active().unwrap().unwrap().id, a.id);
    }

    #[test]
    fn finish_marks_completed_with_actual() {
        let s = store();
        let a = s.insert_running("任务", 60, "2026-09-06T09:00:00+08:00").unwrap();
        s.finish(a.id, "completed", 60, "2026-09-06T09:01:00+08:00").unwrap();
        let done = s.get(a.id).unwrap().unwrap();
        assert_eq!(done.status, "completed");
        assert_eq!(done.actual_sec, Some(60));
        assert!(done.ended_at.is_some());
        assert!(s.get_active().unwrap().is_none());
    }

    #[test]
    fn day_focus_counts_only_completed() {
        let s = store();
        let a = s.insert_running("完成", 1200, "2026-09-06T09:00:00+08:00").unwrap();
        s.finish(a.id, "completed", 1200, "2026-09-06T09:20:00+08:00").unwrap();
        let b = s.insert_running("放弃", 600, "2026-09-06T10:00:00+08:00").unwrap();
        s.finish(b.id, "aborted", 300, "2026-09-06T10:05:00+08:00").unwrap();
        assert_eq!(s.day_focus("2026-09-06").unwrap(), 1200);
        assert_eq!(s.day_focus("2026-09-05").unwrap(), 0);
    }

    #[test]
    fn list_today_and_recent_ordering() {
        let s = store();
        let a = s.insert_running("早", 60, "2026-09-06T09:00:00+08:00").unwrap();
        let b = s.insert_running("晚", 60, "2026-09-06T18:00:00+08:00").unwrap();
        let _other = s.insert_running("昨天", 60, "2026-09-05T18:00:00+08:00").unwrap();
        let today = s.list_today("2026-09-06").unwrap();
        assert_eq!(today.len(), 2);
        assert_eq!(today[0].id, b.id);
        let recent = s.list_recent(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, _other.id);
        assert_eq!(a.id, a.id);
    }

    #[test]
    fn completed_count_counts_all_time() {
        let s = store();
        let a = s.insert_running("x", 60, "2026-09-01T09:00:00+08:00").unwrap();
        s.finish(a.id, "completed", 60, "2026-09-01T09:01:00+08:00").unwrap();
        let b = s.insert_running("y", 60, "2026-09-06T09:00:00+08:00").unwrap();
        s.finish(b.id, "aborted", 10, "2026-09-06T09:01:00+08:00").unwrap();
        assert_eq!(s.completed_count().unwrap(), 1);
    }

    // ───── 任务 10：OpenOutcome 相关 ─────

    #[test]
    fn open_outcome_opened_when_file_exists_and_valid() {
        let path = tmp_path("open_ok.db");
        let _ = std::fs::remove_file(&path);
        // 第一次打开：文件不存在 → Recovered
        let (_s1, o1) = Store::open(&path).unwrap();
        assert_eq!(o1, OpenOutcome::Recovered);
        // 第二次打开：文件已存在且有效 → Opened
        let (_s2, o2) = Store::open(&path).unwrap();
        assert_eq!(o2, OpenOutcome::Opened);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_outcome_reset_when_file_is_corrupt() {
        let path = tmp_path("open_corrupt.db");
        let _ = std::fs::remove_file(&path);
        // 先正常打开，植入数据
        {
            let (s, o) = Store::open(&path).unwrap();
            assert_eq!(o, OpenOutcome::Recovered);
            s.insert_running("seed", 60, "2026-09-06T09:00:00+08:00").unwrap();
        }
        // 覆写为垃圾字节模拟 SQLite 损坏
        std::fs::write(&path, b"\0\0\0garbage not a sqlite db\0\0").unwrap();
        // 重新打开：触发备份 + 重建 → Reset
        let (s, o) = Store::open(&path).unwrap();
        assert_eq!(o, OpenOutcome::Reset);
        // 新库是空的，sessions 表存在但无数据
        assert!(s.get_active().unwrap().is_none());
        assert_eq!(s.completed_count().unwrap(), 0);
        // 备份文件存在且以 .corrupt- 开头
        let dir = path.parent().unwrap();
        let backups: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .contains("open_corrupt.db.corrupt-")
            })
            .collect();
        assert!(!backups.is_empty(), "应生成 corrupt 备份");
        // 清理
        for b in backups {
            let _ = std::fs::remove_file(b.path());
        }
        let _ = std::fs::remove_file(&path);
    }
}
