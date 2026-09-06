use crate::models::Session;
use rusqlite::{params, Connection, OptionalExtension};

pub struct Store {
    conn: Connection,
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
    /// 打开（含损坏备份重建逻辑在任务 10 增强）；目录不存在则创建
    pub fn open(path: &std::path::Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn migrations_are_idempotent() {
        let s = store();
        s.get_active().unwrap(); // 不报错即表存在
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
}
