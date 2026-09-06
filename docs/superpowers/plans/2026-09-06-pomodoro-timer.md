# 番茄钟（Windows 桌面）实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 按规格 `docs/superpowers/specs/2026-09-06-pomodoro-timer-design.md` 实现 Rust + Tauri v2 桌面番茄钟：悬浮圆环倒计时 + 单页滚动主面板（时长 chips/自定义整数 + 500 字目标备注 + 今日时间轴）+ 统计视图（KPI/7 天柱状图/时间轴/历史）+ 全套完成提醒 + 本地 SQLite。

**架构：** 双 WebviewWindow（`ring` 透明置顶悬浮环 / `panel` 主面板）共用一套 Svelte 前端，按窗口 label 渲染不同组件。计时真相在 Rust 侧（`started_at + planned_sec` 截止时间戳 + 墙钟比对），每秒 emit `tick`；rusqlite WAL 单文件库存 `sessions`/`settings`。

**技术栈：** Tauri v2 · Rust（chrono/rusqlite-bundled/tokio-time/tauri-plugin-notification/tauri-plugin-autostart）· Svelte 5 + TS + Vite · Vitest（前端逻辑）· cargo test（Rust 逻辑）。

**前置环境（执行前检查一次）：** Rust `msvc` 工具链（`rustup default stable-x86_64-pc-windows-msvc`）、VS Build Tools（C++ 桌面负载，rusqlite bundled 需要 cc）、Node ≥ 20、WebView2 Runtime（Win10/11 通常自带）。

**约定：** 所有命令在 `E:\openCode-code\番茄钟` 下执行；应用代码位于子目录 `pomodoro/`。每个任务的 TDD 步骤即"红→绿"证据链，commit message 中文。

---

## 文件结构

```
pomodoro/                          # 应用根（create-tauri-app svelte-ts 模板生成后改造）
├── package.json                   # 加 vitest、@tauri-apps/api、plugin 包、test 脚本
├── vite.config.ts                 # 模板自带
├── public/ding.wav                # 完成提示音（复制系统音效）
├── index.html
├── src/
│   ├── main.ts                    # 挂载 App
│   ├── app.css                    # 主题 CSS 变量（system/dark/light）
│   ├── App.svelte                 # 按窗口 label 分发 Ring / Panel
│   └── lib/
│       ├── ipc.ts                 # invoke 封装 + DTO 类型（唯一 IPC 入口）
│       ├── panel.ts               # 时长钳制/预设/秒数换算（纯逻辑，vitest）
│       ├── panel.test.ts          # panel.ts 测试
│       ├── noteCounter.ts         # 字数统计（纯逻辑，vitest）
│       ├── noteCounter.test.ts    # 字数统计测试
│       ├── theme.ts               # 应用主题 + matchMedia 跟随
│       ├── Ring.svelte            # 悬浮圆环（tick/完成脉冲/点击/拖拽停靠）
│       ├── Panel.svelte           # 主面板（chips/自定义/备注/开始/时间轴/设置区）
│       └── Stats.svelte           # 统计覆盖层（KPI/柱状图/时间轴/历史）
└── src-tauri/
    ├── Cargo.toml                 # 依赖（rusqlite/chrono/tokio/notification/autostart）
    ├── tauri.conf.json            # 双窗口 + nsis bundle
    ├── capabilities/default.json  # 权限（window show/hide/position、notification、autostart）
    └── src/
        ├── main.rs                # 调 pomodoro_lib::run()
        ├── lib.rs                 # Builder 装配 + AppState + 崩溃恢复 setup
        ├── models.rs              # Session / Unit（clamp、seconds、字数）
        ├── store.rs               # SQLite 迁移 + sessions DAO（TDD）
        ├── settings.rs            # settings kv DAO（TDD）
        ├── timer.rs               # 纯状态机 make_timer/remaining_sec/poll（TDD）
        ├── commands.rs            # IPC 命令 + tick 循环
        └── notify.rs              # 系统通知（插件 Rust API）
```

**设计边界：** `store`/`settings`/`timer` 不依赖 tauri 类型，纯 Rust 可测；`commands` 是唯一胶水层；前端只经 `ipc.ts` 访问后端。

---

### 任务 1：工程脚手架 + 双窗口 + 依赖打通

**文件：** 创建 `pomodoro/`（模板）、修改 `pomodoro/src-tauri/tauri.conf.json`、`pomodoro/src-tauri/Cargo.toml`、`pomodoro/src-tauri/capabilities/default.json`、`pomodoro/package.json`、`pomodoro/src/App.svelte`、`pomodoro/src/app.css`、`pomodoro/src/main.ts`、`pomodoro/public/ding.wav`

- [ ] **步骤 1：环境检查**

```powershell
cargo --version; rustc --version; node --version
```

预期：三者版本号正常输出。任一缺失先安装再继续。

- [ ] **步骤 2：脚手架**

```powershell
cd E:\openCode-code\番茄钟
npm create tauri-app@latest pomodoro -- --template svelte-ts --manager npm --yes
cd pomodoro; npm install
```

- [ ] **步骤 3：改写 `pomodoro/src-tauri/tauri.conf.json`（整文件替换）**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "番茄钟",
  "version": "0.1.0",
  "identifier": "local.pomodoro.desktop",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "panel", "title": "番茄钟", "width": 340, "height": 640, "minWidth": 320, "minHeight": 480 },
      {
        "label": "ring", "width": 96, "height": 96, "transparent": true,
        "decorations": false, "alwaysOnTop": true, "skipTaskbar": true, "resizable": false
      }
    ],
    "security": { "csp": null }
  },
  "bundle": { "active": true, "targets": ["nsis"], "icon": ["icons/icon.ico"] }
}
```

- [ ] **步骤 4：`pomodoro/src-tauri/Cargo.toml` 的 `[dependencies]` 整段替换为**

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-notification = "2"
tauri-plugin-autostart = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
rusqlite = { version = "0.32", features = ["bundled"] }
tokio = { version = "1", features = ["time"] }
```

（保留模板已有的 `[build-dependencies]`、`[lib]` 段不动。）

- [ ] **步骤 5：改写 `pomodoro/src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["panel", "ring"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:window:allow-set-focus",
    "core:window:allow-outer-position",
    "core:window:allow-current-monitor",
    "notification:default",
    "autostart:default"
  ]
}
```

- [ ] **步骤 6：前端依赖 + 测试脚本。`pomodoro/package.json` scripts 中加入 `"test": "vitest run"`，然后：**

```powershell
cd pomodoro
npm i -D vitest
npm i @tauri-apps/api @tauri-apps/plugin-notification @tauri-apps/plugin-autostart
```

- [ ] **步骤 7：临时前端（双窗口分发验证）。`pomodoro/src/App.svelte` 整文件替换：**

```svelte
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  const label = getCurrentWindow().label;
  if (label === 'ring') document.body.classList.add('ring-body');
</script>

<main>
  <p>窗口：{label}</p>
</main>
```

`pomodoro/src/app.css` 追加到文件末尾：

```css
.ring-body { background: transparent !important; overflow: hidden; }
:root {
  --bg: #0f172a; --card: #1e293b; --border: #334155;
  --text: #e2e8f0; --muted: #64748b; --accent: #6366f1; --accent2: #22d3ee; --ok: #34d399;
}
body { margin: 0; font-family: 'Microsoft YaHei', system-ui, sans-serif; background: var(--bg); color: var(--text); }
```

- [ ] **步骤 8：提示音资产**

```powershell
Copy-Item 'C:\Windows\Media\Windows Notify System Generic.wav' 'E:\openCode-code\番茄钟\pomodoro\public\ding.wav'
```

- [ ] **步骤 9：构建验证（先 npm build 产出 dist，否则 cargo 宏展开会因缺 dist 失败）**

```powershell
cd pomodoro
npm run build
cd src-tauri; cargo check
```

预期：两者零 error。

- [ ] **步骤 10：Commit**

```powershell
cd E:\openCode-code\番茄钟
git add pomodoro .gitignore
git commit -m "feat: Tauri v2 脚手架，双窗口(ring/panel)+依赖+主题变量打通"
```

---

### 任务 2：models + store 迁移与基础 DAO（TDD）

**文件：** 创建 `pomodoro/src-tauri/src/models.rs`、`pomodoro/src-tauri/src/store.rs`；修改 `pomodoro/src-tauri/src/lib.rs`（挂模块）

- [ ] **步骤 1：在 `lib.rs` 顶部加入模块声明（此时 store.rs 尚不存在，制造红）**

```rust
pub mod models;
pub mod store;
```

- [ ] **步骤 2：运行验证失败（红）**

```powershell
cd pomodoro\src-tauri
cargo test
```

预期：编译错误 `file not found for module 'models'`。

- [ ] **步骤 3：创建 `models.rs`（完整文件）**

```rust
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
```

- [ ] **步骤 4：创建 `store.rs`（完整文件）**

```rust
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
```

- [ ] **步骤 5：运行验证通过（绿）**

```powershell
cargo test
```

预期：`note_len_counts_chars_not_bytes`、`unit_seconds_conversion`、`unit_clamp_domains`、`migrations_are_idempotent`、`insert_and_get_roundtrip`、`get_active_returns_latest_running_only` 全 PASS。

- [ ] **步骤 6：Commit**

```powershell
git add pomodoro/src-tauri/src
git commit -m "feat: models(Unit/Session) + store 迁移与 insert/get/get_active DAO（红→绿）"
```

---

### 任务 3：store 会话流转与统计聚合 DAO（TDD）

**文件：** 修改 `pomodoro/src-tauri/src/store.rs`（追加方法与测试）

- [ ] **步骤 1：先在 `mod tests` 中追加失败测试（红）**

```rust
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
        assert_eq!(today[0].id, b.id); // 倒序
        let recent = s.list_recent(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, b.id);
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
```

- [ ] **步骤 2：运行验证失败（红）**

```powershell
cargo test
```

预期：编译错误 `no method named 'finish'` 等。

- [ ] **步骤 3：在 `impl Store` 内追加实现**

```rust
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
```

> 依赖 `started_at` 为本地时间 ISO8601（`Local::now().to_rfc3339()`，形如 `2026-09-06T14:02:00.123456+08:00`），`substr(1,10)` 即本地日期——见任务 6 的 `now_iso()`。

- [ ] **步骤 4：运行验证通过（绿）**

```powershell
cargo test
```

预期：本任务 4 个新测试 + 任务 2 全部 PASS。

- [ ] **步骤 5：Commit**

```powershell
git add pomodoro/src-tauri/src/store.rs
git commit -m "feat: store 完成/放弃/今日列表/最近列表/每日专注聚合 DAO（红→绿）"
```

---

### 任务 4：settings kv DAO（TDD）

**文件：** 创建 `pomodoro/src-tauri/src/settings.rs`；修改 `pomodoro/src-tauri/src/lib.rs`（挂模块）

- [ ] **步骤 1：`lib.rs` 加 `pub mod settings;`，创建 `settings.rs`（完整文件，测试与实现同文件）**

```rust
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
```

- [ ] **步骤 2：运行验证（绿）**

```powershell
cargo test
```

预期：`defaults_then_roundtrip`、`corrupt_json_falls_back_to_default` 通过。

- [ ] **步骤 3：Commit**

```powershell
git add pomodoro/src-tauri/src
git commit -m "feat: settings kv DAO，默认值(通知/声音开,自启关,跟随系统)（红→绿）"
```

---

### 任务 5：timer 纯状态机（TDD）

**文件：** 创建 `pomodoro/src-tauri/src/timer.rs`；修改 `pomodoro/src-tauri/src/lib.rs`（挂模块）

- [ ] **步骤 1：`lib.rs` 加 `pub mod timer;`，创建 `timer.rs`（完整文件）**

```rust
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
```

- [ ] **步骤 2：运行验证（绿）**

```powershell
cargo test
```

预期：timer 3 个测试通过。

- [ ] **步骤 3：Commit**

```powershell
git add pomodoro/src-tauri/src
git commit -m "feat: timer 纯状态机，墙钟截止时间戳防休眠漂移（红→绿）"
```

---

### 任务 6：IPC 命令 + tick 循环 + 系统通知（装配）

**文件：** 创建 `pomodoro/src-tauri/src/commands.rs`、`pomodoro/src-tauri/src/notify.rs`；修改 `pomodoro/src-tauri/src/lib.rs`（AppState/插件/命令注册）

- [ ] **步骤 1：创建 `notify.rs`（完整文件）**

```rust
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
```

- [ ] **步骤 2：创建 `commands.rs`（完整文件）**

```rust
use crate::models::{Session, Unit};
use crate::notify;
use crate::settings::Settings;
use crate::timer::{self, ActiveTimer, Tick};
use crate::AppState;
use chrono::{Datelike, Local};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

fn now_iso() -> String {
    Local::now().to_rfc3339()
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TickPayload {
    pub session_id: i64,
    pub remaining_sec: i64,
    pub planned_sec: i64,
    pub note_short: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DonePayload {
    pub session_id: i64,
    pub note: String,
    pub planned_sec: i64,
    pub note_len: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayFocus {
    pub day: String,
    pub focus_sec: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsPayload {
    pub today_sec: i64,
    pub week_sec: i64,
    pub completed_count: i64,
    pub by_day: Vec<DayFocus>,
}

/// 每秒 tick 循环：以 deadline 墙钟比对，休眠唤醒自动校准
pub fn spawn_tick(app: AppHandle, t: ActiveTimer) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let state = app.state::<AppState>();
            let now = Local::now();
            let current = state.timer.lock().unwrap().clone();
            let Some(active) = current else { break };
            if active.session_id != t.session_id {
                break; // 被新任务替换或已放弃
            }
            match timer::poll(&active, now) {
                Tick::Running(rem) => {
                    let note_short = state
                        .store
                        .lock()
                        .unwrap()
                        .get(active.session_id)
                        .ok()
                        .flatten()
                        .map(|s| s.note.chars().take(8).collect::<String>())
                        .unwrap_or_default();
                    let _ = app.emit(
                        "tick",
                        TickPayload {
                            session_id: active.session_id,
                            remaining_sec: rem,
                            planned_sec: active.planned_sec,
                            note_short,
                        },
                    );
                }
                Tick::Finished => {
                    let ended = now_iso();
                    let session = {
                        let store = state.store.lock().unwrap();
                        let _ = store.finish(active.session_id, "completed", active.planned_sec, &ended);
                        store.get(active.session_id).ok().flatten()
                    };
                    *state.timer.lock().unwrap() = None;
                    let payload = DonePayload {
                        session_id: active.session_id,
                        note: session.as_ref().map(|s| s.note.clone()).unwrap_or_default(),
                        planned_sec: active.planned_sec,
                        note_len: session.as_ref().map(|s| s.note_len).unwrap_or(0),
                    };
                    let settings = state.store.lock().unwrap().get_settings().unwrap_or_default();
                    if settings.notification {
                        notify::notify_done(&app, &payload.note, payload.planned_sec);
                    }
                    let _ = app.emit("session_done", payload);
                    break;
                }
            }
        }
    });
}

#[tauri::command]
pub fn start_session(
    app: AppHandle,
    state: State<AppState>,
    note: String,
    unit: Unit,
    amount: i64,
) -> Result<Session, String> {
    // 硬上限 5000 字，超出静默截断（规格 §6）
    let note: String = note.chars().take(5000).collect();
    let amount = unit.clamp_amount(amount);
    let planned = unit.seconds(amount);
    let started = now_iso();
    let session = state
        .store
        .lock()
        .unwrap()
        .insert_running(&note, planned, &started)
        .map_err(e)?;
    let t = timer::make_timer(session.id, planned, Local::now());
    *state.timer.lock().unwrap() = Some(t.clone());
    spawn_tick(app, t);
    Ok(session)
}

#[tauri::command]
pub fn abort_session(state: State<AppState>, id: i64) -> Result<Session, String> {
    let now = Local::now();
    let actual = {
        let guard = state.timer.lock().unwrap();
        match guard.as_ref().filter(|t| t.session_id == id) {
            Some(t) => (t.planned_sec - timer::remaining_sec(t, now)).max(0),
            None => 0,
        }
    };
    *state.timer.lock().unwrap() = None;
    let ended = now_iso();
    {
        let store = state.store.lock().unwrap();
        store.finish(id, "aborted", actual, &ended).map_err(e)?;
        store.get(id).map_err(e)?.ok_or_else(|| "session not found".into())
    }
}

#[tauri::command]
pub fn get_active_session(state: State<AppState>) -> Result<Option<Session>, String> {
    state.store.lock().unwrap().get_active().map_err(e)
}

#[tauri::command]
pub fn list_today(state: State<AppState>) -> Result<Vec<Session>, String> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    state.store.lock().unwrap().list_today(&today).map_err(e)
}

#[tauri::command]
pub fn list_recent(state: State<AppState>, limit: i64) -> Result<Vec<Session>, String> {
    state.store.lock().unwrap().list_recent(limit).map_err(e)
}

#[tauri::command]
pub fn get_stats(state: State<AppState>) -> Result<StatsPayload, String> {
    let today = Local::now();
    let today_str = today.format("%Y-%m-%d").to_string();
    let store = state.store.lock().unwrap();
    let mut by_day = Vec::new();
    for i in (0..7).rev() {
        let day = (today - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let focus_sec = store.day_focus(&day).map_err(e)?;
        by_day.push(DayFocus { day, focus_sec });
    }
    // 本周 = 本周一 00:00 起（NaiveDate 逐日累加）
    let cursor = today.date_naive() - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    let today_date = today.date_naive();
    let mut week_sec = 0i64;
    loop {
        week_sec += store.day_focus(&cursor.format("%Y-%m-%d").to_string()).map_err(e)?;
        if cursor == today_date {
            break;
        }
        cursor += chrono::Duration::days(1);
    }
    Ok(StatsPayload {
        today_sec: store.day_focus(&today_str).map_err(e)?,
        week_sec,
        completed_count: store.completed_count().map_err(e)?,
        by_day,
    })
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    state.store.lock().unwrap().get_settings().map_err(e)
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<Settings, String> {
    {
        let store = state.store.lock().unwrap();
        store.save_settings(&settings).map_err(e)?;
    }
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let _ = if settings.autostart { autolaunch.enable() } else { autolaunch.disable() };
    let _ = app.emit("theme_changed", settings.theme.clone());
    Ok(settings)
}
```

- [ ] **步骤 3：改写 `lib.rs`（完整文件）**

```rust
pub mod commands;
pub mod models;
pub mod notify;
pub mod settings;
pub mod store;
pub mod timer;

use commands::*;
use std::sync::Mutex;
use store::Store;

pub struct AppState {
    pub store: Mutex<Store>,
    pub timer: Mutex<Option<timer::ActiveTimer>>,
}

fn db_path() -> std::path::PathBuf {
    std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Pomodoro")
        .join("data.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::open(&db_path()).expect("init sqlite");
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            store: Mutex::new(store),
            timer: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            abort_session,
            get_active_session,
            list_today,
            list_recent,
            get_stats,
            get_settings,
            set_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **步骤 4：回归验证**

```powershell
cd pomodoro\src-tauri
cargo test
cargo check
```

预期：全部既有测试 PASS；check 零 error。

- [ ] **步骤 5：Commit**

```powershell
git add pomodoro/src-tauri/src
git commit -m "feat: IPC 命令层(start/abort/stats/settings)+秒级 tick 循环+系统通知，lib 装配"
```

---

### 任务 7：前端 Ring.svelte（圆环/完成脉冲/点击/拖拽停靠）

**文件：** 创建 `pomodoro/src/lib/ipc.ts`、`pomodoro/src/lib/Ring.svelte`、`pomodoro/src/lib/theme.ts`；修改 `pomodoro/src/App.svelte`、`pomodoro/src/app.css`

- [ ] **步骤 1：创建 `src/lib/ipc.ts`（完整文件，全项目唯一 IPC 入口）**

```ts
import { invoke } from '@tauri-apps/api/core';

export type Unit = 'minute' | 'hour' | 'day';

export interface SessionDto {
  id: number;
  note: string;
  noteLen: number;
  plannedSec: number;
  actualSec: number | null;
  startedAt: string;
  endedAt: string | null;
  status: 'running' | 'completed' | 'aborted';
}

export interface SettingsDto {
  theme: 'system' | 'dark' | 'light';
  sound: boolean;
  notification: boolean;
  autostart: boolean;
}

export interface StatsDto {
  todaySec: number;
  weekSec: number;
  completedCount: number;
  byDay: { day: string; focusSec: number }[];
}

export interface TickDto {
  sessionId: number;
  remainingSec: number;
  plannedSec: number;
  noteShort: string;
}

export interface DoneDto {
  sessionId: number;
  note: string;
  plannedSec: number;
  noteLen: number;
}

export const startSession = (note: string, unit: Unit, amount: number) =>
  invoke<SessionDto>('start_session', { note, unit, amount });
export const abortSession = (id: number) => invoke<SessionDto>('abort_session', { id });
export const getActiveSession = () => invoke<SessionDto | null>('get_active_session');
export const listToday = () => invoke<SessionDto[]>('list_today');
export const listRecent = (limit = 12) => invoke<SessionDto[]>('list_recent', { limit });
export const getStats = () => invoke<StatsDto>('get_stats');
export const getSettings = () => invoke<SettingsDto>('get_settings');
export const saveSettings = (settings: SettingsDto) =>
  invoke<SettingsDto>('set_settings', { settings });
```

- [ ] **步骤 2：创建 `src/lib/theme.ts`**

```ts
import { listen } from '@tauri-apps/api/event';
import type { SettingsDto } from './ipc';

export function applyTheme(theme: SettingsDto['theme']) {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}

export async function initTheme(base: SettingsDto['theme']) {
  applyTheme(base);
  await listen<string>('theme_changed', (e) => applyTheme(e.payload as SettingsDto['theme']));
  window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
    if (!document.documentElement.dataset.theme) applyTheme('system'); // 触发系统档重算
  });
}
```

`app.css` 追加浅色变量与窗口分发：

```css
[data-theme='light'] {
  --bg: #f8fafc; --card: #ffffff; --border: #e2e8f0;
  --text: #0f172a; --muted: #64748b;
}
```

- [ ] **步骤 3：创建 `src/lib/Ring.svelte`（完整文件）**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { getActiveSession, type TickDto } from './ipc';

  let remaining = 0;
  let planned = 0;
  let noteShort = '';
  let running = false;
  let done = false;
  let docked = false;
  const R = 40;
  const C = 2 * Math.PI * R;

  $: progress = planned > 0 ? remaining / planned : 0;
  $: timeText =
    remaining >= 3600
      ? `${Math.floor(remaining / 3600)}:${String(Math.floor((remaining % 3600) / 60)).padStart(2, '0')}:${String(remaining % 60).padStart(2, '0')}`
      : `${String(Math.floor(remaining / 60)).padStart(2, '0')}:${String(remaining % 60).padStart(2, '0')}`;

  function fmt(s: TickDto) {
    remaining = s.remainingSec;
    planned = s.plannedSec;
    noteShort = s.noteShort;
    running = true;
    done = false;
  }

  async function openPanel() {
    done = false;
    const panel = await WebviewWindow.getByLabel('panel');
    await panel?.show();
    await panel?.setFocus();
  }

  onMount(() => {
    const offs: (() => void)[] = [];
    (async () => {
      offs.push(
        await listen<TickDto>('tick', (e) => fmt(e.payload)),
        await listen('session_done', () => {
          running = false;
          done = true;
          remaining = 0;
          new Audio('/ding.wav').play().catch(() => {});
        })
      );
      const active = await getActiveSession();
      if (active) {
        running = true;
        planned = active.plannedSec;
        remaining = active.plannedSec;
        noteShort = active.note.slice(0, 8);
      }
      const win = getCurrentWindow();
      offs.push(
        await win.onMoved(async () => {
          const pos = await win.outerPosition();
          const mon = await currentMonitor();
          docked =
            !!mon &&
            (pos.x <= 8 ||
              pos.y <= 8 ||
              pos.x + 96 >= mon.size.width - 8 ||
              pos.y + 96 >= mon.size.height - 8);
        })
      );
    })();
    return () => offs.forEach((f) => f());
  });
</script>

<div
  class="ring"
  class:docked
  class:pulse={done}
  data-tauri-drag-region
  on:click={openPanel}
  role="button"
  tabindex="0"
>
  {#if running}
    <svg viewBox="0 0 96 96" width="96" height="96">
      <circle cx="48" cy="48" r={R} class="track" />
      <circle
        cx="48"
        cy="48"
        r={R}
        class="arc"
        stroke-dasharray={C}
        stroke-dashoffset={C * (1 - progress)}
      />
    </svg>
    <div class="center">
      <b>{timeText}</b>
      <i>{noteShort}</i>
    </div>
  {:else if done}
    <div class="center ok"><b>✔ 完成</b></div>
  {:else}
    <div class="center"><b>＋</b></div>
  {/if}
</div>

<style>
  .ring {
    width: 96px;
    height: 96px;
    border-radius: 50%;
    position: relative;
    cursor: pointer;
    transition: opacity 0.2s;
    background: transparent;
  }
  .ring.docked { opacity: 0.2; }
  .ring.docked:hover { opacity: 1; }
  .ring:hover { transform: scale(1.08); }
  .track { fill: rgba(15, 23, 42, 0.88); stroke: rgba(255, 255, 255, 0.1); stroke-width: 6; }
  .arc {
    fill: none;
    stroke: var(--accent);
    stroke-width: 6;
    stroke-linecap: round;
    transform: rotate(-90deg);
    transform-origin: center;
    transition: stroke-dashoffset 1s linear;
  }
  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  .center b { font: 700 15px/1 Consolas, monospace; color: var(--text); }
  .center i { font: 10px/1.4 'Microsoft YaHei', sans-serif; font-style: normal; color: var(--muted); max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ok b { color: var(--ok); }
  .pulse { animation: pulse 1.2s ease-out infinite; }
  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(52, 211, 153, 0.5); }
    100% { box-shadow: 0 0 0 18px rgba(52, 211, 153, 0); }
  }
</style>
```

- [ ] **步骤 4：改写 `src/App.svelte`**

```svelte
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Panel from './lib/Panel.svelte';
  import Ring from './lib/Ring.svelte';
  const label = getCurrentWindow().label;
  if (label === 'ring') document.body.classList.add('ring-body');
</script>

{#if label === 'ring'}
  <Ring />
{:else}
  <Panel />
{/if}
```

- [ ] **步骤 5：Panel 占位（下个任务实现完整版）。创建 `src/lib/Panel.svelte`**

```svelte
<main style="padding:16px">面板开发中</main>
```

- [ ] **步骤 6：构建 + 手动验证**

```powershell
cd pomodoro
npm run build; npm run tauri dev
```

手动验证清单：① 出现 panel 主窗与 96px 透明圆环窗（环内「＋」） ② 拖动圆环可移动，拖到屏幕边缘变 20% 透明、移开恢复 ③ 点击圆环 panel 获得焦点 ④ 圆环窗无背景色块。

- [ ] **步骤 7：Commit**

```powershell
git add pomodoro/src
git commit -m "feat: ipc.ts 统一入口 + Ring 悬浮圆环（tick/完成脉冲/拖拽边缘停靠半透明）"
```

---

### 任务 8：时长/字数纯逻辑（vitest TDD）+ Panel.svelte 主面板

**文件：** 创建 `pomodoro/src/lib/panel.ts`、`panel.test.ts`、`noteCounter.ts`、`noteCounter.test.ts`；改写 `pomodoro/src/lib/Panel.svelte`

- [ ] **步骤 1：写失败测试 `panel.test.ts`**

```ts
import { describe, expect, it } from 'vitest';
import { clampAmount, plannedSeconds, PRESETS } from './panel';

describe('clampAmount', () => {
  it('minute 1-59', () => {
    expect(clampAmount('minute', 0)).toBe(1);
    expect(clampAmount('minute', 60)).toBe(59);
    expect(clampAmount('minute', 25)).toBe(25);
  });
  it('hour 1-23', () => {
    expect(clampAmount('hour', 0)).toBe(1);
    expect(clampAmount('hour', 24)).toBe(23);
  });
  it('day 1-30', () => {
    expect(clampAmount('day', 31)).toBe(30);
    expect(clampAmount('day', 1)).toBe(1);
  });
});

describe('plannedSeconds', () => {
  it('converts unit×amount', () => {
    expect(plannedSeconds('minute', 25)).toBe(1500);
    expect(plannedSeconds('hour', 2)).toBe(7200);
    expect(plannedSeconds('day', 1)).toBe(86400);
  });
});

describe('PRESETS', () => {
  it('contains spec presets', () => {
    expect(PRESETS.map((p) => p.label)).toEqual(['25分', '45分', '1时', '2时', '12时', '1天']);
  });
});
```

`noteCounter.test.ts`：

```ts
import { describe, expect, it } from 'vitest';
import { HARD_LIMIT, noteLen } from './noteCounter';

describe('noteLen', () => {
  it('counts code points, 中文=1', () => {
    expect(noteLen('')).toBe(0);
    expect(noteLen('abc')).toBe(3);
    expect(noteLen('番茄钟')).toBe(3);
    expect(noteLen('😀')).toBe(1);
  });
  it('hard limit is 5000', () => {
    expect(HARD_LIMIT).toBe(5000);
  });
});
```

- [ ] **步骤 2：运行验证失败（红）**

```powershell
cd pomodoro
npm test
```

预期：FAIL，`Cannot find module './panel'`。

- [ ] **步骤 3：实现 `panel.ts`、`noteCounter.ts`**

```ts
// panel.ts
export const UNITS = ['minute', 'hour', 'day'] as const;
export type Unit = (typeof UNITS)[number];

export const LIMITS: Record<Unit, [number, number]> = {
  minute: [1, 59],
  hour: [1, 23],
  day: [1, 30],
};

export const UNIT_LABEL: Record<Unit, string> = { minute: '分钟', hour: '小时', day: '天' };

export const PRESETS: { label: string; unit: Unit; amount: number }[] = [
  { label: '25分', unit: 'minute', amount: 25 },
  { label: '45分', unit: 'minute', amount: 45 },
  { label: '1时', unit: 'hour', amount: 1 },
  { label: '2时', unit: 'hour', amount: 2 },
  { label: '12时', unit: 'hour', amount: 12 },
  { label: '1天', unit: 'day', amount: 1 },
];

const SEC: Record<Unit, number> = { minute: 60, hour: 3600, day: 86400 };

export function clampAmount(unit: Unit, n: number): number {
  const [lo, hi] = LIMITS[unit];
  return Math.min(hi, Math.max(lo, Math.round(n)));
}

export function plannedSeconds(unit: Unit, amount: number): number {
  return amount * SEC[unit];
}
```

```ts
// noteCounter.ts
export const GOAL = 500;
export const HARD_LIMIT = 5000;

export function noteLen(s: string): number {
  return [...s].length;
}
```

- [ ] **步骤 4：运行验证通过（绿）**

```powershell
npm test
```

预期：全部 PASS。

- [ ] **步骤 5：改写 `Panel.svelte`（完整文件）**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Stats from './Stats.svelte';
  import { clampAmount, LIMITS, plannedSeconds, PRESETS, UNIT_LABEL, UNITS, type Unit } from './panel';
  import { GOAL, HARD_LIMIT, noteLen } from './noteCounter';
  import {
    abortSession,
    getActiveSession,
    getSettings,
    listToday,
    saveSettings,
    startSession,
    type SessionDto,
    type SettingsDto,
  } from './ipc';
  import { initTheme } from './theme';

  let unit: Unit = 'minute';
  let amount = 25;
  let custom = false;
  let note = '';
  let active: SessionDto | null = null;
  let activeRemaining = 0;
  let today: SessionDto[] = [];
  let settings: SettingsDto | null = null;
  let statsOpen = false;
  let settingsOpen = false;

  $: chars = noteLen(note);
  $: goalMet = chars >= GOAL;
  $: overHard = chars > HARD_LIMIT;

  async function refresh() {
    active = await getActiveSession();
    if (active) activeRemaining = active.plannedSec;
    today = await listToday();
  }

  function pickPreset(p: (typeof PRESETS)[number]) {
    unit = p.unit;
    amount = p.amount;
    custom = false;
  }

  function setUnit(u: Unit) {
    unit = u;
    amount = clampAmount(u, amount);
  }

  function changeAmount(delta: number) {
    amount = clampAmount(unit, amount + delta);
  }

  async function start() {
    active = await startSession(note.trim().slice(0, HARD_LIMIT), unit, amount);
    activeRemaining = active.plannedSec;
    note = '';
    today = await listToday();
  }

  async function giveUp() {
    if (!active) return;
    if (!confirm('确定放弃当前番茄？')) return;
    active = await abortSession(active.id);
    await refresh();
  }

  async function toggleSound() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, sound: !settings.sound });
  }
  async function toggleNotify() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, notification: !settings.notification });
  }
  async function setAutostart() {
    if (!settings) return;
    settings = await saveSettings({ ...settings, autostart: !settings.autostart });
  }
  async function setTheme(e: Event) {
    if (!settings) return;
    const theme = (e.target as HTMLSelectElement).value as SettingsDto['theme'];
    settings = await saveSettings({ ...settings, theme });
  }

  function barWidth(s: SessionDto): string {
    const px = Math.min(120, Math.max(24, s.plannedSec / 60));
    return `${px}px`;
  }

  function timeStr(sec: number): string {
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}小时${m}分` : `${m}分钟`;
  }

  onMount(() => {
    let off: (() => void) | undefined;
    (async () => {
      settings = await getSettings();
      await initTheme(settings.theme);
      await refresh();
      off = await listen('tick', (e: CustomEvent<{ remainingSec: number; sessionId: number }>) => {
        if (active && e.detail.sessionId === active.id) activeRemaining = e.detail.remainingSec;
      });
      await listen('session_done', refresh);
    })();
    return () => off?.();
  });
</script>

<svelte:window on:keydown={(e) => e.key === 'Escape' && (statsOpen = false)} />

<main>
  <header>
    <b>番茄钟</b>
    <span class="spacer" />
    <button class="icon" title="统计" on:click={() => (statsOpen = true)}>📊</button>
    <button class="icon" title="设置" on:click={() => (settingsOpen = !settingsOpen)}>⚙</button>
  </header>

  {#if active}
    <section class="running">
      <div>
        <b>{active.note || '（无备注）'}</b>
        <i>剩余 {timeStr(activeRemaining)}</i>
      </div>
      <button class="danger" on:click={giveUp}>放弃</button>
    </section>
  {/if}

  <section>
    <p class="lbl">快速时长</p>
    <div class="chips">
      {#each PRESETS as p}
        <button class="chip" class:on={!custom && unit === p.unit && amount === p.amount} on:click={() => pickPreset(p)}>
          {p.label}
        </button>
      {/each}
      <button class="chip" class:on={custom} on:click={() => (custom = true)}>＋自定义</button>
    </div>

    {#if custom}
      <div class="units">
        {#each UNITS as u}
          <button class="unit" class:on={unit === u} on:click={() => setUnit(u)}>
            <b>{amount}</b><span>{UNIT_LABEL[u]}</span>
          </button>
        {/each}
      </div>
      <div class="stepper">
        <button on:click={() => changeAmount(-1)}>−</button>
        <b>{amount} {UNIT_LABEL[unit]}</b>
        <button on:click={() => changeAmount(1)}>＋</button>
      </div>
      <p class="hint">范围：分钟 1–59 · 小时 1–23 · 天 1–30（整数）</p>
    {/if}
  </section>

  <section>
    <p class="lbl">任务备注 · 目标 {GOAL} 字</p>
    <textarea
      rows="4"
      maxlength={HARD_LIMIT}
      placeholder="整理 Q3 复盘初稿…"
      bind:value={note}
    />
    <p class="cnt" class:goalMet>{chars} / {GOAL} 字{overHard ? '（已截断）' : ''}</p>
    <button class="go" disabled={!!active} on:click={start}>
      {active ? '番茄进行中…' : '▶ 开始专注'}
    </button>
  </section>

  <section>
    <p class="lbl">今日时间轴</p>
    {#if today.length === 0}
      <p class="empty">今天还没有记录</p>
    {:else}
      {#each today as s}
        <div class="tl">
          <span class="nm">{s.note || '（无备注）'}</span>
          <span class="bar" style="width:{barWidth(s)};opacity:{s.status === 'completed' ? 1 : 0.5}" />
          <span class="tm">{timeStr(s.actualSec ?? s.plannedSec)}</span>
        </div>
      {/each}
    {/if}
  </section>

  {#if settingsOpen && settings}
    <section class="settings">
      <p class="lbl">设置</p>
      <div class="row"><span>主题</span>
        <select value={settings.theme} on:change={setTheme}>
          <option value="system">跟随系统</option>
          <option value="dark">深色</option>
          <option value="light">浅色</option>
        </select>
      </div>
      <div class="row"><span>提示音</span><input type="checkbox" checked={settings.sound} on:change={toggleSound} /></div>
      <div class="row"><span>系统通知</span><input type="checkbox" checked={settings.notification} on:change={toggleNotify} /></div>
      <div class="row"><span>开机自启</span><input type="checkbox" checked={settings.autostart} on:change={setAutostart} /></div>
    </section>
  {/if}

  {#if statsOpen}
    <div class="overlay" role="presentation" on:click={() => (statsOpen = false)}>
      <div class="sheet" role="dialog" on:click|stopPropagation>
        <Stats />
      </div>
    </div>
  {/if}
</main>

<style>
  main { padding: 12px 14px 20px; max-width: 340px; margin: 0 auto; }
  header { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  header b { color: var(--text); font-size: 14px; }
  .spacer { flex: 1; }
  .icon { background: var(--card); border: 1px solid var(--border); color: var(--text); border-radius: 8px; padding: 4px 8px; cursor: pointer; }
  .running { display: flex; align-items: center; justify-content: space-between; background: rgba(99, 102, 241, 0.15); border: 1px solid var(--accent); border-radius: 10px; padding: 10px 12px; margin-bottom: 12px; }
  .running i { display: block; font-style: normal; color: var(--muted); font-size: 11px; margin-top: 2px; }
  .danger { background: rgba(248, 113, 113, 0.15); color: #f87171; border: 1px solid rgba(248, 113, 113, 0.4); border-radius: 8px; padding: 6px 12px; cursor: pointer; }
  section { margin-bottom: 14px; }
  .lbl { font-size: 10px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.5px; margin: 0 0 6px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { background: var(--card); border: 1px solid var(--border); color: var(--muted); border-radius: 999px; font-size: 12px; padding: 5px 12px; cursor: pointer; }
  .chip.on { border-color: var(--accent); color: var(--text); background: rgba(99, 102, 241, 0.15); }
  .units { display: flex; gap: 8px; margin: 10px 0; }
  .unit { flex: 1; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 8px 4px; text-align: center; cursor: pointer; color: var(--muted); }
  .unit.on { border-color: var(--accent); background: rgba(99, 102, 241, 0.15); color: var(--text); }
  .unit b { display: block; font: 700 18px/1.2 Consolas, monospace; }
  .stepper { display: flex; align-items: center; justify-content: space-between; background: var(--card); border: 1px solid var(--border); border-radius: 8px; padding: 4px 8px; }
  .stepper button { background: none; border: none; color: var(--accent); font-size: 16px; cursor: pointer; }
  .hint { font-size: 10px; color: var(--muted); margin: 6px 0 0; }
  textarea { width: 100%; box-sizing: border-box; background: var(--card); border: 1px solid var(--border); border-radius: 8px; color: var(--text); font: 13px/1.6 'Microsoft YaHei', sans-serif; padding: 8px; resize: vertical; }
  .cnt { font-size: 11px; color: var(--muted); text-align: right; margin: 4px 0 10px; }
  .cnt.goalMet { color: var(--ok); }
  .go { width: 100%; background: linear-gradient(90deg, #6366f1, #8b5cf6); color: #fff; border: none; border-radius: 8px; padding: 11px; font-size: 14px; font-weight: 600; cursor: pointer; }
  .go:disabled { opacity: 0.5; cursor: not-allowed; }
  .empty { color: var(--muted); font-size: 12px; }
  .tl { display: flex; align-items: center; gap: 8px; padding: 6px 0; border-bottom: 1px dashed var(--border); }
  .tl .nm { font-size: 12px; color: var(--text); width: 96px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tl .bar { height: 6px; border-radius: 3px; background: linear-gradient(90deg, var(--accent), var(--accent2)); }
  .tl .tm { font-size: 10px; color: var(--muted); margin-left: auto; font-family: Consolas, monospace; }
  .settings { background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 10px 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; font-size: 13px; }
  .overlay { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.55); display: flex; align-items: center; justify-content: center; z-index: 10; }
  .sheet { background: var(--bg); border: 1px solid var(--border); border-radius: 12px; width: 320px; max-height: 90vh; overflow: auto; padding: 6px; }
</style>
```

- [ ] **步骤 6：类型检查 + 构建**

```powershell
npm run build
```

预期：svelte-check/vite 构建零 error（Stats 未创建前先建占位 `Stats.svelte`：`<div>统计开发中</div>`，任务 9 实现完整版）。

- [ ] **步骤 7：手动验证（`npm run tauri dev`）**

① 选 45 分 + 输入备注 → 字数实时计数，≥500 变绿 ② 自定义：切单位后数值钳制进合法域 ③ 开始后按钮禁用、运行卡显示剩余 ④ 放弃弹确认，时间轴出现「放弃」记录 ⑤ chips 高亮正确。

- [ ] **步骤 8：Commit**

```powershell
git add pomodoro/src
git commit -m "feat: 主面板（预设chips/自定义整数钳制/500字目标备注/开始/今日时间轴/设置区），vitest 红→绿"
```

---

### 任务 9：Stats.svelte 统计视图

**文件：** 改写 `pomodoro/src/lib/Stats.svelte`（完整文件）

- [ ] **步骤 1：实现（替换任务 8 的占位）**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, listRecent, type SessionDto, type StatsDto } from './ipc';

  let stats: StatsDto | null = null;
  let recent: SessionDto[] = [];

  $: maxFocus = Math.max(1, ...(stats?.byDay.map((d) => d.focusSec) ?? [1]));
  const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
  const COLORS = ['#6366f1', '#22d3ee', '#8b5cf6', '#34d399', '#f472b6', '#fbbf24', '#f87171'];

  function dayLabel(day: string): string {
    const d = new Date(`${day}T12:00:00`);
    return WEEK[d.getDay()];
  }

  function hours(sec: number): string {
    return (sec / 3600).toFixed(1);
  }

  function hhmm(iso: string): string {
    return new Date(iso).toTimeString().slice(0, 5);
  }

  function dur(s: SessionDto): string {
    const sec = s.actualSec ?? s.plannedSec;
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}小时${m}分` : `${m}分钟`;
  }

  interface Seg { startPct: number; widthPct: number; color: string; name: string }

  // 时间轴：默认 09:00–21:00，自动扩展覆盖当天最早/最晚任务
  $: segments = buildSegments(recent.filter((s) => s.startedAt.slice(0, 10) === todayStr()));

  function todayStr(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  }

  function buildSegments(todays: SessionDto[]): Seg[] {
    if (todays.length === 0) return [];
    const mins = (iso: string) => {
      const d = new Date(iso);
      return d.getHours() * 60 + d.getMinutes();
    };
    let lo = 9 * 60;
    let hi = 21 * 60;
    for (const s of todays) {
      lo = Math.min(lo, mins(s.startedAt));
      const end = mins(s.endedAt ?? new Date().toISOString());
      hi = Math.max(hi, end);
    }
    const span = Math.max(1, hi - lo);
    return todays.map((s, i) => ({
      startPct: ((mins(s.startedAt) - lo) / span) * 100,
      widthPct: Math.max(1.5, (((mins(s.endedAt ?? new Date().toISOString())) - mins(s.startedAt)) / span) * 100),
      color: COLORS[i % COLORS.length],
      name: `${(s.note || '（无备注）').slice(0, 10)} ${dur(s)}`,
    }));
  }

  function statusLabel(s: SessionDto['status']): string {
    return { running: '运行', completed: '完成', aborted: '放弃' }[s];
  }

  onMount(async () => {
    stats = await getStats();
    recent = await listRecent(12);
  });
</script>

<div class="stats">
  <h3>📊 专注统计</h3>

  {#if stats}
    <div class="kpis">
      <div class="kpi"><b>{hours(stats.todaySec)}</b><i>今日 · 小时</i></div>
      <div class="kpi"><b>{hours(stats.weekSec)}</b><i>本周 · 小时</i></div>
      <div class="kpi"><b>{stats.completedCount}</b><i>完成番茄 · 个</i></div>
    </div>

    <p class="lbl">最近 7 天 · 每日专注小时</p>
    <div class="bars">
      {#each stats.byDay as d}
        <div class="col">
          <div class="bar" style="height:{Math.round((d.focusSec / maxFocus) * 100)}%" />
          <em>{dayLabel(d.day)}</em>
        </div>
      {/each}
    </div>

    <p class="lbl">时间轴 · 今天</p>
    <div class="axis">
      {#each segments as seg}
        <div
          class="seg"
          style="left:{seg.startPct}%;width:{seg.widthPct}%;background:{seg.color}"
          title={seg.name}
        />
      {/each}
    </div>
    <div class="legend">
      {#each segments as seg}
        <span><i style="background:{seg.color}" />{seg.name}</span>
      {/each}
    </div>
  {/if}

  <p class="lbl">历史记录 · 最近</p>
  <table>
    {#each recent as s}
      <tr>
        <td class="nm">{s.note || '（无备注）'}</td>
        <td class="mono">{hhmm(s.startedAt)}–{s.endedAt ? hhmm(s.endedAt) : '…'}</td>
        <td class="mono">{dur(s)}</td>
        <td><span class="tag {s.status}">{statusLabel(s.status)}</span></td>
      </tr>
    {/each}
  </table>
</div>

<style>
  .stats { padding: 8px 6px; font-size: 12px; }
  h3 { margin: 4px 0 10px; font-size: 14px; color: var(--text); }
  .kpis { display: flex; gap: 8px; }
  .kpi { flex: 1; background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 10px; text-align: center; }
  .kpi b { display: block; font: 700 18px/1.2 Consolas, monospace; color: var(--accent2); }
  .kpi:nth-child(2) b { color: #818cf8; }
  .kpi:nth-child(3) b { color: var(--ok); }
  .kpi i { font-style: normal; font-size: 10px; color: var(--muted); }
  .lbl { font-size: 10px; color: var(--muted); text-transform: uppercase; margin: 12px 0 6px; }
  .bars { display: flex; align-items: flex-end; gap: 8px; height: 88px; background: var(--card); border: 1px solid var(--border); border-radius: 10px; padding: 10px; }
  .col { flex: 1; display: flex; flex-direction: column; justify-content: flex-end; align-items: center; height: 100%; gap: 4px; }
  .bar { width: 100%; min-height: 2px; border-radius: 4px 4px 0 0; background: linear-gradient(180deg, var(--accent2), var(--accent)); }
  .col em { font-style: normal; font-size: 9px; color: var(--muted); }
  .axis { position: relative; height: 10px; border-radius: 5px; background: var(--card); overflow: hidden; }
  .seg { position: absolute; top: 0; height: 100%; }
  .legend { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 6px; font-size: 10px; color: var(--muted); }
  .legend i { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin-right: 4px; }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 6px 4px; border-bottom: 1px dashed var(--border); color: var(--text); }
  td.nm { max-width: 110px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  td.mono { font: 10px Consolas, monospace; color: var(--muted); }
  .tag { font-size: 9px; padding: 2px 8px; border-radius: 999px; }
  .tag.completed { background: rgba(52, 211, 153, 0.15); color: var(--ok); }
  .tag.aborted { background: rgba(248, 113, 113, 0.15); color: #f87171; }
  .tag.running { background: rgba(99, 102, 241, 0.2); color: #818cf8; }
</style>
```

- [ ] **步骤 2：构建 + 手动验证**

```powershell
npm run build; npm run tauri dev
```

手动验证：① 完成 2 个短任务（自定义 1 分钟 ×2，第二个放弃）后打开 📊：KPI 今日=完成的那个、柱状图今天有柱、时间轴有色段、历史含完成/放弃标签 ② Esc 遮罩点击可关闭（组件在遮罩 div 上，点遮罩即关） ③ 7 天柱与数据一致。

- [ ] **步骤 3：Commit**

```powershell
git add pomodoro/src
git commit -m "feat: 统计视图 KPI/7天柱状图/当日时间轴(自动扩展)/历史列表"
```

---

### 任务 10：崩溃恢复 + 数据库损坏降级

**文件：** 修改 `pomodoro/src-tauri/src/lib.rs`（setup 钩子 + db 损坏备份）

- [ ] **步骤 1：`lib.rs` 中给 `Store::open` 调用加损坏降级，并加 setup 钩子。`run()` 改为：**

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = open_db_with_recovery();
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState {
            store: Mutex::new(store),
            timer: Mutex::new(None),
        })
        .setup(|app| {
            recover_running_session(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_session,
            abort_session,
            get_active_session,
            list_today,
            list_recent,
            get_stats,
            get_settings,
            set_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 数据库打不开 → 备份损坏文件后重建（规格 §6）
fn open_db_with_recovery() -> Store {
    let path = db_path();
    match Store::open(&path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("db open failed: {err}; resetting");
            let _ = std::fs::rename(
                &path,
                path.with_extension(format!("db.bak-{}", chrono::Local::now().format("%Y%m%d%H%M%S"))),
            );
            Store::open(&path).expect("recreate sqlite")
        }
    }
}

/// 崩溃恢复：running 记录已过期→补完成并提醒；未过期→续跑（规格 §6）
fn recover_running_session(app: tauri::AppHandle) {
    use std::sync::Mutex;
    let state = app.state::<AppState>();
    let Some(s) = state.store.lock().unwrap().get_active().ok().flatten() else {
        return;
    };
    let started = chrono::DateTime::parse_from_rfc3339(&s.started_at)
        .map(|d| d.with_timezone(&chrono::Local))
        .unwrap_or_else(|_| chrono::Local::now());
    let t = timer::make_timer(s.id, s.planned_sec, started);
    match timer::poll(&t, chrono::Local::now()) {
        timer::Tick::Finished => {
            let ended = chrono::Local::now().to_rfc3339();
            let settings = state.store.lock().unwrap().get_settings().unwrap_or_default();
            let _ = state
                .store
                .lock()
                .unwrap()
                .finish(s.id, "completed", s.planned_sec, &ended);
            if settings.notification {
                notify::notify_done(&app, &s.note, s.planned_sec);
            }
        }
        timer::Tick::Running(_) => {
            *state.timer.lock().unwrap() = Some(t.clone());
            commands::spawn_tick(app, t);
        }
    }
}
```

- [ ] **步骤 2：回归**

```powershell
cd pomodoro\src-tauri
cargo test; cargo check
cd ..
npm test; npm run build
```

预期：全部 PASS / 零 error（timer::poll 的过期/续跑两条路径已在任务 5 单测覆盖）。

- [ ] **步骤 3：手动验证**

`npm run tauri dev` → 开始一个 2 分钟番茄 → 直接杀掉进程（任务栏右键关闭）→ 重新 `npm run tauri dev`：圆环恢复倒计时（未过期路径）。再起一个 1 分钟任务后杀进程，等 2 分钟重启：收到「🍅 番茄完成！」通知且历史中该条为「完成」（过期路径）。

- [ ] **步骤 4：Commit**

```powershell
git add pomodoro/src-tauri/src/lib.rs
git commit -m "feat: 崩溃恢复(过期补完成/未过期续跑)+SQLite损坏备份重建"
```

---

### 任务 11：打包 NSIS + 全量回归 + 手动验收

**文件：** 无新文件（收尾验证）

- [ ] **步骤 1：全量回归（红绿回归证据链 🔄）**

```powershell
cd pomodoro\src-tauri; cargo test
cd ..; npm test; npm run build
```

预期：全绿。任一失败回到对应任务修复后重跑。

- [ ] **步骤 2：打包**

```powershell
npm run tauri build
```

预期：产物 `pomodoro/src-tauri/target/release/bundle/nsis/番茄钟_0.1.0_x64-setup.exe`；安装包体积 < 15MB。

- [ ] **步骤 3：手动验收清单（对照规格 §2/§6）**

① 安装 exe 后双窗口正常 ② 悬浮圆环：置顶/拖拽/边缘停靠 20% 透明/点击展开 ③ 快速 chips + 自定义钳制（分钟 1-59/小时 1-23/天 1-30） ④ 备注 500 字目标计数变绿、5000 硬截断 ⑤ 开始/放弃（二次确认） ⑥ 完成：圆环变绿脉冲+系统通知+提示音；通知/声音开关生效 ⑦ 统计四区块数据正确 ⑧ 主题三档切换（含跟随系统） ⑨ 开机自启开关生效（任务管理器→启动） ⑩ 崩溃恢复两路径 ⑪ 重启后数据仍在（%APPDATA%\Pomodoro\data.db）。

- [ ] **步骤 4：Commit + tag**

```powershell
git add -A
git commit -m "chore: v0.1.0 打包与验收收尾"
git tag v0.1.0
```

---

## 规格覆盖映射（自检用）

| 规格章节 | 任务 |
|---|---|
| §2.1 悬浮圆环 | T1(窗口) T7(渲染/拖拽停靠/脉冲) |
| §2.2 主面板 | T8 |
| §2.3 仅开始/放弃 | T6(命令) T8(确认交互) |
| §2.4 全套提醒 | T6(Rust 通知) T7(脉冲+ding.wav) T4(开关默认开) T11(验收⑥) |
| §2.5 统计视图 | T3(DAO) T6(get_stats) T9(渲染) |
| §2.6 设置 | T4(DAO) T8(UI) T11(自启生效) |
| §3 架构/IPC | T1 T6 |
| §4 数据模型 | T2 T4 |
| §6 错误处理 | T5(休眠) T10(崩溃/损坏) T6(5000 截断) T2/T8(钳制) T1(WebView2 引导由 NSIS 自带) |
| §7 测试策略 | T2-T5(cargo) T8(vitest) T11(手动清单) |
