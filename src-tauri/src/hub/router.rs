use super::{DeviceInfo, DEVICE_CONNECTED_EVENT, DEVICE_DISCONNECTED_EVENT, REMOTE_ALERT_EVENT};
use serde::{Deserialize, Serialize};
use serde_json::json;

const READ_ONLY_COMMANDS: &[&str] = &[
    "ping",
    "scan_meta",
    "scan_drive",
    "get_disk_health",
    "detect_anomalies",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HubRequest {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HubResponse {
    pub id: String,
    pub ok: bool,
    pub payload: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteAlertPayload {
    pub device_id: String,
    pub alert_payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum HubMessage {
    DeviceConnected(DeviceInfo),
    DeviceDisconnected { device_id: String },
    RemoteAlert(RemoteAlertPayload),
}

impl HubMessage {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::DeviceConnected(_) => DEVICE_CONNECTED_EVENT,
            Self::DeviceDisconnected { .. } => DEVICE_DISCONNECTED_EVENT,
            Self::RemoteAlert(_) => REMOTE_ALERT_EVENT,
        }
    }

    pub fn device_id(&self) -> Option<&str> {
        match self {
            Self::DeviceConnected(device) => Some(&device.device_id),
            Self::DeviceDisconnected { device_id } => Some(device_id),
            Self::RemoteAlert(payload) => Some(&payload.device_id),
        }
    }
}

pub fn is_allowed_remote_command(command: &str) -> bool {
    READ_ONLY_COMMANDS.contains(&command)
}

pub fn handle_request(request: HubRequest) -> HubResponse {
    if !is_allowed_remote_command(&request.command) {
        return HubResponse {
            id: request.id,
            ok: false,
            payload: serde_json::Value::Null,
            error: Some(format!(
                "Remote command '{}' is not allowed without local confirmation",
                request.command
            )),
        };
    }

    match request.command.as_str() {
        "ping" => ok(request.id, json!({ "message": "pong" })),
        "scan_meta" => {
            let drive = request
                .payload
                .get("drive")
                .and_then(|value| value.as_str())
                .unwrap_or("C");
            match crate::scanner::scan_drive_meta(drive, None, None) {
                Ok(meta) => ok(
                    request.id,
                    serde_json::to_value(meta).unwrap_or(serde_json::Value::Null),
                ),
                Err(e) => err(request.id, e),
            }
        }
        "scan_drive" => {
            let drive = request
                .payload
                .get("drive")
                .and_then(|value| value.as_str())
                .unwrap_or("C");
            match crate::scanner::scan_drive(drive) {
                Ok(info) => ok(
                    request.id,
                    serde_json::to_value(info).unwrap_or(serde_json::Value::Null),
                ),
                Err(e) => err(request.id, e),
            }
        }
        "get_disk_health" => {
            let drive = request
                .payload
                .get("drive")
                .and_then(|value| value.as_str())
                .unwrap_or("C");
            match crate::recommendations::get_disk_health(drive) {
                Ok(health) => ok(
                    request.id,
                    serde_json::to_value(health).unwrap_or(serde_json::Value::Null),
                ),
                Err(e) => err(request.id, e),
            }
        }
        "detect_anomalies" => {
            let drive = request
                .payload
                .get("drive")
                .and_then(|value| value.as_str())
                .unwrap_or("C");
            match crate::anomaly::detect_anomalies(drive) {
                Ok(report) => ok(
                    request.id,
                    serde_json::to_value(report).unwrap_or(serde_json::Value::Null),
                ),
                Err(e) => err(request.id, e),
            }
        }
        _ => err(request.id, "Unsupported remote command".into()),
    }
}

fn ok(id: String, payload: serde_json::Value) -> HubResponse {
    HubResponse {
        id,
        ok: true,
        payload,
        error: None,
    }
}

fn err(id: String, error: String) -> HubResponse {
    HubResponse {
        id,
        ok: false,
        payload: serde_json::Value::Null,
        error: Some(error),
    }
}
