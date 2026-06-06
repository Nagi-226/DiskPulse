use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayStatus {
    pub connected: bool,
    pub url: Option<String>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub peer_count: usize,
    pub last_error: Option<String>,
    pub mode: String,
}

impl Default for RelayStatus {
    fn default() -> Self {
        Self {
            connected: false,
            url: None,
            device_id: None,
            device_name: None,
            peer_count: 0,
            last_error: None,
            mode: "local_ready".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudDevice {
    pub device_id: String,
    pub name: String,
    pub relay_url: String,
    pub connected: bool,
    pub last_seen_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayEnvelope {
    pub id: String,
    pub from_device_id: String,
    #[serde(default)]
    pub to_device_id: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl RelayEnvelope {
    pub fn hub_command(
        id: String,
        from_device_id: String,
        to_device_id: Option<String>,
        command: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id,
            from_device_id,
            to_device_id,
            kind: "hub_command".into(),
            payload: serde_json::json!({
                "command": command,
                "payload": payload,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayClientMessage {
    Register { device_id: String, name: String },
    Ping { id: String },
    Route { envelope: RelayEnvelope },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RelayServerMessage {
    Registered { device: CloudDevice },
    Pong { id: String },
    Routed { envelope: RelayEnvelope },
    Error { message: String },
}

#[derive(Default)]
struct RelayState {
    status: RelayStatus,
    devices: Vec<CloudDevice>,
}

fn state() -> &'static Mutex<RelayState> {
    static STATE: OnceLock<Mutex<RelayState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(RelayState::default()))
}

#[derive(Debug)]
pub struct RelayRuntime {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl RelayRuntime {
    pub fn start(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("Relay port must be greater than 0".into());
        }

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let devices = Arc::new(Mutex::new(HashMap::new()));
        let worker_devices = Arc::clone(&devices);
        let worker = thread::Builder::new()
            .name("diskpulse-relay-ws".into())
            .spawn(move || run_relay_server(port, worker_stop, worker_devices))
            .map_err(|e| format!("Failed to spawn relay server: {e}"))?;

        Ok(Self {
            port,
            stop,
            worker: Some(worker),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn validate_relay_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.starts_with("ws://") && trimmed.len() > "ws://".len() {
        return Ok(());
    }
    if trimmed.starts_with("wss://") && trimmed.len() > "wss://".len() {
        return Ok(());
    }
    Err("Relay URL must start with ws:// or wss://".into())
}

pub fn validate_relay_envelope(envelope: &RelayEnvelope) -> Result<(), String> {
    if envelope.id.trim().is_empty() || envelope.from_device_id.trim().is_empty() {
        return Err("Relay envelope requires id and from_device_id".into());
    }

    if envelope.kind == "hub_command" {
        let command = envelope
            .payload
            .get("command")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "Hub command envelope requires payload.command".to_string())?;
        if !crate::hub::is_allowed_remote_command(command) {
            return Err(format!(
                "Relay refuses non-read-only hub command '{command}' without local confirmation"
            ));
        }
    }

    Ok(())
}

pub fn connect(url: &str, device_name: &str) -> Result<RelayStatus, String> {
    validate_relay_url(url)?;
    let name = if device_name.trim().is_empty() {
        "DiskPulse Device"
    } else {
        device_name.trim()
    };
    let device_id = stable_device_id(name);
    let relay_url = url.trim().to_string();

    let device = perform_register_handshake(&relay_url, &device_id, name)?;
    let status = RelayStatus {
        connected: true,
        url: Some(relay_url.clone()),
        device_id: Some(device.device_id.clone()),
        device_name: Some(device.name.clone()),
        peer_count: 1,
        last_error: None,
        mode: "local_ready".into(),
    };

    let mut state = state()
        .lock()
        .map_err(|e| format!("Relay state lock error: {e}"))?;
    state.status = status.clone();
    state.devices = vec![CloudDevice {
        relay_url,
        ..device
    }];
    Ok(status)
}

pub fn disconnect() -> Result<RelayStatus, String> {
    let mut state = state()
        .lock()
        .map_err(|e| format!("Relay state lock error: {e}"))?;
    state.status = RelayStatus::default();
    state.devices.clear();
    Ok(state.status.clone())
}

pub fn status() -> RelayStatus {
    state()
        .lock()
        .map(|state| state.status.clone())
        .unwrap_or_default()
}

pub fn list_cloud_devices() -> Vec<CloudDevice> {
    state()
        .lock()
        .map(|state| state.devices.clone())
        .unwrap_or_default()
}

fn perform_register_handshake(
    url: &str,
    device_id: &str,
    device_name: &str,
) -> Result<CloudDevice, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|e| format!("Failed to create relay client runtime: {e}"))?;

    runtime.block_on(async move {
        let (mut ws, _) = tokio::time::timeout(
            Duration::from_secs(2),
            tokio_tungstenite::connect_async(url),
        )
        .await
        .map_err(|_| "Relay connection timed out".to_string())?
        .map_err(|e| format!("Relay connection failed: {e}"))?;

        let message = RelayClientMessage::Register {
            device_id: device_id.into(),
            name: device_name.into(),
        };
        ws.send(Message::Text(
            serde_json::to_string(&message)
                .map_err(|e| format!("Relay register encode failed: {e}"))?
                .into(),
        ))
        .await
        .map_err(|e| format!("Relay register send failed: {e}"))?;

        let response = tokio::time::timeout(Duration::from_secs(2), ws.next())
            .await
            .map_err(|_| "Relay register response timed out".to_string())?
            .ok_or_else(|| "Relay closed before register response".to_string())?
            .map_err(|e| format!("Relay register response failed: {e}"))?;

        let response: RelayServerMessage = serde_json::from_str(
            response
                .to_text()
                .map_err(|e| format!("Relay response was not text: {e}"))?,
        )
        .map_err(|e| format!("Relay register response decode failed: {e}"))?;

        match response {
            RelayServerMessage::Registered { device } => Ok(device),
            RelayServerMessage::Error { message } => Err(message),
            other => Err(format!("Unexpected relay register response: {other:?}")),
        }
    })
}

fn run_relay_server(
    port: u16,
    stop: Arc<AtomicBool>,
    devices: Arc<Mutex<HashMap<String, CloudDevice>>>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
    else {
        return;
    };

    runtime.block_on(async move {
        let Ok(listener) = TcpListener::bind(("127.0.0.1", port)).await else {
            return;
        };

        while !stop.load(Ordering::Relaxed) {
            if let Ok(Ok((stream, _addr))) =
                tokio::time::timeout(Duration::from_millis(100), listener.accept()).await
            {
                let devices = Arc::clone(&devices);
                tokio::spawn(async move {
                    handle_relay_connection(stream, devices).await;
                });
            }
        }
    });
}

async fn handle_relay_connection(
    stream: tokio::net::TcpStream,
    devices: Arc<Mutex<HashMap<String, CloudDevice>>>,
) {
    let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };

    while let Some(message) = ws.next().await {
        let response = match message {
            Ok(Message::Text(text)) => process_client_message(&text, &devices),
            Ok(Message::Binary(bytes)) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => process_client_message(&text, &devices),
                Err(e) => RelayServerMessage::Error {
                    message: format!("Invalid UTF-8 relay message: {e}"),
                },
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => RelayServerMessage::Error {
                message: format!("Relay WebSocket error: {e}"),
            },
        };

        let Ok(encoded) = serde_json::to_string(&response) else {
            continue;
        };
        if ws.send(Message::Text(encoded.into())).await.is_err() {
            break;
        }
    }
}

fn process_client_message(
    text: &str,
    devices: &Arc<Mutex<HashMap<String, CloudDevice>>>,
) -> RelayServerMessage {
    match serde_json::from_str::<RelayClientMessage>(text) {
        Ok(RelayClientMessage::Register { device_id, name }) => {
            let device = CloudDevice {
                device_id: device_id.clone(),
                name,
                relay_url: "local-relay".into(),
                connected: true,
                last_seen_epoch_ms: now_epoch_ms(),
            };
            if let Ok(mut devices) = devices.lock() {
                devices.insert(device_id, device.clone());
            }
            RelayServerMessage::Registered { device }
        }
        Ok(RelayClientMessage::Ping { id }) => RelayServerMessage::Pong { id },
        Ok(RelayClientMessage::Route { envelope }) => match validate_relay_envelope(&envelope) {
            Ok(()) => RelayServerMessage::Routed { envelope },
            Err(message) => RelayServerMessage::Error { message },
        },
        Err(e) => RelayServerMessage::Error {
            message: format!("Invalid relay message JSON: {e}"),
        },
    }
}

fn stable_device_id(name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    format!("relay-device-{:016x}", hasher.finish())
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn relay_rejects_non_websocket_urls() {
        assert!(validate_relay_url("http://relay.diskpulse.dev").is_err());
        assert!(validate_relay_url("wss://relay.diskpulse.dev").is_ok());
        assert!(validate_relay_url("ws://127.0.0.1:19741").is_ok());
    }

    #[test]
    fn relay_envelope_blocks_cleanup_commands() {
        let envelope = RelayEnvelope::hub_command(
            "env-1".into(),
            "device-a".into(),
            Some("device-b".into()),
            "clean_items".into(),
            serde_json::Value::Null,
        );

        assert!(validate_relay_envelope(&envelope).is_err());
    }

    #[test]
    fn relay_envelope_allows_read_only_hub_commands() {
        let envelope = RelayEnvelope::hub_command(
            "env-1".into(),
            "device-a".into(),
            Some("device-b".into()),
            "scan_meta".into(),
            serde_json::json!({ "drive": "C" }),
        );

        assert!(validate_relay_envelope(&envelope).is_ok());
    }

    #[test]
    fn relay_server_accepts_register_handshake() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = RelayRuntime::start(port).unwrap();
        let tokio = tokio::runtime::Runtime::new().unwrap();
        tokio.block_on(async {
            let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
                .await
                .unwrap();
            let message = RelayClientMessage::Register {
                device_id: "device-a".into(),
                name: "Office PC".into(),
            };
            ws.send(Message::Text(
                serde_json::to_string(&message).unwrap().into(),
            ))
            .await
            .unwrap();

            let response = ws.next().await.unwrap().unwrap();
            let response: RelayServerMessage =
                serde_json::from_str(response.to_text().unwrap()).unwrap();
            match response {
                RelayServerMessage::Registered { device } => {
                    assert_eq!(device.device_id, "device-a");
                    assert_eq!(device.name, "Office PC");
                    assert!(device.connected);
                }
                other => panic!("unexpected relay response: {other:?}"),
            }
        });
        runtime.stop();
    }

    #[test]
    fn connect_disconnect_updates_status_and_cloud_devices() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = RelayRuntime::start(port).unwrap();
        let url = format!("ws://127.0.0.1:{port}");

        let status = connect(&url, "Office PC").unwrap();
        assert!(status.connected);
        assert_eq!(status.url.as_deref(), Some(url.as_str()));
        assert_eq!(list_cloud_devices().len(), 1);

        let status = disconnect().unwrap();
        assert!(!status.connected);
        assert!(list_cloud_devices().is_empty());

        runtime.stop();
    }
}
