mod discovery;
mod pairing;
mod registry;
mod router;
mod server;

use std::sync::{Mutex, OnceLock};

pub use discovery::{DiscoveryInfo, DiscoveryMode};
pub use pairing::{PairingManager, PairingToken};
pub use registry::{DeviceInfo, DeviceRegistry};
pub use router::{is_allowed_remote_command, HubMessage, RemoteAlertPayload};
use server::HubRuntime;

pub const DEVICE_CONNECTED_EVENT: &str = "device-connected";
pub const DEVICE_DISCONNECTED_EVENT: &str = "device-disconnected";
pub const REMOTE_ALERT_EVENT: &str = "remote-alert";

#[derive(Default)]
struct HubState {
    runtime: Option<HubRuntime>,
    discovery_runtime: Option<discovery::DiscoveryRuntime>,
    discovery: Option<DiscoveryInfo>,
    registry: DeviceRegistry,
    pairing: PairingManager,
}

fn state() -> &'static Mutex<HubState> {
    static STATE: OnceLock<Mutex<HubState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HubState::default()))
}

pub fn start(port: u16) -> Result<(), String> {
    let mut state = state()
        .lock()
        .map_err(|e| format!("Hub state lock error: {e}"))?;
    if state.runtime.is_some() {
        return Err("Hub is already running".into());
    }
    let runtime = HubRuntime::start(port)?;
    let instance_name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "diskpulse".into());
    match discovery::DiscoveryRuntime::advertise(&instance_name, runtime.port()) {
        Ok(discovery_runtime) => {
            state.discovery_runtime = Some(discovery_runtime);
            state.discovery = Some(discovery::mdns_service(&instance_name, runtime.port()));
        }
        Err(_) => {
            state.discovery_runtime = None;
            state.discovery = Some(discovery::local_service(runtime.port()));
        }
    }
    state.runtime = Some(runtime);
    Ok(())
}

pub fn stop() -> Result<(), String> {
    let (runtime, discovery_runtime) = {
        let mut state = state()
            .lock()
            .map_err(|e| format!("Hub state lock error: {e}"))?;
        (state.runtime.take(), state.discovery_runtime.take())
    };
    if let Some(discovery_runtime) = discovery_runtime {
        discovery_runtime.shutdown();
    }
    if let Some(runtime) = runtime {
        runtime.stop();
    }
    if let Ok(mut state) = state().lock() {
        state.discovery = None;
    }
    Ok(())
}

pub fn discovery_info() -> Option<DiscoveryInfo> {
    state()
        .lock()
        .ok()
        .and_then(|state| state.discovery.clone())
}

pub fn discover_devices(timeout_ms: u64) -> Result<Vec<DeviceInfo>, String> {
    discovery::discover_devices(timeout_ms)
}

pub fn connected_devices() -> Vec<DeviceInfo> {
    state()
        .lock()
        .map(|state| state.registry.list_devices())
        .unwrap_or_default()
}

pub fn create_pairing_token(device_name: String, ttl_seconds: u64) -> Result<PairingToken, String> {
    Ok(state()
        .lock()
        .map_err(|e| format!("Hub state lock error: {e}"))?
        .pairing
        .create_token(device_name, ttl_seconds))
}

pub fn pair_device(token: &str) -> Result<DeviceInfo, String> {
    let mut state = state()
        .lock()
        .map_err(|e| format!("Hub state lock error: {e}"))?;
    let token = state
        .pairing
        .consume_token(token)
        .ok_or_else(|| "Invalid or expired pairing token".to_string())?;
    let device = DeviceInfo::new_paired(token.device_name, "paired-manually".into());
    state.registry.upsert(device.clone());
    Ok(device)
}

pub fn unpair_device(device_id: &str) -> Result<(), String> {
    let removed = state()
        .lock()
        .map_err(|e| format!("Hub state lock error: {e}"))?
        .registry
        .unpair(device_id);
    if removed {
        Ok(())
    } else {
        Err(format!("Device not found: {device_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_tokens_are_six_digits_and_single_use() {
        let mut pairing = PairingManager::default();

        let token = pairing.create_token("laptop".into(), 60);

        assert_eq!(token.code.len(), 6);
        assert!(token.code.chars().all(|ch| ch.is_ascii_digit()));
        assert!(pairing.consume_token(&token.code).is_some());
        assert!(pairing.consume_token(&token.code).is_none());
    }

    #[test]
    fn expired_pairing_tokens_are_rejected() {
        let mut pairing = PairingManager::default();

        let token = pairing.create_token("phone".into(), 0);

        assert!(pairing.consume_token(&token.code).is_none());
    }

    #[test]
    fn registry_upserts_and_unpairs_devices() {
        let mut registry = DeviceRegistry::default();
        let device = DeviceInfo {
            device_id: "device-1".into(),
            name: "Office PC".into(),
            address: "192.168.1.20:48765".into(),
            paired: true,
            connected: true,
            last_seen_epoch_ms: 1_700_000_000_000,
        };

        registry.upsert(device.clone());
        registry.mark_connected("device-1", false);

        let listed = registry.list_devices();
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].connected);
        assert!(registry.unpair("device-1"));
        assert!(registry.list_devices().is_empty());
    }

    #[test]
    fn router_envelopes_remote_alerts() {
        let alert = RemoteAlertPayload {
            device_id: "device-1".into(),
            alert_payload: serde_json::json!({ "free_bytes": 1024 }),
        };

        let message = HubMessage::RemoteAlert(alert.clone());

        assert_eq!(message.event_name(), "remote-alert");
        assert_eq!(message.device_id(), Some("device-1"));
    }

    #[test]
    fn hub_rejects_invalid_zero_port() {
        assert!(start(0).is_err());
    }

    #[test]
    fn router_allows_only_read_only_remote_commands() {
        assert!(router::is_allowed_remote_command("ping"));
        assert!(router::is_allowed_remote_command("scan_meta"));
        assert!(router::is_allowed_remote_command("scan_drive"));
        assert!(!router::is_allowed_remote_command("clean_items"));
    }

    #[test]
    fn discovery_builds_mdns_service_record() {
        let info = discovery::mdns_service("office-pc", 19740);

        assert_eq!(info.mode, DiscoveryMode::Mdns);
        assert_eq!(info.service_name, "_diskpulse._tcp.local");
        assert_eq!(info.port, 19740);
        assert!(info.address_hint.contains("19740"));
    }

    #[test]
    fn discovery_maps_resolved_service_to_device() {
        let device = discovery::device_from_resolved_parts(
            "Office._diskpulse._tcp.local.",
            "office.local.",
            "192.168.1.10",
            19740,
        );

        assert_eq!(device.name, "Office");
        assert_eq!(device.address, "192.168.1.10:19740");
        assert!(!device.paired);
        assert!(device.connected);
    }

    #[test]
    fn hub_request_round_trips_json() {
        let request = router::HubRequest {
            id: "req-1".into(),
            command: "ping".into(),
            device_id: None,
            token: None,
            payload: serde_json::json!({ "hello": true }),
        };

        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: router::HubRequest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.id, "req-1");
        assert_eq!(decoded.command, "ping");
    }

    #[test]
    fn websocket_server_answers_ping_request() {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::tungstenite::Message;

        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let runtime = HubRuntime::start(port).unwrap();
        let tokio = tokio::runtime::Runtime::new().unwrap();
        tokio.block_on(async {
            let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}"))
                .await
                .unwrap();
            let request = router::HubRequest {
                id: "ping-1".into(),
                command: "ping".into(),
                device_id: None,
                token: None,
                payload: serde_json::Value::Null,
            };
            ws.send(Message::Text(serde_json::to_string(&request).unwrap().into()))
                .await
                .unwrap();

            let response = ws.next().await.unwrap().unwrap();
            let response: router::HubResponse =
                serde_json::from_str(response.to_text().unwrap()).unwrap();
            assert!(response.ok);
            assert_eq!(response.id, "ping-1");
        });
        runtime.stop();
    }
}
