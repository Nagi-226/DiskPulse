//! 板端四类指标的数据结构（最小字段集，与派单表格一一对应）。
//! 字段名即 JSON wire 字段名（serde 默认 snake_case），板端固件实现时照此输出。

use serde::{Deserialize, Serialize};

/// flash 分区：每个分区的 name / offset / size / used（单位均为字节，offset 为 flash 内偏移）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashPartition {
    pub name: String,
    pub offset: u64,
    pub size: u64,
    pub used: u64,
}

/// heap / PSRAM 水位：total / free / largest_free_block（字节）。
/// heap 与 psram 共用此结构；无 PSRAM 的板子 status 帧里 psram 为 null/缺省。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total: u64,
    pub free: u64,
    pub largest_free_block: u64,
}

/// 任务栈水位：每个任务的 name / high_water_mark（字节，FreeRTOS uxTaskGetStackHighWaterMark 口径）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStack {
    pub name: String,
    pub high_water_mark: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileNodeType {
    Dir,
    File,
}

/// 文件树节点（扁平列表，path 表达层级）：path / size / type。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileNode {
    pub path: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub node_type: FileNodeType,
}

/// 一帧 status 的完整载荷。received_at_epoch_ms 由主机侧接收时填写，不是 wire 字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardStatus {
    pub flash: Vec<FlashPartition>,
    pub heap: MemoryStats,
    #[serde(default)]
    pub psram: Option<MemoryStats>,
    pub tasks: Vec<TaskStack>,
    pub files: Vec<FileNode>,
    #[serde(skip)]
    pub received_at_epoch_ms: u64,
}
