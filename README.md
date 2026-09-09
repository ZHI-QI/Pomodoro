<div align="center">

# 🍅 番茄钟 Pomodoro

**极光玻璃质感的 Windows 桌面番茄钟 · 极简 · 本地 · 免登录**

一个番茄，一次专注。到点自动息屏休息，数据全部留在你的电脑里。

[![Version](https://img.shields.io/badge/version-0.1.5-6366f1)](#-快速开始)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8D8?logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Svelte](https://img.shields.io/badge/Svelte-4-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev)
[![Platform](https://img.shields.io/badge/Windows-10%20%2F%2011-0078D6?logo=windows11&logoColor=white)](#-快速开始)
[![Installer](https://img.shields.io/badge/安装包-2.2MB-34d399)](#-快速开始)

</div>

---

## ✨ 为什么选择它

| | |
|---|---|
| 🪶 **轻若无物** | NSIS 安装包仅 **2.2 MB**，绿色版单文件 6 MB，内存占用极低，老电脑也秒开 |
| 🌌 **极光玻璃 UI** | 三团极光光晕缓慢流动，全局磨砂玻璃质感，深色 / 浅色 / 跟随系统三档自适应 |
| ⏱️ **专注即全屏** | 点击「开始专注」，面板丝滑切换为渐变圆环倒计时视图，一眼只看时间 |
| 🌙 **到点强制息屏** | 完成番茄自动关闭显示器 1 分钟——护眼休息，硬性执行，不靠自觉 |
| 📊 **专注看得见** | 今日 / 本周专注时长、7 天柱状图、今日时间轴、最近记录，一屏尽览 |
| 💾 **数据永不离身** | 本地 SQLite 存储，无需注册登录；崩溃自动恢复，数据库损坏自动备份重建 |
| ⚙️ **恰到好处的设置** | 提示音 / 系统通知 / 开机自启，一个不多，一个不少 |

## 🚀 快速开始

### 方式一 · 直接下载（推荐）

前往 [**Releases**](../../releases) 页面下载：

- `番茄钟_x.x.x_x64-setup.exe` —— 安装版（NSIS 向导）
- `pomodoro.exe` —— 绿色版，双击即用 *（需系统已装 WebView2，Win11 自带）*

### 方式二 · 从源码构建

```bash
# 前置：Node.js 18+ 与 Rust stable（msvc toolchain）
git clone <本仓库地址>
cd 番茄钟/pomodoro

npm install          # 安装前端依赖
npm run tauri dev    # 开发调试
npm run tauri build  # 产出安装包与绿色 exe
```

构建产物位于 `src-tauri/target/release/bundle/nsis/`。

## 🎯 使用一览

```
┌─────────────────────────┐
 │  🍅 番茄钟        📊 ⚙️ │
 │                         │
 │   [25分] [45分] [1时]   │  ← 快速时长，或自定义 分钟/小时/天
 │                         │
 │   任务备注…  12/20 字   │  ← 写下目标，更易进入状态
 │                         │
 │      ▶ 开始专注         │  ← 点击后面板切换为
 │                         │     渐变圆环全屏倒计时
 │   今日时间轴 ▓▓▓░░       │
 └─────────────────────────┘
        完成 → 🔔 提示音 + 系统通知
             → 🌙 息屏休息 1 分钟
```

- **时长预设**：25 / 45 分钟一键开跑，也支持自定义 1–59 分钟、1–23 小时、1–30 天
- **任务备注**：带目标字数进度提示，写清目标再开始
- **放弃保护**：误触「放弃」会弹出玻璃质感确认弹窗，防止手滑

## 🛠️ 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2（单 WebView，安装包 2.2 MB） |
| 后端 | Rust · `rusqlite`(WAL) · `chrono` · `tokio(time)` · `windows-rs`(息屏) · 通知 / 自启插件 |
| 前端 | Svelte 4 + TypeScript + Vite · 自绘极光玻璃主题 |
| 质量 | `cargo test`（27 项）· `vitest`（10 项）· `svelte-check` 0 警告 |

### 工程亮点

- **状态机驱动的计时核心**：`timer::poll` 每秒推进，完成 / 放弃 / 崩溃三态清晰
- **崩溃恢复**：应用意外退出后重启，未结束的番茄自动按策略补账，绝丢单
- **数据库自愈**：损坏时自动备份原库并重建，横幅提示用户
- **纯函数业务层**：时长钳制、横幅文案、字数统计均为纯函数，单测全覆盖

## 📁 目录结构

```
pomodoro/
├── src/                  # Svelte 前端
│   ├── lib/
│   │   ├── Panel.svelte  # 主面板（计时 / 统计 / 设置 / 确认弹窗）
│   │   ├── Stats.svelte  # 统计视图（KPI / 柱状图 / 时间轴）
│   │   ├── Icon.svelte   # 线性图标组件
│   │   └── *.ts          # 纯函数业务层 + IPC 封装
│   └── app.css           # 极光玻璃主题（全局）
└── src-tauri/            # Rust 后端
    └── src/
        ├── commands.rs   # Tauri IPC 命令 + 计时循环
        ├── timer.rs      # 计时状态机
        ├── store.rs      # SQLite DAO（WAL）
        ├── screen.rs     # 息屏休息（Win32 SC_MONITORPOWER）
        └── ...
```

## 🗺️ Roadmap

- [ ] 休息倒计时置顶小窗
- [ ] 长休息 / 番茄工作法四轮循环
- [ ] 数据导出 CSV

## 📄 许可

仅供学习交流使用。

<div align="center">

**专注当下，一次一个番茄。** 🍅

</div>
