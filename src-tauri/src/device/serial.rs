//! 串口传输层：DeviceTransport trait + MockTransport + SerialTransport。
//!
//! board_* 命令族操作的是**串口 MCU 板子**（如 ESP32-S3），
//! 与 hub 的 DeviceInfo（局域网对等 PC/手机）完全无关，两套设备模型不共享状态。

#[cfg(test)]
use std::collections::VecDeque;
use std::io::{Read, Write};
#[cfg(test)]
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::protocol::FrameBuffer;

/// 传输层抽象：真实串口与 mock 各实现一份，command 层只认 trait。
/// read_frame 返回完整帧的 JSON 载荷（不含长度头）；Ok(None) = 暂无完整帧。
/// write_frame 收的是**已 encode 好的完整帧字节**（含 [u32 BE len] 头）——
/// 组帧是 protocol::encode 的事，传输层只负责把字节送出去，分层不打乱。
pub trait DeviceTransport: Send {
    fn list_ports(&self) -> Result<Vec<String>, String>;
    fn open(&mut self, port: &str, baud: u32) -> Result<(), String>;
    fn read_frame(&mut self) -> Result<Option<Vec<u8>>, String>;
    fn write_frame(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn close(&mut self);
}

/// 内存 mock：从字节队列喂数据，切帧走与真实串口完全相同的 FrameBuffer 路径。
/// 第一版仅供测试注入；command 层真实路径只走 SerialTransport。
#[cfg(test)]
pub struct MockTransport {
    chunks: VecDeque<Vec<u8>>,
    frame_buf: FrameBuffer,
    opened: bool,
    written: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[cfg(test)]
impl MockTransport {
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            frame_buf: FrameBuffer::default(),
            opened: false,
            written: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 注入一段原始字节（可以是一帧、多帧、半帧、或故意坏的长度头）。
    pub fn push_bytes(&mut self, bytes: Vec<u8>) {
        self.chunks.push_back(bytes);
    }

    /// 共享的写入记录：mock 被 move 进连接状态后，测试仍可通过此句柄断言写出的字节。
    pub fn written_log(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
        Arc::clone(&self.written)
    }
}

#[cfg(test)]
impl DeviceTransport for MockTransport {
    fn list_ports(&self) -> Result<Vec<String>, String> {
        Ok(vec!["MOCK0".into()])
    }

    fn open(&mut self, _port: &str, _baud: u32) -> Result<(), String> {
        self.opened = true;
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        if !self.opened {
            return Err("mock transport not opened".into());
        }
        loop {
            match self.frame_buf.next_frame()? {
                Some(payload) => return Ok(Some(payload)),
                None => match self.chunks.pop_front() {
                    Some(chunk) => self.frame_buf.feed(&chunk),
                    None => return Ok(None),
                },
            }
        }
    }

    fn write_frame(&mut self, bytes: &[u8]) -> Result<(), String> {
        if !self.opened {
            return Err("mock transport not opened".into());
        }
        self.written
            .lock()
            .map_err(|e| format!("mock written log lock: {e}"))?
            .push(bytes.to_vec());
        Ok(())
    }

    fn close(&mut self) {
        self.opened = false;
        self.chunks.clear();
        self.frame_buf.reset();
    }
}

/// 真实串口（serialport crate，Windows/Linux/macOS 均支持）。
pub struct SerialTransport {
    port: Option<Box<dyn serialport::SerialPort>>,
    port_name: String,
    frame_buf: FrameBuffer,
}

impl SerialTransport {
    pub fn new() -> Self {
        Self {
            port: None,
            port_name: String::new(),
            frame_buf: FrameBuffer::default(),
        }
    }
}

impl DeviceTransport for SerialTransport {
    fn list_ports(&self) -> Result<Vec<String>, String> {
        let ports = serialport::available_ports().map_err(|e| format!("list serial ports: {e}"))?;
        Ok(ports.into_iter().map(|p| p.port_name).collect())
    }

    fn open(&mut self, port: &str, baud: u32) -> Result<(), String> {
        self.close();
        let handle = serialport::new(port, baud)
            // 读超时让 board_get_status 的 drain 循环不会在无数据时阻塞
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| format!("open serial port {port}: {e}"))?;
        self.port = Some(handle);
        self.port_name = port.to_string();
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Option<Vec<u8>>, String> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| "serial port not opened".to_string())?;
        loop {
            match self.frame_buf.next_frame()? {
                Some(payload) => return Ok(Some(payload)),
                None => {
                    let mut chunk = [0u8; 1024];
                    match port.read(&mut chunk) {
                        Ok(0) => return Ok(None),
                        Ok(n) => self.frame_buf.feed(&chunk[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Ok(None),
                        Err(e) => return Err(format!("serial read: {e}")),
                    }
                }
            }
        }
    }

    fn write_frame(&mut self, bytes: &[u8]) -> Result<(), String> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| "serial port not opened".to_string())?;
        port.write_all(bytes)
            .map_err(|e| format!("serial write on {}: {e}", self.port_name))?;
        port.flush()
            .map_err(|e| format!("serial flush on {}: {e}", self.port_name))?;
        Ok(())
    }

    fn close(&mut self) {
        self.port = None;
        self.port_name.clear();
        self.frame_buf.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::protocol::{encode, BoardMessage};

    #[test]
    fn mock_requires_open_before_read() {
        let mut mock = MockTransport::new();
        assert!(mock.read_frame().is_err());
        mock.open("MOCK0", 115200).expect("open");
        assert_eq!(mock.read_frame().expect("no error"), None);
    }

    #[test]
    fn mock_splits_frames_across_arbitrary_chunks() {
        let frame = encode(&BoardMessage::Ack { ok: true }).expect("encode");
        let mut mock = MockTransport::new();
        mock.open("MOCK0", 115200).expect("open");
        // 一帧拆三段喂入，模拟串口半包
        mock.push_bytes(frame[..2].to_vec());
        mock.push_bytes(frame[2..5].to_vec());
        mock.push_bytes(frame[5..].to_vec());
        let payload = mock.read_frame().expect("no error").expect("frame");
        assert_eq!(
            crate::device::protocol::decode(&payload).expect("decode"),
            BoardMessage::Ack { ok: true }
        );
        assert_eq!(mock.read_frame().expect("no error"), None);
    }

    #[test]
    fn serial_list_ports_does_not_error_on_this_machine() {
        let transport = SerialTransport::new();
        let ports = transport.list_ports().expect("list_ports must not error");
        println!("serial ports on this machine: {ports:?}");
    }
}
