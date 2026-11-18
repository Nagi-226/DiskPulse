//! 板端串口帧编解码：`[u32 大端长度] + JSON`。
//!
//! 为什么是新契约而不是复用 hub/relay 的编解码（三点理由，避免后人再问）：
//! 1. 长度前缀是字节流的必需品 —— 串口没有消息边界，没有长度头就无法切帧；
//!    hub 走 WebSocket，协议栈已经帮它分好帧，所以它不需要长度头，两边不可互换。
//! 2. JSON 而非固定字段信封 —— flash 分区列表、任务栈列表、文件树都是变长的，
//!    固定字段信封表达不了。
//! 3. 与 LanChat-Next 的 FrameCodec 帧格式完全同构（[u32 BE len] + payload），
//!    已有先例，将来板端做 JSON 输出时不用再学一套。
//!
//! 第一版只有三种消息：hello（握手）/ status（状态上报）/ ack。
//! 不做真实板端协议协商状态机（板端固件不在本任务范围）。

use serde::{Deserialize, Serialize};

use super::metrics::BoardStatus;

/// 单帧 JSON 上限。文件树可能较大，给 1MiB；超过视为流错位。
pub const MAX_FRAME_LEN: u32 = 1 << 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoardMessage {
    Hello {
        board: String,
        fw_version: String,
    },
    Status {
        #[serde(flatten)]
        status: BoardStatus,
    },
    Ack {
        ok: bool,
    },
}

/// 编码一帧：4 字节大端长度头 + compact JSON。主机下发 hello/ack 用。
pub fn encode(msg: &BoardMessage) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_vec(msg).map_err(|e| format!("board frame encode: {e}"))?;
    if payload.len() > MAX_FRAME_LEN as usize {
        return Err(format!(
            "board frame too large: {} > {MAX_FRAME_LEN}",
            payload.len()
        ));
    }
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// 解码一帧的 JSON 载荷（不含长度头）。
pub fn decode(payload: &[u8]) -> Result<BoardMessage, String> {
    serde_json::from_slice(payload).map_err(|e| format!("board frame decode: {e}"))
}

/// 流式切帧缓冲：串口/mock 共用同一条切帧路径，负向用例测的就是真实路径。
#[derive(Default)]
pub struct FrameBuffer {
    buf: Vec<u8>,
}

impl FrameBuffer {
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// 尝试切出一帧完整 JSON 载荷。
    /// Ok(None) = 半包，等更多字节；Err = 长度头非法，缓冲已清空重新对齐（不 panic）。
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        if self.buf.len() < 4 {
            return Ok(None);
        }
        let n = u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]);
        if n == 0 || n > MAX_FRAME_LEN {
            self.buf.clear();
            return Err(format!(
                "board frame bad length header: {n} (valid: 1..={MAX_FRAME_LEN}), buffer reset"
            ));
        }
        if self.buf.len() < 4 + n as usize {
            return Ok(None);
        }
        let payload = self.buf[4..4 + n as usize].to_vec();
        self.buf.drain(..4 + n as usize);
        Ok(Some(payload))
    }

    pub fn reset(&mut self) {
        self.buf.clear();
    }

    #[cfg(test)]
    pub fn pending(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::metrics::{FileNode, FileNodeType, FlashPartition, MemoryStats, TaskStack};

    fn sample_status() -> BoardStatus {
        BoardStatus {
            flash: vec![FlashPartition {
                name: "factory".into(),
                offset: 0x10000,
                size: 0x200000,
                used: 0x180000,
            }],
            heap: MemoryStats {
                total: 327680,
                free: 120000,
                largest_free_block: 65536,
            },
            psram: Some(MemoryStats {
                total: 8388608,
                free: 4000000,
                largest_free_block: 2097152,
            }),
            tasks: vec![TaskStack {
                name: "ppm".into(),
                high_water_mark: 1024,
            }],
            files: vec![FileNode {
                path: "/littlefs/config.json".into(),
                size: 512,
                node_type: FileNodeType::File,
            }],
            received_at_epoch_ms: 0,
        }
    }

    #[test]
    fn frame_round_trips_all_three_types() {
        let messages = [
            BoardMessage::Hello {
                board: "robomind-s3".into(),
                fw_version: "0.1.0".into(),
            },
            BoardMessage::Status {
                status: sample_status(),
            },
            BoardMessage::Ack { ok: true },
        ];
        for msg in &messages {
            let frame = encode(msg).expect("encode");
            let n = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
            assert_eq!(n, frame.len() - 4);
            let decoded = decode(&frame[4..]).expect("decode");
            assert_eq!(&decoded, msg);
        }
    }

    #[test]
    fn bad_length_header_is_error_not_panic() {
        let mut fb = FrameBuffer::default();
        // 长度头 0xFFFFFFFF 远超上限
        fb.feed(&[0xFF, 0xFF, 0xFF, 0xFF, b'{', b'}']);
        let err = fb.next_frame().expect_err("must reject");
        assert!(err.contains("bad length header"), "got: {err}");
        assert_eq!(fb.pending(), 0, "buffer must be cleared for re-alignment");

        // 零长度同样拒收
        fb.feed(&[0, 0, 0, 0]);
        assert!(fb.next_frame().is_err());
    }

    #[test]
    fn half_packet_waits_then_completes() {
        let frame = encode(&BoardMessage::Ack { ok: true }).expect("encode");
        let mut fb = FrameBuffer::default();
        fb.feed(&frame[..3]); // 连长度头都不完整
        assert_eq!(fb.next_frame().expect("no error"), None);
        fb.feed(&frame[3..6]); // 头完整但 payload 半包
        assert_eq!(fb.next_frame().expect("no error"), None);
        fb.feed(&frame[6..]);
        let payload = fb.next_frame().expect("no error").expect("complete frame");
        assert_eq!(
            decode(&payload).expect("decode"),
            BoardMessage::Ack { ok: true }
        );
        assert_eq!(fb.pending(), 0);
    }

    #[test]
    fn two_frames_in_one_chunk_are_split() {
        let a = encode(&BoardMessage::Ack { ok: true }).expect("encode");
        let b = encode(&BoardMessage::Ack { ok: false }).expect("encode");
        let mut fb = FrameBuffer::default();
        fb.feed(&[a.clone(), b.clone()].concat());
        assert_eq!(
            decode(&fb.next_frame().unwrap().unwrap()).unwrap(),
            BoardMessage::Ack { ok: true }
        );
        assert_eq!(
            decode(&fb.next_frame().unwrap().unwrap()).unwrap(),
            BoardMessage::Ack { ok: false }
        );
        assert_eq!(fb.next_frame().unwrap(), None);
    }

    #[test]
    fn unknown_message_type_is_error() {
        let err = decode(br#"{"type":"reboot"}"#).expect_err("unknown type must fail");
        assert!(err.contains("board frame decode"), "got: {err}");
    }

    #[test]
    fn status_without_psram_parses() {
        let json = br#"{"type":"status","flash":[],"heap":{"total":1,"free":1,"largest_free_block":1},"tasks":[],"files":[]}"#;
        let msg = decode(json).expect("psram optional");
        match msg {
            BoardMessage::Status { status } => assert!(status.psram.is_none()),
            other => panic!("unexpected: {other:?}"),
        }
    }
}
