use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
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
    ListDevices,
    Ping { id: String },
    Route { envelope: RelayEnvelope },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum RelayServerMessage {
    Registered { device: CloudDevice },
    DeviceList { devices: Vec<CloudDevice> },
    Pong { id: String },
    Routed { envelope: RelayEnvelope },
    Error { message: String },
}

#[derive(Default)]
struct RelayState {
    status: RelayStatus,
    devices: Vec<CloudDevice>,
    client: Option<RelayClientHandle>,
}

#[derive(Clone)]
struct RelayPeer {
    device: CloudDevice,
    sender: mpsc::UnboundedSender<RelayServerMessage>,
}

#[derive(Debug)]
struct RelayClientHandle {
    stop: Arc<AtomicBool>,
    commands: mpsc::UnboundedSender<RelayClientCommand>,
    worker: JoinHandle<()>,
}

#[derive(Debug)]
enum RelayClientCommand {
    ListDevices {
        response: std_mpsc::Sender<Result<Vec<CloudDevice>, String>>,
    },
    RouteHubRequest {
        envelope: RelayEnvelope,
        response: std_mpsc::Sender<Result<crate::hub::HubResponse, String>>,
    },
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
        let peers = Arc::new(Mutex::new(HashMap::new()));
        let worker_peers = Arc::clone(&peers);
        let worker = thread::Builder::new()
            .name("diskpulse-relay-ws".into())
            .spawn(move || run_relay_server(port, worker_stop, worker_peers))
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

    let (device, client) = start_relay_client(&relay_url, &device_id, name)?;
    let status = RelayStatus {
        connected: true,
        url: Some(relay_url.clone()),
        device_id: Some(device.device_id.clone()),
        device_name: Some(device.name.clone()),
        peer_count: 1,
        last_error: None,
        mode: "cloud_bridge".into(),
    };

    let mut state = state()
        .lock()
        .map_err(|e| format!("Relay state lock error: {e}"))?;
    if let Some(previous) = state.client.take() {
        previous.stop();
    }
    state.status = status.clone();
    state.devices = vec![CloudDevice {
        relay_url,
        ..device
    }];
    state.client = Some(client);
    Ok(status)
}

pub fn disconnect() -> Result<RelayStatus, String> {
    let mut state = state()
        .lock()
        .map_err(|e| format!("Relay state lock error: {e}"))?;
    if let Some(client) = state.client.take() {
        client.stop();
    }
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
    let command_sender = state()
        .lock()
        .ok()
        .and_then(|state| state.client.as_ref().map(|client| client.commands.clone()));
    if let Some(commands) = command_sender {
        let (tx, rx) = std_mpsc::channel();
        if commands
            .send(RelayClientCommand::ListDevices { response: tx })
            .is_ok()
        {
            if let Ok(Ok(devices)) = rx.recv_timeout(Duration::from_secs(2)) {
                if let Ok(mut state) = state().lock() {
                    state.devices = devices.clone();
                }
                return devices;
            }
        }
    }

    state()
        .lock()
        .map(|state| state.devices.clone())
        .unwrap_or_default()
}

pub fn route_hub_request(
    to_device_id: &str,
    request: crate::hub::HubRequest,
) -> Result<crate::hub::HubResponse, String> {
    let (envelope, commands) = {
        let state = state()
            .lock()
            .map_err(|e| format!("Relay state lock error: {e}"))?;
        let status = &state.status;
        let from_device_id = status
            .device_id
            .clone()
            .ok_or_else(|| "Relay is not connected".to_string())?;
        let commands = state
            .client
            .as_ref()
            .map(|client| client.commands.clone())
            .ok_or_else(|| "Relay client is not running".to_string())?;
        (
            RelayEnvelope::hub_command(
                request.id.clone(),
                from_device_id,
                Some(to_device_id.into()),
                request.command,
                request.payload,
            ),
            commands,
        )
    };
    validate_relay_envelope(&envelope)?;

    let (tx, rx) = std_mpsc::channel();
    commands
        .send(RelayClientCommand::RouteHubRequest {
            envelope,
            response: tx,
        })
        .map_err(|_| "Relay client command channel is closed".to_string())?;
    rx.recv_timeout(Duration::from_secs(8))
        .map_err(|_| "Timed out waiting for cloud device response".to_string())?
}

impl RelayClientHandle {
    fn stop(self) {
        self.stop.store(true, Ordering::Relaxed);
        drop(self.commands);
        let _ = self.worker.join();
    }
}

fn start_relay_client(
    url: &str,
    device_id: &str,
    device_name: &str,
) -> Result<(CloudDevice, RelayClientHandle), String> {
    let stop = Arc::new(AtomicBool::new(false));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (ready_tx, ready_rx) = std_mpsc::channel();
    let url = url.to_string();
    let device_id = device_id.to_string();
    let device_name = device_name.to_string();
    let worker_stop = Arc::clone(&stop);
    let worker = thread::Builder::new()
        .name("diskpulse-relay-client".into())
        .spawn(move || {
            run_relay_client(
                url,
                device_id,
                device_name,
                worker_stop,
                command_rx,
                ready_tx,
            )
        })
        .map_err(|e| format!("Failed to spawn relay client: {e}"))?;

    let device = ready_rx
        .recv_timeout(Duration::from_secs(3))
        .map_err(|_| "Relay register response timed out".to_string())??;
    Ok((
        device,
        RelayClientHandle {
            stop,
            commands: command_tx,
            worker,
        },
    ))
}

fn run_relay_client(
    url: String,
    device_id: String,
    device_name: String,
    stop: Arc<AtomicBool>,
    command_rx: mpsc::UnboundedReceiver<RelayClientCommand>,
    ready: std_mpsc::Sender<Result<CloudDevice, String>>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
    else {
        let _ = ready.send(Err("Failed to create relay client runtime".into()));
        return;
    };

    runtime.block_on(async move {
        if let Err(e) =
            relay_client_loop(url, device_id, device_name, stop, command_rx, ready).await
        {
            eprintln!("{e}");
        }
    });
}

async fn relay_client_loop(
    url: String,
    device_id: String,
    device_name: String,
    stop: Arc<AtomicBool>,
    mut command_rx: mpsc::UnboundedReceiver<RelayClientCommand>,
    ready: std_mpsc::Sender<Result<CloudDevice, String>>,
) -> Result<(), String> {
    let (ws, _) = tokio::time::timeout(
        Duration::from_secs(2),
        tokio_tungstenite::connect_async(&url),
    )
    .await
    .map_err(|_| "Relay connection timed out".to_string())?
    .map_err(|e| format!("Relay connection failed: {e}"))?;
    let (mut write, mut read) = ws.split();

    let register = RelayClientMessage::Register {
        device_id: device_id.clone(),
        name: device_name,
    };
    write_relay_client_message(&mut write, &register).await?;

    let response = tokio::time::timeout(Duration::from_secs(2), read.next())
        .await
        .map_err(|_| "Relay register response timed out".to_string())?
        .ok_or_else(|| "Relay closed before register response".to_string())?
        .map_err(|e| format!("Relay register response failed: {e}"))?;
    let response = decode_server_message(response)?;
    let device = match response {
        RelayServerMessage::Registered { mut device } => {
            device.relay_url = url.clone();
            device
        }
        RelayServerMessage::Error { message } => return Err(message),
        other => return Err(format!("Unexpected relay register response: {other:?}")),
    };
    let _ = ready.send(Ok(device));

    let mut pending: HashMap<String, std_mpsc::Sender<Result<crate::hub::HubResponse, String>>> =
        HashMap::new();
    let mut pending_device_list: Option<std_mpsc::Sender<Result<Vec<CloudDevice>, String>>> = None;
    while !stop.load(Ordering::Relaxed) {
        tokio::select! {
            command = command_rx.recv() => {
                match command {
                    Some(RelayClientCommand::ListDevices { response }) => {
                        pending_device_list = Some(response);
                        if let Err(e) = write_relay_client_message(&mut write, &RelayClientMessage::ListDevices).await {
                            if let Some(response) = pending_device_list.take() {
                                let _ = response.send(Err(e.clone()));
                            }
                            return Err(e);
                        }
                    }
                    Some(RelayClientCommand::RouteHubRequest { envelope, response }) => {
                        let envelope_id = envelope.id.clone();
                        pending.insert(envelope_id.clone(), response);
                        if let Err(e) = write_relay_client_message(&mut write, &RelayClientMessage::Route { envelope }).await {
                            if let Some(response) = pending.remove(&envelope_id) {
                                let _ = response.send(Err(e.clone()));
                            }
                            return Err(e);
                        }
                    }
                    None => break,
                }
            }
            message = read.next() => {
                let Some(message) = message else { break; };
                let message = decode_server_message(message.map_err(|e| format!("Relay read failed: {e}"))?)?;
                handle_relay_client_server_message(
                    message,
                    &device_id,
                    &mut write,
                    &mut pending,
                    &mut pending_device_list,
                ).await?;
            }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }
    }

    Ok(())
}

async fn write_relay_client_message<S>(
    write: &mut S,
    message: &RelayClientMessage,
) -> Result<(), String>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    write
        .send(Message::Text(
            serde_json::to_string(message)
                .map_err(|e| format!("Relay client encode failed: {e}"))?
                .into(),
        ))
        .await
        .map_err(|e| format!("Relay client send failed: {e}"))
}

fn decode_server_message(message: Message) -> Result<RelayServerMessage, String> {
    serde_json::from_str(
        message
            .to_text()
            .map_err(|e| format!("Relay response was not text: {e}"))?,
    )
    .map_err(|e| format!("Relay response decode failed: {e}"))
}

async fn handle_relay_client_server_message<S>(
    message: RelayServerMessage,
    local_device_id: &str,
    write: &mut S,
    pending: &mut HashMap<String, std_mpsc::Sender<Result<crate::hub::HubResponse, String>>>,
    pending_device_list: &mut Option<std_mpsc::Sender<Result<Vec<CloudDevice>, String>>>,
) -> Result<(), String>
where
    S: futures_util::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    match message {
        RelayServerMessage::Routed { envelope } if envelope.kind == "hub_command" => {
            validate_relay_envelope(&envelope)?;
            let command = envelope
                .payload
                .get("command")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "Hub command envelope requires payload.command".to_string())?
                .to_string();
            let payload = envelope
                .payload
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let response = crate::hub::handle_request(crate::hub::HubRequest {
                id: envelope.id.clone(),
                command,
                device_id: Some(envelope.from_device_id.clone()),
                token: None,
                payload,
            });
            let response_envelope = RelayEnvelope {
                id: envelope.id,
                from_device_id: local_device_id.into(),
                to_device_id: Some(envelope.from_device_id),
                kind: "hub_response".into(),
                payload: serde_json::to_value(response).unwrap_or(serde_json::Value::Null),
            };
            write_relay_client_message(
                write,
                &RelayClientMessage::Route {
                    envelope: response_envelope,
                },
            )
            .await?;
        }
        RelayServerMessage::Routed { envelope } if envelope.kind == "hub_response" => {
            if let Some(response_tx) = pending.remove(&envelope.id) {
                let response = serde_json::from_value::<crate::hub::HubResponse>(envelope.payload)
                    .map_err(|e| format!("Cloud hub response decode failed: {e}"));
                let _ = response_tx.send(response);
            }
        }
        RelayServerMessage::DeviceList { devices } => {
            if let Some(response_tx) = pending_device_list.take() {
                let _ = response_tx.send(Ok(devices));
            }
        }
        RelayServerMessage::Error { message } => {
            if let Some((_id, response_tx)) = pending.drain().next() {
                let _ = response_tx.send(Err(message.clone()));
            }
            if let Some(response_tx) = pending_device_list.take() {
                let _ = response_tx.send(Err(message));
            }
        }
        _ => {}
    }
    Ok(())
}

fn run_relay_server(
    port: u16,
    stop: Arc<AtomicBool>,
    peers: Arc<Mutex<HashMap<String, RelayPeer>>>,
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
                let peers = Arc::clone(&peers);
                tokio::spawn(async move {
                    handle_relay_connection(stream, peers).await;
                });
            }
        }
    });
}

async fn handle_relay_connection(
    stream: tokio::net::TcpStream,
    peers: Arc<Mutex<HashMap<String, RelayPeer>>>,
) {
    let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };
    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<RelayServerMessage>();
    let writer = tokio::spawn(async move {
        while let Some(response) = rx.recv().await {
            let Ok(encoded) = serde_json::to_string(&response) else {
                continue;
            };
            if write.send(Message::Text(encoded.into())).await.is_err() {
                break;
            }
        }
    });

    let mut registered_device_id: Option<String> = None;
    while let Some(message) = read.next().await {
        let response = match message {
            Ok(Message::Text(text)) => {
                process_client_message(&text, &peers, &tx, &mut registered_device_id)
            }
            Ok(Message::Binary(bytes)) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => process_client_message(&text, &peers, &tx, &mut registered_device_id),
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

        if tx.send(response).is_err() {
            break;
        }
    }

    if let Some(device_id) = registered_device_id {
        if let Ok(mut peers) = peers.lock() {
            peers.remove(&device_id);
        }
    }
    writer.abort();
}

fn process_client_message(
    text: &str,
    peers: &Arc<Mutex<HashMap<String, RelayPeer>>>,
    sender: &mpsc::UnboundedSender<RelayServerMessage>,
    registered_device_id: &mut Option<String>,
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
            if let Ok(mut peers) = peers.lock() {
                peers.insert(
                    device_id.clone(),
                    RelayPeer {
                        device: device.clone(),
                        sender: sender.clone(),
                    },
                );
            }
            *registered_device_id = Some(device_id);
            RelayServerMessage::Registered { device }
        }
        Ok(RelayClientMessage::ListDevices) => {
            let devices = peers
                .lock()
                .map(|peers| peers.values().map(|peer| peer.device.clone()).collect())
                .unwrap_or_default();
            RelayServerMessage::DeviceList { devices }
        }
        Ok(RelayClientMessage::Ping { id }) => RelayServerMessage::Pong { id },
        Ok(RelayClientMessage::Route { envelope }) => match validate_relay_envelope(&envelope) {
            Ok(()) => {
                let Some(target_device_id) = envelope.to_device_id.as_deref() else {
                    return RelayServerMessage::Error {
                        message: "Relay route requires to_device_id".into(),
                    };
                };
                let target = peers
                    .lock()
                    .ok()
                    .and_then(|peers| peers.get(target_device_id).cloned());
                if let Some(target) = target {
                    if target
                        .sender
                        .send(RelayServerMessage::Routed {
                            envelope: envelope.clone(),
                        })
                        .is_ok()
                    {
                        RelayServerMessage::Pong { id: envelope.id }
                    } else {
                        RelayServerMessage::Error {
                            message: format!(
                                "Target device '{}' relay channel is closed",
                                target.device.device_id
                            ),
                        }
                    }
                } else {
                    RelayServerMessage::Error {
                        message: format!("Target device '{target_device_id}' is not connected"),
                    }
                }
            }
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

    #[test]
    fn relay_server_routes_envelopes_between_registered_devices() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = RelayRuntime::start(port).unwrap();
        let tokio = tokio::runtime::Runtime::new().unwrap();
        tokio.block_on(async {
            let url = format!("ws://127.0.0.1:{port}");
            let (mut sender, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
            let (mut receiver, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

            for (ws, device_id, name) in [
                (&mut sender, "device-a", "Office PC"),
                (&mut receiver, "device-b", "Laptop"),
            ] {
                let register = RelayClientMessage::Register {
                    device_id: device_id.into(),
                    name: name.into(),
                };
                ws.send(Message::Text(
                    serde_json::to_string(&register).unwrap().into(),
                ))
                .await
                .unwrap();
                let response = ws.next().await.unwrap().unwrap();
                assert!(matches!(
                    serde_json::from_str::<RelayServerMessage>(response.to_text().unwrap())
                        .unwrap(),
                    RelayServerMessage::Registered { .. }
                ));
            }

            let envelope = RelayEnvelope::hub_command(
                "env-route-1".into(),
                "device-a".into(),
                Some("device-b".into()),
                "ping".into(),
                serde_json::Value::Null,
            );
            sender
                .send(Message::Text(
                    serde_json::to_string(&RelayClientMessage::Route { envelope })
                        .unwrap()
                        .into(),
                ))
                .await
                .unwrap();

            let ack = tokio::time::timeout(Duration::from_secs(2), sender.next())
                .await
                .expect("route sender should receive ack")
                .unwrap()
                .unwrap();
            assert!(matches!(
                serde_json::from_str::<RelayServerMessage>(ack.to_text().unwrap()).unwrap(),
                RelayServerMessage::Pong { id } if id == "env-route-1"
            ));

            let forwarded = tokio::time::timeout(Duration::from_secs(2), receiver.next())
                .await
                .expect("route should be forwarded to target")
                .unwrap()
                .unwrap();
            let forwarded: RelayServerMessage =
                serde_json::from_str(forwarded.to_text().unwrap()).unwrap();
            match forwarded {
                RelayServerMessage::Routed { envelope } => {
                    assert_eq!(envelope.id, "env-route-1");
                    assert_eq!(envelope.from_device_id, "device-a");
                    assert_eq!(envelope.to_device_id.as_deref(), Some("device-b"));
                }
                other => panic!("unexpected forwarded relay message: {other:?}"),
            }
        });
        runtime.stop();
    }

    #[test]
    fn relay_server_returns_registered_device_list() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = RelayRuntime::start(port).unwrap();
        let tokio = tokio::runtime::Runtime::new().unwrap();
        tokio.block_on(async {
            let url = format!("ws://127.0.0.1:{port}");
            let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
            let register = RelayClientMessage::Register {
                device_id: "device-list-a".into(),
                name: "List Test PC".into(),
            };
            ws.send(Message::Text(serde_json::to_string(&register).unwrap().into()))
                .await
                .unwrap();
            let _ = ws.next().await.unwrap().unwrap();

            ws.send(Message::Text(
                serde_json::to_string(&RelayClientMessage::ListDevices)
                    .unwrap()
                    .into(),
            ))
            .await
            .unwrap();
            let response = ws.next().await.unwrap().unwrap();
            let response: RelayServerMessage =
                serde_json::from_str(response.to_text().unwrap()).unwrap();
            match response {
                RelayServerMessage::DeviceList { devices } => {
                    assert!(devices
                        .iter()
                        .any(|device| device.device_id == "device-list-a"));
                }
                other => panic!("unexpected device list response: {other:?}"),
            }
        });
        runtime.stop();
    }
}
