# Windows 桌面番茄钟 — 设计规格

- 日期：2026-09-06
- 状态：已获用户逐节批准（产品设计 4 节 + 架构 1 节）
- 技术栈：Rust + Tauri v2 + TypeScript/Svelte + rusqlite(SQLite)
- 原型存档：`.superpowers/brainstorm/622-1788665732/content/`

## 1. 产品概述

一款 Windows 桌面常驻番茄钟：悬浮圆环显示倒计时，点击展开主面板即可「选时长 + 写备注 + 开始专注」。结束后记录进本地 SQLite，提供 KPI、7 天柱状图、当日时间轴与历史列表。追求轻量（安装包 ~10MB）、不打扰、数据完全本地。

## 2. 功能规格

### 2.1 悬浮圆环（RingWindow）

- 运行时常驻桌面的圆形进度环：直径 64px，悬停放大至 88px
- 环内显示剩余时间（`mm:ss`；超过 1 小时显示 `h:mm:ss`）与任务名（超长省略）
- 进度弧随剩余时间减少；透明无边框窗口，始终置顶
- 可拖拽移动；拖到屏幕边缘自动停靠并降为 20% 不透明度
- 单击展开主面板；计时完成时圆环变绿色脉冲闪烁
- 无任务运行时显示半透明「+」待启动态（不隐藏）

### 2.2 主面板（PanelWindow，单页滚动式）

自上而下：

1. **快速时长**：预设 chips `25分 / 45分 / 1时 / 2时 / 12时(半天) / 1天` + `＋自定义`（预设值即对应整数时长）
2. **自定义时长**：三档单位「分钟 / 小时 / 天」，整数选择器，单选单位；范围：分钟 1–59、小时 1–23、天 1–30；跨单位不叠加（一次任务只是一种单位 × 一个整数）
3. **任务备注**：多行文本框，占位提示「整理 Q3 复盘初稿…」；实时计数 `n / 500 字`，达到 500 字后计数变绿；软目标不强制截断；中文按 1 字符计
4. **开始专注**：主按钮，校验通过后收起面板、显示圆环
5. **今日时间轴**：当天已完成/进行中任务列表（任务名 + 时长条 + 分钟数），运行中任务置顶

### 2.3 计时控制

- 仅两种操作：**开始** 与 **放弃**；不支持暂停/继续，无自动休息循环
- 放弃需二次确认（防误触），记录 `status=aborted`

### 2.4 完成提醒（全套提醒）

- 圆环变绿脉冲闪烁（持续直到用户查看）
- Windows 系统通知：标题「🍅 番茄完成！」，正文含任务名、专注时长、备注字数
- 内置提示音（随应用分发的短 wav）
- 通知与声音可在设置中分别开关（默认开）

### 2.5 统计视图（面板内覆盖层，Esc/点击遮罩关闭）

1. **KPI 三卡**：今日专注小时、本周累计小时、累计完成番茄数
2. **最近 7 天柱状图**：每日专注分钟，SVG 自绘（不引入图表库）
3. **当日时间轴**：水平色带，默认视口 09:00–21:00，自动扩展以覆盖当天最早/最晚任务；每段颜色 = 一个任务，下方图例
4. **历史列表**：任务名 + 起止时间 + 时长 + 状态标签（完成/放弃/运行中），倒序，最近优先

### 2.6 设置

- 主题：跟随系统 / 深色 / 浅色（默认跟随系统）
- 提示音开关、系统通知开关
- 开机自启（可选，默认关）

## 3. 架构设计

```
┌────────────────────────────────────────────┐
│ 前端层  WebView2 · TypeScript + Svelte · Vite│
│  RingWindow(透明置顶) PanelWindow(单页)      │
│  StatsView(面板内覆盖层) · SVG 图表自绘      │
└──────────────┬─────────────────────────────┘
        Tauri IPC：invoke 命令 + emit 事件
┌──────────────┴─────────────────────────────┐
│ Rust 核心层  tauri v2 · tokio               │
│  timer    状态机 + 墙钟校准 + 每秒 tick 事件  │
│  store    rusqlite 连接/迁移/DAO (WAL)      │
│  notify   系统通知 + 提示音                 │
│  windows  窗口生命周期/拖拽/停靠/托盘        │
│  settings 主题/声音/通知/自启 (kv 表)        │
└──────────────┬─────────────────────────────┘
               ▼
   SQLite 单文件 %APPDATA%\Pomodoro\data.db
```

### 3.1 IPC 命令清单

| 命令 | 方向 | 说明 |
|------|------|------|
| `start_session(note, unit, amount)` | F→R | 创建 running 记录并启动计时 |
| `abort_session(id)` | F→R | 二次确认后置 aborted |
| `get_active_session()` | F→R | 启动时恢复运行中任务 |
| `list_sessions(range)` | F→R | 今日/本周/全部 |
| `get_stats()` | F→R | KPI + 7 天聚合 + 当日时间轴 |
| `get_settings` / `set_settings` | F→R | 读写设置 |
| 事件 `tick` | R→F | 每秒剩余秒数 |
| 事件 `session_done` | R→F | 完成通知触发 |

### 3.2 timer 模块（计时真相在 Rust 侧）

- 状态机：`Idle → Running → Completed | Aborted`（无暂停，故无中间态）
- 不用累加 tick 计时：以 `started_at + planned_sec` 为截止时间戳，每秒 tick 仅用于刷新 UI
- 系统休眠唤醒后用墙钟差值重算剩余时间，不丢进度
- 到点判定由 tokio 定时器 + tick 双重检查，触发 `Completed` → 存库 → `notify` → emit `session_done`

## 4. 数据模型

```sql
CREATE TABLE sessions (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  note        TEXT NOT NULL DEFAULT '',
  note_len    INTEGER NOT NULL DEFAULT 0,   -- 备注字数快照（目标500）
  planned_sec INTEGER NOT NULL,             -- 计划时长
  actual_sec  INTEGER,                      -- 实际用时（完成时=planned，放弃时=已用）
  started_at  TEXT NOT NULL,                -- ISO8601 含本地时区偏移
  ended_at    TEXT,
  status      TEXT NOT NULL CHECK(status IN ('running','completed','aborted'))
);
CREATE INDEX idx_sessions_started ON sessions(started_at);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL            -- JSON 编码值
);
```

- 数据库文件：`%APPDATA%\Pomodoro\data.db`，PRAGMA journal_mode=WAL
- 迁移：内嵌版本号 + `migrations` 数组，启动时顺序执行
- 字数规则：`note_len = note.chars().count()`（中文 1 字计 1）

## 5. 关键技术决策

| 决策 | 理由 |
|------|------|
| Tauri v2 而非 Qt/egui | 安装包 ~10MB；Win10/11 自带 WebView2；UI 表现力满足圆环/图表/时间轴 |
| 计时真相在 Rust | 前端只是显示层；改系统时间、休眠、崩溃都不漂移 |
| SVG 自绘图表 | 统计图简单（柱状+色带），不值得引入 Chart.js 的体积 |
| 单 sessions 表 | 任务=会话一一对应，无暂停无多任务并行，无需拆表 |
| Svelte 而非 React | 产物小、模板直白，适合 3 个小窗口的规模 |

## 6. 错误处理

| 场景 | 处理 |
|------|------|
| 系统休眠/唤醒 | 唤醒事件后按截止时间戳重算剩余；已过期立即判定完成 |
| 应用崩溃重启 | 启动扫描 `running` 记录：`started_at+planned_sec` 已过→标 completed 并补提醒；未过→恢复计时 |
| SQLite 打不开/损坏 | 备份损坏文件后重建；UI 顶栏黄条提示「数据已重置」；不影响当次计时 |
| 备注超长 | 软上限 500 字目标不截断；硬上限 5000 字防滥用，超出提示 |
| 非法时长输入 | 输入控件钳制在合法整数域，提交前再校验一次 |
| WebView2 缺失 | 安装器内置 Evergreen Bootstrapper 引导安装 |

## 7. 测试策略

- **Rust 单元测试**：timer 状态机全转移路径；休眠重算；`note_len` 字数；DAO CRUD 与迁移升级
- **前端测试**：时长钳制逻辑、备注计数组件
- **手动验收清单**：圆环拖拽/停靠、通知与声音开关、7 天图表正确性、崩溃恢复、开机自启
- 命令：`cargo test`（Rust 侧）+ `npm run test`（前端侧）

## 8. 范围外（YAGNI，明确不做）

暂停/继续、自动休息循环、多任务并行、云同步、账号系统、番茄统计导出、国际化（仅中文）、自动更新。

## 9. 里程碑（粗粒度）

1. 骨架：Tauri 工程 + 双窗口 + SQLite 迁移跑通
2. 核心闭环：定时 + 备注 + 圆环倒计时 + 完成提醒
3. 统计：KPI/柱状图/时间轴/历史列表
4. 打磨：停靠半透明、主题三档、崩溃恢复、安装包
