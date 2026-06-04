use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use super::DeviceInfo;

pub const DISKPULSE_SERVICE_TYPE: &str = "_diskpulse._tcp.local.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryMode {
    Mdns,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryInfo {
    pub service_name: String,
    pub mode: DiscoveryMode,
    pub port: u16,
    pub address_hint: String,
}

pub fn discover_devices(timeout_ms: u64) -> Result<Vec<DeviceInfo>, String> {
    let daemon = mdns_sd::ServiceDaemon::new()
        .map_err(|e| format!("Failed to start mDNS discovery: {e}"))?;
    let receiver = daemon
        .browse(DISKPULSE_SERVICE_TYPE)
        .map_err(|e| format!("Failed to browse DiskPulse services: {e}"))?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(100));
    let mut devices = Vec::new();

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait = remaining.min(Duration::from_millis(100));
        match receiver.recv_timeout(wait) {
            Ok(mdns_sd::ServiceEvent::ServiceResolved(info)) => {
                let address = info
                    .addresses
                    .iter()
                    .find(|addr| addr.is_ipv4())
                    .or_else(|| info.addresses.iter().next())
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|| info.host.trim_end_matches('.').to_string());
                devices.push(device_from_resolved_parts(
                    &info.fullname,
                    &info.host,
                    &address,
                    info.port,
                ));
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    let _ = daemon.stop_browse(DISKPULSE_SERVICE_TYPE);
    let _ = daemon.shutdown();
    devices.sort_by(|left, right| left.name.cmp(&right.name));
    devices.dedup_by(|left, right| left.device_id == right.device_id);
    Ok(devices)
}

pub fn local_service(port: u16) -> DiscoveryInfo {
    DiscoveryInfo {
        service_name: "_diskpulse._tcp.local".into(),
        mode: DiscoveryMode::Manual,
        port,
        address_hint: format!("127.0.0.1:{port}"),
    }
}

pub fn mdns_service(instance_name: &str, port: u16) -> DiscoveryInfo {
    DiscoveryInfo {
        service_name: "_diskpulse._tcp.local".into(),
        mode: DiscoveryMode::Mdns,
        port,
        address_hint: format!("{instance_name}.local:{port}"),
    }
}

pub fn device_from_resolved_parts(
    fullname: &str,
    host: &str,
    address: &str,
    port: u16,
) -> DeviceInfo {
    let name = fullname
        .split('.')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(host.trim_end_matches('.'))
        .to_string();
    DeviceInfo {
        device_id: fullname.to_string(),
        name,
        address: format!("{address}:{port}"),
        paired: false,
        connected: true,
        last_seen_epoch_ms: super::pairing::now_epoch_ms(),
    }
}

pub struct DiscoveryRuntime {
    daemon: mdns_sd::ServiceDaemon,
    fullname: String,
}

impl DiscoveryRuntime {
    pub fn advertise(instance_name: &str, port: u16) -> Result<Self, String> {
        let daemon = mdns_sd::ServiceDaemon::new()
            .map_err(|e| format!("Failed to start mDNS daemon: {e}"))?;
        let hostname = format!("{}.local.", sanitize_dns_label(instance_name));
        let properties = [("app", "DiskPulse"), ("version", env!("CARGO_PKG_VERSION"))];
        let service = mdns_sd::ServiceInfo::new(
            DISKPULSE_SERVICE_TYPE,
            instance_name,
            &hostname,
            "",
            port,
            &properties[..],
        )
        .map_err(|e| format!("Invalid mDNS service info: {e}"))?
        .enable_addr_auto();
        let fullname = service.get_fullname().to_string();
        daemon
            .register(service)
            .map_err(|e| format!("Failed to advertise mDNS service: {e}"))?;
        Ok(Self { daemon, fullname })
    }

    pub fn shutdown(self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

fn sanitize_dns_label(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    sanitized.trim_matches('-').to_string().max_non_empty("diskpulse")
}

trait NonEmptyString {
    fn max_non_empty(self, fallback: &str) -> String;
}

impl NonEmptyString for String {
    fn max_non_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.into()
        } else {
            self
        }
    }
}
