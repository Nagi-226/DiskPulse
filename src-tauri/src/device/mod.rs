//! device —— 串口 MCU 板子状态监测（第一版只做后端，不碰 UI）。
//!
//! **board_* = 串口 MCU 板子（如 ESP32-S3），与 hub 的 DeviceInfo 无关。**
//! hub/relay 的 "device" 指局域网/云端对等 PC/手机；本模块的设备走串口 +
//! [u32 BE len]+JSON 帧（新契约，理由见 protocol.rs 头注），两套模型不共享状态。
//!
//! 状态管理照仓库惯例：static Mutex/OnceLock（同 hub/mod.rs:31-34、lib.rs:63-69），
//! 不用 tauri::State/manage()。command 定义在 lib.rs，本模块只提供实现函数。

pub mod metrics;
pub mod protocol;
pub mod serial;

use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use metrics::BoardStatus;
use protocol::BoardMessage;
use serial::{DeviceTransport, SerialTransport};

/// 单次 board_get_status 最多 drain 的帧数，防止异常数据流把命令卡死。
const MAX_DRAIN_FRAMES: usize = 256;

#[derive(Default)]
struct BoardState {
    transport: Option<Box<dyn DeviceTransport>>,
    port_name: String,
    hello: Option<(String, String)>, // (board, fw_version)
    latest_status: Option<BoardStatus>,
}

fn state() -> &'static Mutex<BoardState> {
    static STATE: OnceLock<Mutex<BoardState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(BoardState::default()))
}

fn lock_state() -> Result<std::sync::MutexGuard<'static, BoardState>, String> {
    state()
        .lock()
        .map_err(|e| format!("Board state lock error: {e}"))
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 列本机串口。无板子时返回空列表是正常结果；枚举本身失败才返回 Err。
pub fn list_ports() -> Result<Vec<String>, String> {
    SerialTransport::new().list_ports()
}

/// 打开串口并替换当前连接（旧连接自动 close）。
pub fn connect(port: &str, baud: u32) -> Result<(), String> {
    connect_with_transport(Box::new(SerialTransport::new()), port, baud)
}

/// 注入任意传输层（mock 测试入口；真实路径走 connect）。
pub fn connect_with_transport(
    mut transport: Box<dyn DeviceTransport>,
    port: &str,
    baud: u32,
) -> Result<(), String> {
    transport.open(port, baud)?;
    let mut guard = lock_state()?;
    if let Some(mut old) = guard.transport.take() {
        old.close();
    }
    guard.transport = Some(transport);
    guard.port_name = port.to_string();
    guard.hello = None;
    guard.latest_status = None;
    Ok(())
}

/// 断开当前连接。
pub fn disconnect() -> Result<(), String> {
    let mut guard = lock_state()?;
    if let Some(mut old) = guard.transport.take() {
        old.close();
    }
    guard.port_name.clear();
    guard.hello = None;
    guard.latest_status = None;
    Ok(())
}

/// drain 传输层里已到达的帧，更新最新状态后返回。
/// 第一版无后台读线程：调用即读，读不到新帧就返回上一次的状态。
pub fn get_status() -> Result<BoardStatus, String> {
    let mut guard = lock_state()?;
    let port_name = guard.port_name.clone();

    // 先 drain 收集载荷（只借 transport），再更新状态，避免 guard 双重可变借用
    let mut payloads = Vec::new();
    {
        let transport = guard
            .transport
            .as_mut()
            .ok_or_else(|| "board not connected (call board_connect first)".to_string())?;
        for _ in 0..MAX_DRAIN_FRAMES {
            match transport.read_frame() {
                Ok(Some(payload)) => payloads.push(payload),
                Ok(None) => break,
                Err(e) => return Err(format!("board read on {port_name}: {e}")),
            }
        }
    }

    for payload in payloads {
        match protocol::decode(&payload) {
            Ok(BoardMessage::Status { mut status }) => {
                status.received_at_epoch_ms = now_epoch_ms();
                guard.latest_status = Some(status);
            }
            Ok(BoardMessage::Hello { board, fw_version }) => {
                guard.hello = Some((board, fw_version));
            }
            Ok(BoardMessage::Ack { .. }) => {}
            Err(e) => return Err(format!("board frame on {port_name}: {e}")),
        }
    }

    guard
        .latest_status
        .clone()
        .ok_or_else(|| format!("no status frame received yet on {port_name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::metrics::{FileNode, FileNodeType, FlashPartition, MemoryStats, TaskStack};
    use crate::device::protocol::encode;
    use crate::device::serial::MockTransport;

    // 全局状态是 static，测试必须串行，否则互相踩连接
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn status_frame(seed: u64) -> Vec<u8> {
        let status = BoardStatus {
            flash: vec![
                FlashPartition {
                    name: "nvs".into(),
                    offset: 0x9000,
                    size: 0x6000,
                    used: 0x2000 + seed,
                },
                FlashPartition {
                    name: "factory".into(),
                    offset: 0x10000,
                    size: 0x200000,
                    used: 0x180000 + seed,
                },
            ],
            heap: MemoryStats {
                total: 327680,
                free: 120000 - seed,
                largest_free_block: 65536,
            },
            psram: Some(MemoryStats {
                total: 8388608,
                free: 4000000 - seed,
                largest_free_block: 2097152,
            }),
            tasks: vec![
                TaskStack {
                    name: "ppm".into(),
                    high_water_mark: 1024 + seed,
                },
                TaskStack {
                    name: "tcp".into(),
                    high_water_mark: 2048,
                },
            ],
            files: vec![
                FileNode {
                    path: "/littlefs".into(),
                    size: 0,
                    node_type: FileNodeType::Dir,
                },
                FileNode {
                    path: "/littlefs/config.json".into(),
                    size: 512 + seed,
                    node_type: FileNodeType::File,
                },
            ],
            received_at_epoch_ms: 0,
        };
        encode(&BoardMessage::Status { status }).expect("encode status")
    }

    #[test]
    fn mock_three_status_frames_parse_all_four_metric_categories() {
        let _serial = TEST_LOCK.lock().expect("test lock");
        let mut mock = MockTransport::new();
        mock.push_bytes(
            encode(&BoardMessage::Hello {
                board: "robomind-s3".into(),
                fw_version: "0.1.0".into(),
            })
            .expect("encode hello"),
        );
        for seed in [1, 2, 3] {
            mock.push_bytes(status_frame(seed));
        }
        connect_with_transport(Box::new(mock), "MOCK0", 115200).expect("connect");

        let status = get_status().expect("get_status");
        // 四类指标全部解析出来，且取的是最后一帧（seed=3）
        assert_eq!(status.flash.len(), 2);
        assert_eq!(status.flash[1].name, "factory");
        assert_eq!(status.flash[1].used, 0x180000 + 3);
        assert_eq!(status.heap.free, 120000 - 3);
        assert_eq!(status.heap.largest_free_block, 65536);
        assert_eq!(status.psram.as_ref().expect("psram").free, 4000000 - 3);
        assert_eq!(status.tasks.len(), 2);
        assert_eq!(status.tasks[0].high_water_mark, 1024 + 3);
        assert_eq!(status.files.len(), 2);
        assert_eq!(status.files[0].node_type, FileNodeType::Dir);
        assert_eq!(status.files[1].size, 512 + 3);
        assert!(status.received_at_epoch_ms > 0);
        println!("mock status parsed: {status:?}");
        disconnect().expect("disconnect");
    }

    #[test]
    fn bad_length_header_surfaces_as_error_not_panic() {
        let _serial = TEST_LOCK.lock().expect("test lock");
        let mut mock = MockTransport::new();
        // 非法长度头 0xFFFFFFFF + 垃圾字节
        mock.push_bytes(vec![0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x01]);
        connect_with_transport(Box::new(mock), "MOCK0", 115200).expect("connect");
        let err = get_status().expect_err("must return explicit error");
        assert!(err.contains("bad length header"), "got: {err}");
        println!("bad header error surfaced: {err}");
        disconnect().expect("disconnect");
    }

    #[test]
    fn get_status_before_connect_is_error() {
        let _serial = TEST_LOCK.lock().expect("test lock");
        disconnect().expect("clean state");
        let err = get_status().expect_err("not connected");
        assert!(err.contains("not connected"), "got: {err}");
    }

    #[test]
    fn connect_without_status_frames_reports_no_status() {
        let _serial = TEST_LOCK.lock().expect("test lock");
        let mock = MockTransport::new();
        connect_with_transport(Box::new(mock), "MOCK0", 115200).expect("connect");
        let err = get_status().expect_err("no status yet");
        assert!(err.contains("no status frame"), "got: {err}");
        disconnect().expect("disconnect");
    }

    #[test]
    fn list_ports_returns_ok_on_this_machine() {
        let ports = list_ports().expect("list_ports must not error");
        println!("board_list_ports actual result: {ports:?}");
    }
}
