use crate::store::Store;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String, // system | dark | light
    pub sound: bool,
    pub notification: bool,
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "system".into(), sound: true, notification: true, autostart: false }
    }
}

impl Store {
    pub fn get_settings(&self) -> rusqlite::Result<Settings> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |r| r.get(0))
            .optional()?;
        Ok(match raw {
            Some(s) => serde_json::from_str(&s).unwrap_or_default(),
            None => Settings::default(),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> rusqlite::Result<()> {
        let json = serde_json::to_string(s).expect("settings json");
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES('app', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![json],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_then_roundtrip() {
        let s = crate::store::Store::open_in_memory().unwrap();
        assert_eq!(s.get_settings().unwrap(), Settings::default());
        let custom = Settings { theme: "light".into(), sound: false, notification: true, autostart: true };
        s.save_settings(&custom).unwrap();
        assert_eq!(s.get_settings().unwrap(), custom);
        // 二次保存走 ON CONFLICT 分支
        s.save_settings(&Settings::default()).unwrap();
        assert_eq!(s.get_settings().unwrap(), Settings::default());
    }

    #[test]
    fn corrupt_json_falls_back_to_default() {
        let s = crate::store::Store::open_in_memory().unwrap();
        s.conn.execute(
            "INSERT INTO settings(key, value) VALUES('app', '{bad json')",
            [],
        ).unwrap();
        assert_eq!(s.get_settings().unwrap(), Settings::default());
    }
}
