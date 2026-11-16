use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebDashboardStatus {
    pub running: bool,
    pub url: Option<String>,
    pub port: Option<u16>,
    pub mode: String,
}

impl Default for WebDashboardStatus {
    fn default() -> Self {
        Self {
            running: false,
            url: None,
            port: None,
            mode: "localhost_read_only".into(),
        }
    }
}

#[derive(Debug)]
pub struct WebDashboardRuntime {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct WebDashboardState {
    runtime: Option<WebDashboardRuntime>,
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

#[derive(Debug)]
struct HttpResponse {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct WebCommandResponse {
    id: String,
    ok: bool,
    payload: serde_json::Value,
    error: Option<String>,
}

fn state() -> &'static Mutex<WebDashboardState> {
    static STATE: OnceLock<Mutex<WebDashboardState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WebDashboardState::default()))
}

impl WebDashboardRuntime {
    pub fn start(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("Web Dashboard port must be greater than 0".into());
        }

        let listener = TcpListener::bind(("127.0.0.1", port))
            .map_err(|e| format!("Failed to bind Web Dashboard on localhost:{port}: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Failed to configure Web Dashboard listener: {e}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("diskpulse-web-dashboard".into())
            .spawn(move || run_web_server(listener, worker_stop))
            .map_err(|e| format!("Failed to spawn Web Dashboard server: {e}"))?;

        Ok(Self {
            port,
            stop,
            worker: Some(worker),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn status(&self) -> WebDashboardStatus {
        WebDashboardStatus {
            running: !self.stop.load(Ordering::Relaxed),
            url: Some(format!("http://127.0.0.1:{}", self.port)),
            port: Some(self.port),
            mode: "localhost_read_only".into(),
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn is_web_read_only_command(command: &str) -> bool {
    crate::hub::is_allowed_remote_command(command)
        || web_only_read_only_commands().contains(&command)
}

pub fn start(port: u16) -> Result<WebDashboardStatus, String> {
    let mut state = state()
        .lock()
        .map_err(|e| format!("Web Dashboard state lock error: {e}"))?;
    if state.runtime.is_some() {
        return Err("Web Dashboard is already running".into());
    }

    let runtime = WebDashboardRuntime::start(port)?;
    let status = runtime.status();
    state.runtime = Some(runtime);
    Ok(status)
}

pub fn stop() -> Result<WebDashboardStatus, String> {
    let runtime = state()
        .lock()
        .map_err(|e| format!("Web Dashboard state lock error: {e}"))?
        .runtime
        .take();
    if let Some(runtime) = runtime {
        runtime.stop();
    }
    Ok(WebDashboardStatus::default())
}

pub fn get_status() -> WebDashboardStatus {
    state()
        .lock()
        .ok()
        .and_then(|state| state.runtime.as_ref().map(WebDashboardRuntime::status))
        .unwrap_or_default()
}

fn web_only_read_only_commands() -> &'static [&'static str] {
    &[
        "analyze_file_aging",
        "analyze_fragmentation",
        "app_version",
        "classify_risks",
        "detect_anomalies",
        "discover_devices",
        "find_large_files",
        "get_auto_cleanup_history",
        "get_auto_cleanup_status",
        "get_cleanup_history",
        "get_connected_devices",
        "get_disk_health",
        "get_file_fragmentation",
        "get_health_history",
        "get_hub_discovery_info",
        "get_model_status",
        "get_notifications",
        "get_pre_cleanup_candidates",
        "get_recommendations",
        "get_relay_status",
        "get_rules",
        "get_settings",
        "get_snapshot_history",
        "get_storage_info",
        "get_system_info",
        "list_cloud_devices",
        "list_drives",
        "list_external_storage",
        "predict_disk_full",
        "predict_disk_usage",
        "preview_cleanup",
        "scan_directory",
        "scan_drive_dirs",
        "scan_drive_meta",
        "scan_duplicates",
        "simulate_cleanup_gain",
        "test_rule_pattern",
    ]
}

fn run_web_server(listener: TcpListener, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _addr)) => handle_connection(stream),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let response = match read_http_request(&mut stream) {
        Ok(request) => route_request(request),
        Err(error) => json_response(
            "400 Bad Request",
            json!({ "error": "bad_request", "message": error }),
        ),
    };
    let _ = write_http_response(&mut stream, response);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Both);
}

fn route_request(request: HttpRequest) -> HttpResponse {
    if request.method == "OPTIONS" {
        return HttpResponse {
            status: "204 No Content",
            content_type: "text/plain; charset=utf-8",
            body: Vec::new(),
        };
    }

    let clean_path = request.path.split('?').next().unwrap_or("/");
    if request.method == "GET" && clean_path == "/api/health" {
        return json_response(
            "200 OK",
            json!({
                "service": "diskpulse-web",
                "mode": "localhost_read_only",
                "version": env!("CARGO_PKG_VERSION"),
            }),
        );
    }

    if request.method == "GET" && clean_path == "/ws" {
        return json_response(
            "501 Not Implemented",
            json!({
                "error": "websocket_proxy_pending",
                "message": "WebSocket proxy is reserved for Web Dashboard realtime events; REST polling remains active."
            }),
        );
    }

    if request.method == "POST" {
        if let Some(command) = clean_path.strip_prefix("/api/cmd/") {
            return handle_api_command(command, &request.body);
        }
    }

    if request.method == "GET" {
        return serve_static_asset(clean_path)
            .unwrap_or_else(|| json_response("404 Not Found", json!({ "error": "not_found" })));
    }

    json_response(
        "405 Method Not Allowed",
        json!({ "error": "method_not_allowed" }),
    )
}

fn handle_api_command(encoded_command: &str, body: &str) -> HttpResponse {
    let command = percent_decode(encoded_command);
    let payload = if body.trim().is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(value) => value,
            Err(error) => {
                return command_response(WebCommandResponse {
                    id: "web-dashboard".into(),
                    ok: false,
                    payload: serde_json::Value::Null,
                    error: Some(format!("Invalid command payload: {error}")),
                })
            }
        }
    };

    command_response(handle_web_command(&command, payload))
}

fn handle_web_command(command: &str, payload: serde_json::Value) -> WebCommandResponse {
    let id = payload
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or("web-dashboard")
        .to_string();

    if !is_web_read_only_command(command) {
        return err(
            id,
            format!(
                "Web Dashboard command '{command}' requires desktop confirmation and is not available from browser mode"
            ),
        );
    }

    match command {
        "ping" => ok(id, json!({ "message": "pong" })),
        "app_version" => ok(id, env!("CARGO_PKG_VERSION")),
        "list_drives" => result(id, crate::platform::providers().disk_info.list_drives()),
        "get_settings" => result(id, crate::db::get_settings()),
        "get_system_info" => {
            let providers = crate::platform::providers();
            result(
                id,
                (|| -> Result<crate::platform::PlatformSystemInfo, String> {
                    Ok(crate::platform::PlatformSystemInfo {
                        os_name: providers.system_info.os_name()?,
                        os_version: providers.system_info.os_version()?,
                        cpu_count: providers.system_info.cpu_count(),
                        total_ram_bytes: providers.system_info.total_ram_bytes()?,
                        app_data_dir: providers.system_info.app_data_dir()?,
                    })
                })(),
            )
        }
        "scan_drive_meta" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::scanner::scan_drive_meta(&drive, None, None))
        }
        "scan_drive" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::scanner::scan_drive(&drive))
        }
        "scan_drive_dirs" => {
            let drive = string_arg(&payload, "drive", "C");
            result(
                id,
                crate::scanner::scan_drive(&drive).map(|info| info.top_dirs),
            )
        }
        "scan_directory" => {
            let path = string_arg(&payload, "path", "");
            result(id, crate::scanner::scan_directory(&path))
        }
        "classify_risks" => match payload.get("scan").cloned() {
            Some(scan) => match serde_json::from_value::<crate::scanner::DriveInfo>(scan) {
                Ok(scan) => {
                    let mut rules = crate::risk::built_in_rules();
                    if let Ok(custom_rules) = crate::db::get_custom_rules() {
                        rules.extend(custom_rules);
                    }
                    ok(id, crate::risk::classify_risks_with_rules(&scan, &rules))
                }
                Err(error) => err(id, format!("Invalid scan payload: {error}")),
            },
            None => err(id, "Missing scan payload".into()),
        },
        "find_large_files" => {
            let drive = string_arg(&payload, "drive", "C");
            let min_size = u64_arg(&payload, "minSize", "min_size", 0);
            let limit = usize_arg(&payload, "limit", 100);
            result(
                id,
                crate::scanner::find_large_files_with_progress_and_cancel(
                    &drive,
                    min_size,
                    limit,
                    |_| {},
                    None,
                ),
            )
        }
        "scan_duplicates" => {
            let drive = string_arg(&payload, "drive", "C");
            let min_size = u64_arg(&payload, "minSize", "min_size", 0);
            result(
                id,
                crate::duplicates::scan_duplicates_with_progress_and_cancel(
                    &drive,
                    min_size,
                    |_| {},
                    None,
                ),
            )
        }
        "analyze_file_aging" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::aging::analyze_file_aging(&drive))
        }
        "get_snapshot_history" => {
            let drive = string_arg(&payload, "drive", "C");
            let days = u32_arg(&payload, "days", 30);
            result(id, crate::db::get_snapshot_history(&drive, days))
        }
        "get_cleanup_history" => result(id, crate::db::get_cleanup_history()),
        "get_auto_cleanup_history" => result(id, crate::scheduler::get_auto_cleanup_history()),
        "predict_disk_usage" => {
            let drive = string_arg(&payload, "drive", "C");
            let days = u32_arg(&payload, "days", 30);
            result(id, crate::prediction::predict_disk_usage(&drive, days))
        }
        "predict_disk_full" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::prediction::predict_disk_full(&drive))
        }
        "detect_anomalies" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::anomaly::detect_anomalies(&drive))
        }
        "get_recommendations" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::recommendations::get_recommendations(&drive))
        }
        "get_disk_health" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::recommendations::get_disk_health(&drive))
        }
        "get_health_history" => {
            let drive = string_arg(&payload, "drive", "C");
            let limit = usize_arg(&payload, "limit", 30);
            result(id, crate::db::get_health_history(&drive, limit))
        }
        "get_pre_cleanup_candidates" => {
            let drive = string_arg(&payload, "drive", "C");
            result(
                id,
                crate::recommendations::get_pre_cleanup_candidates(&drive),
            )
        }
        "simulate_cleanup_gain" => match payload.get("items").cloned() {
            Some(items) => match serde_json::from_value::<Vec<crate::cleaner::CleanItem>>(items) {
                Ok(items) => {
                    let drive = string_arg(&payload, "drive", "C");
                    result(id, crate::prediction::simulate_cleanup_gain(&drive, items))
                }
                Err(error) => err(id, format!("Invalid cleanup item payload: {error}")),
            },
            None => err(id, "Missing items payload".into()),
        },
        "preview_cleanup" => match payload.get("items").cloned() {
            Some(items) => match serde_json::from_value::<Vec<crate::cleaner::CleanItem>>(items) {
                Ok(items) => ok(id, crate::cleaner::preview_cleanup(items)),
                Err(error) => err(id, format!("Invalid cleanup item payload: {error}")),
            },
            None => err(id, "Missing items payload".into()),
        },
        "analyze_fragmentation" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::fragmentation::analyze_drive(&drive, None))
        }
        "get_file_fragmentation" => {
            let path = string_arg(&payload, "path", "");
            result(
                id,
                crate::fragmentation::get_file_fragmentation(Path::new(&path)),
            )
        }
        "list_external_storage" => result(id, crate::storage::list_external_storage()),
        "get_storage_info" => {
            let path = string_arg(&payload, "path", "");
            result(id, crate::storage::get_storage_info(&path))
        }
        "get_model_status" => {
            let drive = string_arg(&payload, "drive", "C");
            result(id, crate::model_manager::get_model_status(&drive))
        }
        "get_auto_cleanup_status" => result(id, crate::scheduler::get_auto_cleanup_status()),
        "get_notifications" => result(id, crate::db::get_notifications()),
        "get_rules" => {
            let rules = crate::db::get_rule_overrides().and_then(|overrides| {
                let mut rules = crate::risk::get_rules_with_overrides(&overrides);
                rules.extend(crate::db::get_custom_rules()?);
                Ok(rules)
            });
            result(id, rules)
        }
        "test_rule_pattern" => {
            let pattern = string_arg(&payload, "pattern", "");
            let test_path = string_arg_alt(&payload, "testPath", "test_path", "");
            ok(id, crate::risk::test_rule_pattern(&pattern, &test_path))
        }
        "get_connected_devices" => ok(id, crate::hub::connected_devices()),
        "get_hub_discovery_info" => ok(id, crate::hub::discovery_info()),
        "discover_devices" => {
            let timeout_ms = u64_arg(&payload, "timeoutMs", "timeout_ms", 1500);
            result(id, crate::hub::discover_devices(timeout_ms))
        }
        "get_relay_status" => ok(id, crate::relay::status()),
        "list_cloud_devices" => {
            let devices = crate::relay::list_cloud_devices();
            let _ = crate::hub::sync_cloud_devices(devices.clone());
            ok(id, devices)
        }
        _ => {
            let request = crate::hub::HubRequest {
                id,
                command: command.into(),
                device_id: None,
                token: None,
                payload,
            };
            let response = crate::hub::handle_request(request);
            WebCommandResponse {
                id: response.id,
                ok: response.ok,
                payload: response.payload,
                error: response.error,
            }
        }
    }
}

fn string_arg(payload: &serde_json::Value, key: &str, default: &str) -> String {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(default)
        .to_string()
}

fn string_arg_alt(
    payload: &serde_json::Value,
    camel_key: &str,
    snake_key: &str,
    default: &str,
) -> String {
    payload
        .get(camel_key)
        .or_else(|| payload.get(snake_key))
        .and_then(|value| value.as_str())
        .unwrap_or(default)
        .to_string()
}

fn u32_arg(payload: &serde_json::Value, key: &str, default: u32) -> u32 {
    payload
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(default)
}

fn u64_arg(payload: &serde_json::Value, camel_key: &str, snake_key: &str, default: u64) -> u64 {
    payload
        .get(camel_key)
        .or_else(|| payload.get(snake_key))
        .and_then(|value| value.as_u64())
        .unwrap_or(default)
}

fn usize_arg(payload: &serde_json::Value, key: &str, default: usize) -> usize {
    payload
        .get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

fn ok<T: Serialize>(id: String, payload: T) -> WebCommandResponse {
    WebCommandResponse {
        id,
        ok: true,
        payload: serde_json::to_value(payload).unwrap_or(serde_json::Value::Null),
        error: None,
    }
}

fn result<T: Serialize>(id: String, result: Result<T, String>) -> WebCommandResponse {
    match result {
        Ok(value) => ok(id, value),
        Err(error) => err(id, error),
    }
}

fn err(id: String, error: String) -> WebCommandResponse {
    WebCommandResponse {
        id,
        ok: false,
        payload: serde_json::Value::Null,
        error: Some(error),
    }
}

fn command_response(response: WebCommandResponse) -> HttpResponse {
    let status = if response.ok {
        "200 OK"
    } else {
        "403 Forbidden"
    };
    json_response(status, response)
}

fn serve_static_asset(path: &str) -> Option<HttpResponse> {
    let asset_path = sanitize_asset_path(path)?;
    for root in web_asset_roots() {
        let file_path = root.join(&asset_path);
        if file_path.is_file() {
            let body = fs::read(&file_path).ok()?;
            return Some(HttpResponse {
                status: "200 OK",
                content_type: content_type_for_path(&file_path),
                body,
            });
        }
    }

    if path == "/" || !asset_path.to_string_lossy().contains('.') {
        if let Some(index) = read_index_asset() {
            return Some(index);
        }
        return Some(fallback_dashboard_shell());
    }

    None
}

fn read_index_asset() -> Option<HttpResponse> {
    for root in web_asset_roots() {
        let file_path = root.join("index.html");
        if file_path.is_file() {
            let body = fs::read(file_path).ok()?;
            return Some(HttpResponse {
                status: "200 OK",
                content_type: "text/html; charset=utf-8",
                body,
            });
        }
    }
    None
}

fn fallback_dashboard_shell() -> HttpResponse {
    let body = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>DiskPulse Web Dashboard</title>
    <style>
      body { margin: 0; font-family: Segoe UI, sans-serif; background: #101820; color: #f4fbff; }
      main { min-height: 100vh; display: grid; place-items: center; padding: 32px; }
      section { max-width: 760px; border: 1px solid rgba(255,255,255,.14); border-radius: 28px; padding: 32px; background: linear-gradient(135deg, rgba(42,111,151,.28), rgba(16,24,32,.94)); box-shadow: 0 24px 80px rgba(0,0,0,.35); }
      h1 { margin: 0 0 12px; font-size: clamp(32px, 7vw, 68px); line-height: .95; }
      p { color: #cfe8f3; font-size: 17px; line-height: 1.6; }
      code { color: #9be7ff; }
    </style>
  </head>
  <body>
    <main>
      <section>
        <h1>DiskPulse Web Dashboard</h1>
        <p>The embedded localhost server is running in read-only mode. Build the React web bundle with <code>npm run build:webui</code> to serve the full shared dashboard from <code>dist-web-dashboard</code>.</p>
      </section>
    </main>
  </body>
</html>"#;
    HttpResponse {
        status: "200 OK",
        content_type: "text/html; charset=utf-8",
        body: body.as_bytes().to_vec(),
    }
}

fn web_asset_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.join("dist-web-dashboard"));
        roots.push(current.join("dist"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.join("dist-web-dashboard"));
            roots.push(dir.join("dist"));
        }
    }
    roots
}

fn sanitize_asset_path(path: &str) -> Option<PathBuf> {
    let decoded = percent_decode(path.split('?').next().unwrap_or(path));
    let requested = if decoded == "/" || decoded.is_empty() {
        "index.html".to_string()
    } else {
        decoded.trim_start_matches('/').replace('\\', "/")
    };

    let mut clean = PathBuf::new();
    for component in Path::new(&requested).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        clean.push("index.html");
    }
    Some(clean)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "css" => "text/css; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|e| format!("Failed to configure request timeout: {e}"))?;

    let mut data = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                data.extend_from_slice(&buffer[..read]);
                if request_complete(&data) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => return Err(format!("Failed to read request: {error}")),
        }
    }

    let request = String::from_utf8_lossy(&data);
    let header_end = request
        .find("\r\n\r\n")
        .ok_or_else(|| "Missing HTTP headers".to_string())?;
    let headers = &request[..header_end];
    let mut lines = headers.lines();
    let first = lines
        .next()
        .ok_or_else(|| "Missing HTTP request line".to_string())?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let body = request[header_end + 4..].to_string();

    Ok(HttpRequest { method, path, body })
}

fn request_complete(data: &[u8]) -> bool {
    let Some(header_end) = data.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&data[..header_end]);
    let content_length = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    });
    match content_length {
        Some(len) => data.len() >= header_end + 4 + len,
        None => true,
    }
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: http://127.0.0.1\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&response.body)
}

fn json_response<T: Serialize>(status: &'static str, value: T) -> HttpResponse {
    HttpResponse {
        status,
        content_type: "application/json",
        body: serde_json::to_vec(&value).unwrap_or_else(|_| b"{\"error\":\"serialize\"}".to_vec()),
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&input[index + 1..index + 3], 16) {
                out.push(hex);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn free_port() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.local_addr().unwrap().port()
    }

    fn request(port: u16, raw: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn web_dashboard_rejects_zero_port() {
        assert!(WebDashboardRuntime::start(0).is_err());
    }

    #[test]
    fn web_dashboard_serves_health_on_localhost() {
        let port = free_port();
        let runtime = WebDashboardRuntime::start(port).unwrap();
        let response = request(port, "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("\"service\":\"diskpulse-web\""));
        runtime.stop();
    }

    #[test]
    fn web_dashboard_routes_read_only_rest_commands() {
        let response = handle_web_command("ping", json!({}));

        assert!(response.ok);
        assert_eq!(response.payload, json!({ "message": "pong" }));
    }

    #[test]
    fn web_dashboard_blocks_cleanup_without_desktop_confirmation() {
        let response = handle_web_command("clean_items", json!({ "items": [] }));

        assert!(!response.ok);
        assert!(response
            .error
            .as_deref()
            .is_some_and(|error| error.contains("requires desktop confirmation")));
    }

    #[test]
    fn web_dashboard_exposes_websocket_route_placeholder() {
        let port = free_port();
        let runtime = WebDashboardRuntime::start(port).unwrap();
        let response = request(port, "GET /ws HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

        assert!(response.starts_with("HTTP/1.1 501 Not Implemented"));
        assert!(response.contains("WebSocket proxy"));
        runtime.stop();
    }

    #[test]
    fn web_dashboard_allows_only_read_only_commands() {
        assert!(is_web_read_only_command("scan_meta"));
        assert!(is_web_read_only_command("get_disk_health"));
        assert!(is_web_read_only_command("list_drives"));
        assert!(!is_web_read_only_command("clean_items"));
    }

    #[test]
    fn web_dashboard_sanitizes_static_paths() {
        assert_eq!(
            sanitize_asset_path("/assets/app.js").unwrap(),
            PathBuf::from("assets").join("app.js")
        );
        assert!(sanitize_asset_path("/../secrets.txt").is_none());
    }

    #[test]
    fn web_dashboard_serves_fallback_shell_when_build_is_missing() {
        let response = serve_static_asset("/").unwrap();

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(String::from_utf8(response.body)
            .unwrap()
            .contains("DiskPulse Web Dashboard"));
    }

    #[test]
    fn web_dashboard_module_tracks_start_stop_status() {
        let port = free_port();
        let status = start(port).unwrap();

        assert!(status.running);
        assert_eq!(status.port, Some(port));
        assert_eq!(
            status.url.as_deref(),
            Some(format!("http://127.0.0.1:{port}").as_str())
        );
        assert!(get_status().running);

        let status = stop().unwrap();
        assert!(!status.running);
        assert!(status.url.is_none());
    }
}
