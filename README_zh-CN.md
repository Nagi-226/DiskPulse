# DiskPulse

**Windows 11 实时磁盘空间监控与安全清理工具**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/tauri-2.0-6366f1)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19-06b6d4)](https://react.dev)
[![Rust](https://img.shields.io/badge/rust-1.94-orange)](https://www.rust-lang.org)
[![Windows](https://img.shields.io/badge/windows-11-0078D6)](https://www.microsoft.com/windows)

> [English Version](README.md)

DiskPulse 让你全面掌握磁盘空间使用情况，并安全地回收被浪费的存储空间。基于 Aurora 设计系统打造的精美 UI，由高性能 Rust 后端驱动，恪守"绝不丢失你的数据"的承诺。

## ✨ 功能特性

- **交互式矩形树图** — 直观查看磁盘空间占用，支持逐级下钻到任意子目录
- **智能风险分类** — 16 条内置规则将每个目录划分为低/中/高三个风险等级
- **一键安全清理** — 所有删除操作均进入回收站，绝不永久删除
- **多驱动器支持** — 可扫描任意盘符，实时进度反馈
- **清理报告** — 支持搜索、筛选、排序，可导出为 HTML/CSV 格式
- **并行扫描引擎** — 基于 rayon 实现，500GB 驱动器扫描时间 < 5 秒

## 🛡 安全第一的设计理念

DiskPulse 从底层架构开始就贯彻以下原则：

| 原则 | 说明 |
|------|------|
| 仅回收站 | 应用内不存在任何永久删除的代码路径 |
| 白名单验证 | 仅删除匹配已知安全模式的路径（临时文件、缓存、下载、日志） |
| 系统路径保护 | `C:\Windows`、`Program Files`、`System32`、`WinSxS` — 绝不触碰 |
| 文件锁检测 | 正在使用的文件会被跳过，绝不强制删除 |
| 删除前路径检查 | 每个路径在删除前都需通过所有规则验证 |
| 预览后执行 | 执行清理前可查看完整的文件清单 |

## 🎨 Aurora 设计系统

一套定制的「Windows 11 Fluent 设计 + 数据可视化」设计语言：

- **深空色系** — `#06080d` 背景搭配靛蓝/青色渐变点缀
- **毛玻璃卡片** — 磨砂玻璃效果配合背景模糊
- **动态环形图** — 带发光投影的磁盘使用率图表
- **流光进度条** — 优雅的扫描进度指示器
- **实时监控指示灯** — 绿色脉冲圆点标识实时模式
- **深色主题** — 专为现代 Windows 11 美学打造

## 🚀 快速开始

### 环境要求

- **Windows 11**（主要目标平台）
- **Node.js** ≥ 22
- **Rust** ≥ 1.94（需 `stable-x86_64-pc-windows-msvc` 工具链）
- **Microsoft Visual C++ Build Tools**（windows crate 编译需要）

### 开发模式

```bash
# 克隆仓库
git clone https://github.com/Nagi-226/DiskPulse.git
cd DiskPulse

# 安装前端依赖
npm install

# 启动开发模式（Vite + Tauri）
npm run tauri dev
```

### 生产构建

```bash
# 生产构建（生成 .msi 安装包）
npm run tauri build
```

## 🏗 架构

```
前端 (React/TS)  <-->  Tauri IPC  <-->  Rust 后端
     |                                      |
  ECharts/D3                           walkdir + rayon
  Tailwind CSS                         rusqlite (SQLite)
  lucide-react                         windows-rs (Win32)
```

| 层级 | 技术栈 |
|------|--------|
| 桌面框架 | Tauri 2.x |
| 后端 | Rust — scanner、risk engine、cleaner、watcher、database |
| 前端 | React 19 + TypeScript 5 + Tailwind CSS 4 |
| 可视化 | ECharts 6 + D3 7 |
| 存储 | SQLite（通过 rusqlite） |
| Win32 API | windows crate 0.58 |

## 📦 项目状态

| 版本 | 功能 | 状态 |
|------|------|------|
| v0.0.1 | 项目脚手架 + Aurora 设计 | ✅ |
| v0.0.2 | 扫描器优化 + 多驱动器 + 测试 | ✅ |
| v0.0.3 | ECharts 矩形树图 + 下钻导航 | ✅ |
| v0.0.4 | 风险分类引擎（16 条规则） | ✅ |
| v0.0.5 | 清理报告页面 | ✅ |
| v0.0.6 | 安全清理执行（进行中 75%） | 🚧 |
| v0.0.7 | 实时文件系统监控 + 系统托盘 | 📅 |
| v0.0.8 | 历史趋势图 + SQLite 快照 | 📅 |
| v0.0.9 | 系统集成（DISM、存储感知） | 📅 |
| v0.1.0 | 公开发布候选版 | 📅 |

## ⌨️ IPC 命令

```rust
scan_drive(drive: String) -> DriveInfo          // 全盘扫描（含进度事件）
list_drives() -> Vec<String>                    // 可用驱动器列表
scan_directory(path: String) -> Vec<DirInfo>    // 子目录下钻扫描
classify_risks(scan: DriveInfo) -> RiskReport   // 风险等级分类
preview_cleanup(items: Vec<CleanItem>) -> CleanPreview  // 安全验证
clean_items(items: Vec<CleanItem>) -> CleanResult       // 回收站清理
```

## 🤝 参与贡献

欢迎贡献代码！请阅读以下指引：

1. **分支命名**: `feature/v0.0.X-description` 或 `fix/description`
2. **提交格式**: `feat:`、`fix:`、`refactor:`、`docs:`、`chore:`
3. **Rust**: 必须通过 `rustfmt` + `clippy`，生产代码禁用 `unwrap()`
4. **TypeScript**: 严格模式，禁用 `any` 类型
5. **安全性 PR**: 涉及 `cleaner/` 模块的更改需要充分的测试覆盖和代码审查

详见 [CLAUDE.md](CLAUDE.md)（开发上下文）和 [PROGRESS.md](PROGRESS.md)（当前进度）。

## 📄 许可协议

MIT © 2026 [Nagi_226](https://github.com/Nagi-226)
