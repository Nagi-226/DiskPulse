# DiskPulse

**实时磁盘空间监控与安全清理工具 — Windows / Linux / macOS**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/tauri-2.0-6366f1)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19-06b6d4)](https://react.dev)
[![Rust](https://img.shields.io/badge/rust-1.94-orange)](https://www.rust-lang.org)
[![Windows](https://img.shields.io/badge/windows-11-0078D6)](https://www.microsoft.com/windows)
[![Linux](https://img.shields.io/badge/linux-FCC624?logo=linux)](https://kernel.org)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple)](https://www.apple.com/macos)

> [English Version](README.md)

DiskPulse 让你全面掌握磁盘空间使用情况，并安全地回收被浪费的存储空间。基于 Aurora 设计系统打造的精美 UI，由高性能 Rust 后端、内核级原生文件监控与智能异常检测驱动，恪守"绝不丢失你的数据"的承诺。

## v0.8.0 生产就绪深度智能

- **磁盘碎片分析** — 基于 extent 的采样碎片评分（FSCTL_GET_RETRIEVAL_POINTERS / FIEMAP / F_LOG2PHYS），TOPN 目录与文件摘要，`FragmentationView` UI。
- **6D 磁盘健康 v2** — 空间容量 / 垃圾堆积 / 增长趋势 / 碎片老化 / 碎片程度 / 异常风险六维雷达图，健康快照历史记录与趋势追踪。
- **预测性清理** — 磁盘满载预测（含置信区间）、清理收益模拟、预清理候选排序与用户确认安全守护。
- **智能文件分类** — 扩展名 + magic-byte 分类流水线，大文件/重复文件/老化文件条目附带 `file_category`。
- **异常融合兜底** — 运行时融合权重（healthy/degraded/disabled），Holt-Winters + Modified Z-Score + 可选 Autoencoder 信号三路融合。
- **代码签名基础** — SignPath 配置 + CI 签名入口 + Homebrew Cask 模板；外部审批待完成。
- **Linux 原生 CI** — ubuntu-latest 依赖、.deb/.AppImage 校验、trash-rs 兜底、inotify 解析器覆盖。
- **macOS FSEvents** — 原生 CoreServices FSEvents 文件监控取代轮询；.dmg 产物校验。
- **代码拆分 + 自动更新** — React.lazy 路由拆分（首屏 <300KB gzip），GitHub Release 更新检查器。
- **国际化 + 性能** — 日语 locale、10 项合成基准测试、边缘场景修复。
- **流式增量扫描** — 首个结果 < 500ms，Treemap 逐批更新。文件变更时自动增量重扫。
- **MFT 直读扫描** — NTFS `FSCTL_ENUM_USN_DATA` 快速近似扫描，自动降级到 JwalkStage。
- **Windows 后台服务** — `diskpulse.exe --service` 无窗口后台运行，通过 Named Pipe 与 GUI 通信。
- **ML 智能异常检测** — Holt-Winters 季节性预测 + Modified Z-Score 检测器。4 种异常类型，纯 Rust 实现，零外部依赖。
- **智能推荐引擎 v2** — 上下文感知的紧迫性倍率（1.0x–3.0x）、用户行为学习、跨模块关联加分。
- **多设备 Dashboard** — 本地 WebSocket Hub、mDNS 发现、6 位配对码、远程只读扫描。
- **自定义规则编辑器** — 创建、编辑、测试、删除自定义清理规则，实时模式测试器。
- **6-Trait 平台抽象层** — 将所有 OS 相关代码隔离在编译期分发的 trait 实现中。
- **CI/CD** — GitHub Actions 三平台矩阵：Windows（MSI + NSIS）、Linux（.deb + .AppImage）、macOS（.dmg）。

## ✅ 功能特性

- **交互式矩形树图** — 直观查看磁盘空间占用，支持逐级下钻到任意子目录
- **智能风险分类** — 16 条内置规则 + 自定义规则编辑器，将每个目录划分为低/中/高三个风险等级
- **一键安全清理** — 所有删除操作均进入回收站/废纸篓，绝不永久删除
- **多驱动器支持** — 可扫描任意盘符，流式实时进度反馈
- **清理报告** — 搜索、筛选、排序分类项目；5 步引导式清理向导
- **原生文件监控** — 内核级文件变更事件（Windows ReadDirectoryChangesW、Linux inotify、macOS FSEvents）
- **重复文件检测** — 三阶段流水线（大小 → 4KB 哈希 → SHA-256），支持硬链接感知
- **文件老化分析** — 7 个时间桶、僵尸文件查找、增长热点检测
- **智能推荐引擎 v2** — 上下文感知评分：紧迫性倍率 + 行为学习 + 关联加分
- **6D 磁盘健康雷达图** — 空间容量 / 垃圾堆积 / 增长趋势 / 碎片老化 / 碎片程度 / 异常风险 + ECharts 雷达图
- **磁盘碎片分析** — 基于 extent 的碎片评分，TOPN 目录与文件摘要
- **预测性清理** — 磁盘满载预测、清理收益模拟、预清理候选与确认守护
- **智能文件分类** — 扩展名 + magic-byte 流水线，扫描条目附带文件类别
- **ML 智能异常检测** — Holt-Winters 季节性预测 + Modified Z-Score；4 种异常类型，融合兜底
- **并行扫描引擎** — jwalk + rayon + 流式推送；500GB 驱动器扫描时间 < 5 秒
- **实时告警** — 低空间阈值 + 突发增长 + 异常检测，Windows 原生通知
- **Windows 后台服务模式** — 无窗口后台监控，Named Pipe IPC，系统托盘集成
- **多设备 Dashboard** — 在局域网发现并监控已配对的 DiskPulse 设备
- **自动清理调度** — 可配置的 LOW 风险自动清理
- **国际化** — 英文 + 简体中文 + 日语，自动检测系统语言
- **深色/浅色主题** — Aurora 设计系统，CSS 变量令牌

## 🛡️ 安全第一的设计理念

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

一套定制的"Windows 11 Fluent 设计 + 数据可视化"设计语言：

- **深空色系** — `#06080d` 背景搭配靛蓝/青色渐变点缀
- **毛玻璃卡片** — 磨砂玻璃效果配合背景模糊
- **动态环形图** — 带发光投影的磁盘使用率图表
- **流光进度条** — 优雅的扫描进度指示器
- **实时监控指示灯** — 绿色脉冲圆点标识实时模式
- **深色/浅色主题** — CSS 变量令牌，自动跟随系统偏好

## 🚀 快速开始

### 环境要求

- **Windows 11** / **Linux** / **macOS**
- **Node.js** ≥ 22
- **Rust** ≥ 1.94
- **Windows**: Microsoft Visual C++ Build Tools
- **Linux**: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`
- **macOS**: Xcode Command Line Tools

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
# 生产构建
npm run tauri build

# 各平台产物：
#   Windows: .msi + .exe (NSIS)
#   Linux:   .deb + .AppImage
#   macOS:   .dmg
```

### CLI 模式

```bash
# 扫描驱动器
diskpulse --cli scan C

# 完整健康检查
diskpulse --cli health C --json

# 预览清理候选
diskpulse --cli clean C --dry-run

# 导出扫描报告
diskpulse --cli export C json scan
```

## 🏗 架构

```
前端 (React/TS)  <-->  Tauri IPC  <-->  Rust 后端
     |                                      |
  ECharts/D3                    ┌────────────────────┐
  Tailwind CSS                  │ 6-trait 平台抽象层 │
  lucide-react                  └────────────────────┘
  react-i18next                 walkdir/jwalk + rayon
                                rusqlite (SQLite)
                                windows-rs / inotify / FSEvents
```

| 层级 | 技术栈 |
|------|--------|
| 桌面框架 | Tauri 2.x |
| 后端 | Rust — 31 个源文件、6 个平台 trait、129 项测试 |
| 前端 | React 19 + TypeScript 5 + Tailwind CSS 4 |
| 可视化 | ECharts 6 + D3 7 |
| 存储 | SQLite（通过 rusqlite） |
| 平台 API | windows crate 0.58 / inotify FFI / FSEvents + sysctl |
| 知识图谱 | graphify-rs — 995 个节点、1356 条边 |

## 📦 项目状态

| 版本 | 功能 | 状态 |
|------|------|------|
| v0.0.1–0.0.9 | 核心基础：扫描器、风险引擎、清理器、文件监控、历史记录、设置 | ✅ |
| v0.1.0 | 正式发布候选版 | ✅ |
| v0.2.0 | 性能与用户体验优化 | ✅ |
| v0.2.5–0.2.9 | 智能洞察：告警、预测、大文件、自动清理 | ✅ |
| v0.3.0 | 生产发布 | ✅ |
| v0.4.0 | 可扩展智能平台（国际化、主题、去重、老化分析、推荐引擎） | ✅ |
| v0.5.0 | 集成卓越（跨模块数据流、CLI、清理向导、通知中心） | ✅ |
| **v0.6.0** | **跨平台性能基础（原生监控、6-trait 架构、Linux、macOS）** | ✅ |
| **v0.7.0** | **智能运维平台（119 项测试，多设备 Dashboard）** | ✅ |
| **v0.7.1** | **代码签名基础（SignPath 配置、Homebrew Cask 模板、CI 签名入口）** | ✅ Local |
| **v0.7.2** | **Linux 原生 CI 配置（ubuntu-latest 依赖、.deb/.AppImage 校验、trash-rs 兜底）** | ✅ Local |
| **v0.7.5** | **生产就绪强化（macOS FSEvents、代码拆分、更新检查、性能基准、日语 locale）** | ✅ Local |
| **v0.8.0** | **生产就绪深度智能（碎片分析、异常融合、6D 健康、预测性清理、文件分类）** | ✅ Local |

## ⌨️ IPC 命令

```rust
// 扫描器
scan_drive(drive: String) -> DriveInfo
scan_drive_meta(drive: String) -> DriveMeta
scan_drive_dirs(drive: String) -> Vec<DirInfo>
cancel_scan() -> ()
find_large_files(drive: String, min_size: u64, limit: usize) -> Vec<FileEntry>
cancel_large_file_scan() -> ()
list_drives() -> Vec<String>
scan_directory(path: String) -> Vec<DirInfo>

// 风险分类
classify_risks(scan: DriveInfo) -> RiskReport

// 清理器
preview_cleanup(items: Vec<CleanItem>) -> CleanPreview
clean_items(items: Vec<CleanItem>) -> CleanResult
undo_cleanup(original_paths: Vec<String>) -> RestoreResult
run_auto_cleanup_now() -> CleanResult
get_auto_cleanup_status() -> AutoCleanupStatus
get_auto_cleanup_history() -> Vec<AutoCleanupReport>

// 文件监控
start_fs_watcher() -> String
stop_fs_watcher() -> String

// 告警
start_alert_monitor() -> String
stop_alert_monitor() -> String

// 历史记录
get_snapshot_history(drive: String, days: u32) -> Vec<Snapshot>
get_cleanup_history() -> Vec<CleanupLog>
predict_disk_usage(drive: String, days: u32) -> Prediction

// 重复文件 & 老化分析
scan_duplicates(drive: String, min_size: u64) -> Vec<DuplicateGroup>
cancel_duplicate_scan() -> ()
analyze_file_aging(drive: String) -> AgingReport
cancel_aging_scan() -> ()

// 推荐引擎 & 磁盘健康
get_recommendations(drive: String) -> Vec<Recommendation>
get_disk_health(drive: String) -> DiskHealth
get_health_history(drive: String, limit: usize) -> Vec<HealthSnapshot>

// 异常检测 & 碎片分析 (v0.8.0)
detect_anomalies(drive: String) -> AnomalyReport
analyze_fragmentation(drive: String) -> FragmentationReport
get_file_fragmentation(path: String) -> FileFragmentation
cancel_fragmentation_scan() -> ()

// 预测性清理 (v0.8.0)
predict_disk_full(drive: String) -> DiskFullPrediction
simulate_cleanup_gain(items: Vec<CleanItem>) -> CleanupGainEstimate
get_pre_cleanup_candidates(drive: String) -> Vec<CleanItem>
execute_pre_cleanup(items: Vec<CleanItem>) -> CleanResult

// 多设备 Hub
start_hub(port: u16) -> ()
stop_hub() -> ()
get_connected_devices() -> Vec<DeviceInfo>
get_hub_discovery_info() -> Option<DiscoveryInfo>
discover_devices(timeout_ms: u64) -> Vec<DeviceInfo>
create_pairing_token(device_name: String, ttl_seconds: u64) -> PairingToken
pair_device(token: String) -> DeviceInfo
unpair_device(device_id: String) -> ()

// 规则 & 导出
create_custom_rule(name: String, pattern: String, risk_level: String) -> RiskRule
delete_custom_rule(rule_id: String) -> ()
test_rule_pattern(pattern: String, test_path: String) -> bool
export_scan_report(drive: String, format: String) -> String
export_cleanup_history(format: String) -> String
export_duplicates(drive: String, format: String) -> String

// 通知
get_notifications() -> Vec<NotificationRecord>
mark_notifications_read() -> ()
mark_notification_read(id: i64) -> ()
clear_notifications() -> ()

// 系统信息 (v0.6.0)
get_system_info() -> PlatformSystemInfo
get_file_meta(path: String) -> FileMeta

// 后台服务 (v0.6.4)
install_service() -> ()
uninstall_service() -> ()
get_service_status() -> ServiceStatus

// 设置
get_settings() -> AppSettings
save_settings(settings: AppSettings) -> ()
get_rules() -> Vec<RiskRule>
save_rule_override(rule_id: String, safe_to_delete: bool) -> ()

// 应用
app_version() -> String
```

## 🤝 参与贡献

欢迎贡献代码！请阅读以下指引：

1. **分支命名**: `feature/v0.0.X-description` 或 `fix/description`
2. **提交格式**: `feat:`、`fix:`、`refactor:`、`docs:`、`chore:`
3. **Rust**: 必须通过 `rustfmt` + `clippy`，生产代码禁用 `unwrap()`
4. **TypeScript**: 严格模式，禁用 `any` 类型
5. **安全性 PR**: 涉及 `cleaner/` 模块的更改需要充分的测试覆盖和代码审查

详见 [CLAUDE.md](CLAUDE.md)（开发上下文）、[PROGRESS.md](PROGRESS.md)（当前进度）和 [CODEX.md](CODEX.md)（实施任务）。

## 📫 许可协议

MIT © 2026 [Nagi_226](https://github.com/Nagi-226)
