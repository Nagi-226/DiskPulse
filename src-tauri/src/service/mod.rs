use serde::{Deserialize, Serialize};

pub const SERVICE_NAME: &str = "DiskPulseService";
pub const SERVICE_DISPLAY_NAME: &str = "DiskPulse Background Service";
pub const SERVICE_PIPE_NAME: &str = r"\\.\pipe\DiskPulseService";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    NotInstalled,
    Stopped,
    StartPending,
    Running,
    StopPending,
    Paused,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub installed: bool,
    pub state: ServiceState,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRequest {
    pub command: String,
    #[serde(default)]
    pub drive: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub ok: bool,
    pub message: String,
}

pub fn service_binary_path(exe_path: &str) -> String {
    format!("\"{}\" --service", exe_path)
}

pub fn state_from_raw(raw: u32) -> ServiceState {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::Services::{
            SERVICE_PAUSED, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STOPPED,
            SERVICE_STOP_PENDING,
        };
        if raw == SERVICE_STOPPED.0 {
            return ServiceState::Stopped;
        }
        if raw == SERVICE_START_PENDING.0 {
            return ServiceState::StartPending;
        }
        if raw == SERVICE_RUNNING.0 {
            return ServiceState::Running;
        }
        if raw == SERVICE_STOP_PENDING.0 {
            return ServiceState::StopPending;
        }
        if raw == SERVICE_PAUSED.0 {
            return ServiceState::Paused;
        }
    }
    let _ = raw;
    ServiceState::Unknown
}

pub fn run_service_mode() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::run_service_dispatcher().or_else(|_| run_service_worker())
    }
    #[cfg(not(target_os = "windows"))]
    {
        run_service_worker()
    }
}

pub fn handle_service_message(input: &str) -> Result<String, String> {
    let request: ServiceRequest =
        serde_json::from_str(input).map_err(|e| format!("Invalid service request JSON: {e}"))?;
    let response = match request.command.as_str() {
        "ping" => ServiceResponse {
            ok: true,
            message: "pong".into(),
        },
        "status" => status()
            .map(|status| ServiceResponse {
                ok: true,
                message: format!("{:?}", status.state),
            })
            .unwrap_or_else(|e| ServiceResponse {
                ok: false,
                message: e,
            }),
        "scan_meta" => {
            let drive = request.drive.unwrap_or_else(|| "C".into());
            match crate::scanner::scan_drive_meta(&drive, None, None) {
                Ok(meta) => ServiceResponse {
                    ok: true,
                    message: format!("{}:{}", meta.drive_letter, meta.used_bytes),
                },
                Err(e) => ServiceResponse {
                    ok: false,
                    message: e,
                },
            }
        }
        command if command.contains("clean") || command.contains("cleanup") => ServiceResponse {
            ok: false,
            message: "Cleanup commands are not allowed in service mode".into(),
        },
        other => ServiceResponse {
            ok: false,
            message: format!("Unsupported service command: {other}"),
        },
    };
    serde_json::to_string(&response).map_err(|e| format!("Service response JSON error: {e}"))
}

pub fn run_service_worker() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::start_named_pipe_server();
    }
    start_background_engine();
    Ok(())
}

fn start_background_engine() {
    std::thread::spawn(|| loop {
        if service_should_stop() {
            break;
        }
        let settings = crate::db::get_settings().unwrap_or_default();
        if let Ok(info) = crate::scanner::scan_drive(&settings.default_drive) {
            let _ = crate::db::save_snapshot(&info);
        }
        for _ in 0..3600 {
            if service_should_stop() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

#[cfg(target_os = "windows")]
fn service_should_stop() -> bool {
    windows_impl::service_should_stop()
}

#[cfg(not(target_os = "windows"))]
fn service_should_stop() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn install() -> Result<ServiceStatus, String> {
    windows_impl::install()
}

#[cfg(not(target_os = "windows"))]
pub fn install() -> Result<ServiceStatus, String> {
    Err("Windows Service mode is only supported on Windows".into())
}

#[cfg(target_os = "windows")]
pub fn uninstall() -> Result<ServiceStatus, String> {
    windows_impl::uninstall()
}

#[cfg(not(target_os = "windows"))]
pub fn uninstall() -> Result<ServiceStatus, String> {
    Err("Windows Service mode is only supported on Windows".into())
}

#[cfg(target_os = "windows")]
pub fn start() -> Result<ServiceStatus, String> {
    windows_impl::start()
}

#[cfg(not(target_os = "windows"))]
pub fn start() -> Result<ServiceStatus, String> {
    Err("Windows Service mode is only supported on Windows".into())
}

#[cfg(target_os = "windows")]
pub fn stop() -> Result<ServiceStatus, String> {
    windows_impl::stop()
}

#[cfg(not(target_os = "windows"))]
pub fn stop() -> Result<ServiceStatus, String> {
    Err("Windows Service mode is only supported on Windows".into())
}

#[cfg(target_os = "windows")]
pub fn status() -> Result<ServiceStatus, String> {
    let mut status = windows_impl::status()?;
    if status.state == ServiceState::Running {
        if let Ok(response) = send_request(&ServiceRequest {
            command: "ping".into(),
            drive: None,
        }) {
            status.message = format!("{}; pipe: {}", status.message, response.message);
        }
    }
    Ok(status)
}

#[cfg(not(target_os = "windows"))]
pub fn status() -> Result<ServiceStatus, String> {
    Ok(ServiceStatus {
        installed: false,
        state: ServiceState::NotInstalled,
        message: "Windows Service mode is only supported on Windows".into(),
    })
}

#[cfg(target_os = "windows")]
pub fn send_request(request: &ServiceRequest) -> Result<ServiceResponse, String> {
    windows_impl::send_pipe_request(request)
}

#[cfg(not(target_os = "windows"))]
pub fn send_request(_request: &ServiceRequest) -> Result<ServiceResponse, String> {
    Err("Named Pipe service client is only supported on Windows".into())
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::mem::size_of;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, OnceLock};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Storage::FileSystem::{ReadFile, WriteFile, PIPE_ACCESS_DUPLEX};
    use windows::Win32::System::Pipes::{
        CallNamedPipeW, ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
        PIPE_READMODE_MESSAGE, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows::Win32::System::Services::{
        CloseServiceHandle, ControlService, CreateServiceW, DeleteService, OpenSCManagerW,
        OpenServiceW, QueryServiceStatusEx, RegisterServiceCtrlHandlerW, SetServiceStatus,
        StartServiceCtrlDispatcherW, StartServiceW, SC_HANDLE, SC_MANAGER_CONNECT,
        SC_MANAGER_CREATE_SERVICE, SC_STATUS_PROCESS_INFO, SERVICE_ACCEPT_STOP, SERVICE_ALL_ACCESS,
        SERVICE_AUTO_START, SERVICE_CONTROL_STOP, SERVICE_ERROR_NORMAL, SERVICE_QUERY_STATUS,
        SERVICE_RUNNING, SERVICE_START, SERVICE_START_PENDING, SERVICE_STATUS,
        SERVICE_STATUS_HANDLE, SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOPPED,
        SERVICE_STOP_PENDING, SERVICE_TABLE_ENTRYW, SERVICE_WIN32_OWN_PROCESS,
    };

    static SERVICE_STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
    static SERVICE_STATUS_HANDLE_RAW: OnceLock<Mutex<usize>> = OnceLock::new();

    struct ScHandle(SC_HANDLE);

    impl Drop for ScHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseServiceHandle(self.0);
            }
        }
    }

    pub fn install() -> Result<ServiceStatus, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Cannot resolve current executable: {e}"))?;
        let binary = service_binary_path(&exe.to_string_lossy());
        let manager = open_manager(SC_MANAGER_CREATE_SERVICE)?;
        let name = wide(SERVICE_NAME);
        let display = wide(SERVICE_DISPLAY_NAME);
        let binary = wide(&binary);
        let local_service = wide("NT AUTHORITY\\LocalService");
        let service = unsafe {
            CreateServiceW(
                manager.0,
                PCWSTR(name.as_ptr()),
                PCWSTR(display.as_ptr()),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                PCWSTR(binary.as_ptr()),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR(local_service.as_ptr()),
                PCWSTR::null(),
            )
        }
        .map_err(|e| format!("CreateServiceW failed: {e}"))?;
        let _service = ScHandle(service);
        Ok(ServiceStatus {
            installed: true,
            state: ServiceState::Stopped,
            message: "service installed".into(),
        })
    }

    pub fn uninstall() -> Result<ServiceStatus, String> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(&manager, SERVICE_ALL_ACCESS)?;
        unsafe { DeleteService(service.0) }.map_err(|e| format!("DeleteService failed: {e}"))?;
        Ok(ServiceStatus {
            installed: false,
            state: ServiceState::NotInstalled,
            message: "service uninstalled".into(),
        })
    }

    pub fn start() -> Result<ServiceStatus, String> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(&manager, SERVICE_START | SERVICE_QUERY_STATUS)?;
        unsafe { StartServiceW(service.0, None) }
            .map_err(|e| format!("StartServiceW failed: {e}"))?;
        query_status(&service)
    }

    pub fn stop() -> Result<ServiceStatus, String> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = open_service(&manager, SERVICE_STOP | SERVICE_QUERY_STATUS)?;
        let mut raw = SERVICE_STATUS::default();
        unsafe { ControlService(service.0, SERVICE_CONTROL_STOP, &mut raw) }
            .map_err(|e| format!("ControlService stop failed: {e}"))?;
        query_status(&service)
    }

    pub fn status() -> Result<ServiceStatus, String> {
        let manager = open_manager(SC_MANAGER_CONNECT)?;
        let service = match open_service(&manager, SERVICE_QUERY_STATUS) {
            Ok(service) => service,
            Err(_) => {
                return Ok(ServiceStatus {
                    installed: false,
                    state: ServiceState::NotInstalled,
                    message: "service not installed".into(),
                })
            }
        };
        query_status(&service)
    }

    pub fn run_service_dispatcher() -> Result<(), String> {
        SERVICE_STOP_REQUESTED.store(false, Ordering::Relaxed);
        let mut name = wide(SERVICE_NAME);
        let table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR(name.as_mut_ptr()),
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW {
                lpServiceName: PWSTR::null(),
                lpServiceProc: None,
            },
        ];
        unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) }
            .map_err(|e| format!("StartServiceCtrlDispatcherW failed: {e}"))
    }

    pub fn service_should_stop() -> bool {
        SERVICE_STOP_REQUESTED.load(Ordering::Relaxed)
    }

    pub fn start_named_pipe_server() {
        std::thread::spawn(|| {
            while !service_should_stop() {
                if let Err(e) = serve_one_pipe_client() {
                    eprintln!("service pipe error: {e}");
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
            }
        });
    }

    pub fn send_pipe_request(request: &ServiceRequest) -> Result<ServiceResponse, String> {
        let payload =
            serde_json::to_vec(request).map_err(|e| format!("Service request JSON error: {e}"))?;
        let name = wide(SERVICE_PIPE_NAME);
        let mut output = vec![0u8; 4096];
        let mut read = 0u32;
        let ok = unsafe {
            CallNamedPipeW(
                PCWSTR(name.as_ptr()),
                Some(payload.as_ptr().cast()),
                payload.len() as u32,
                Some(output.as_mut_ptr().cast()),
                output.len() as u32,
                &mut read,
                1000,
            )
        };
        if !ok.as_bool() {
            return Err("CallNamedPipeW failed".into());
        }
        serde_json::from_slice(&output[..read as usize])
            .map_err(|e| format!("Service response JSON error: {e}"))
    }

    unsafe extern "system" fn service_main(_num_args: u32, _args: *mut PWSTR) {
        let name = wide(SERVICE_NAME);
        let handle =
            match RegisterServiceCtrlHandlerW(PCWSTR(name.as_ptr()), Some(service_control_handler))
            {
                Ok(handle) => handle,
                Err(_) => return,
            };
        *SERVICE_STATUS_HANDLE_RAW
            .get_or_init(|| Mutex::new(0))
            .lock()
            .unwrap() = handle.0 as usize;
        report_service_status(SERVICE_START_PENDING);
        start_named_pipe_server();
        start_background_engine();
        report_service_status(SERVICE_RUNNING);

        while !service_should_stop() {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        report_service_status(SERVICE_STOPPED);
    }

    unsafe extern "system" fn service_control_handler(control: u32) {
        if control == SERVICE_CONTROL_STOP {
            SERVICE_STOP_REQUESTED.store(true, Ordering::Relaxed);
            report_service_status(SERVICE_STOP_PENDING);
        }
    }

    fn report_service_status(
        state: windows::Win32::System::Services::SERVICE_STATUS_CURRENT_STATE,
    ) {
        let raw = SERVICE_STATUS_HANDLE_RAW
            .get_or_init(|| Mutex::new(0))
            .lock()
            .map(|value| *value)
            .unwrap_or(0);
        if raw == 0 {
            return;
        }
        let status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: state,
            dwControlsAccepted: if state == SERVICE_RUNNING {
                SERVICE_ACCEPT_STOP
            } else {
                0
            },
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 0,
        };
        unsafe {
            let _ = SetServiceStatus(SERVICE_STATUS_HANDLE(raw as *mut _), &status);
        }
    }

    fn serve_one_pipe_client() -> Result<(), String> {
        let name = wide(SERVICE_PIPE_NAME);
        let pipe = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                500,
                None,
            )
        };
        if pipe.is_invalid() {
            return Err("CreateNamedPipeW failed".into());
        }
        let _guard = PipeHandle(pipe);
        unsafe { ConnectNamedPipe(pipe, None) }
            .map_err(|e| format!("ConnectNamedPipe failed: {e}"))?;

        let mut buffer = vec![0u8; 4096];
        let mut read = 0u32;
        unsafe { ReadFile(pipe, Some(&mut buffer), Some(&mut read), None) }
            .map_err(|e| format!("ReadFile failed: {e}"))?;
        let request = String::from_utf8_lossy(&buffer[..read as usize]).to_string();
        let response = handle_service_message(&request)?;
        let mut written = 0u32;
        unsafe { WriteFile(pipe, Some(response.as_bytes()), Some(&mut written), None) }
            .map_err(|e| format!("WriteFile failed: {e}"))?;
        unsafe {
            let _ = DisconnectNamedPipe(pipe);
        }
        Ok(())
    }

    struct PipeHandle(HANDLE);

    impl Drop for PipeHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    fn query_status(service: &ScHandle) -> Result<ServiceStatus, String> {
        let mut status = SERVICE_STATUS_PROCESS::default();
        let mut needed = 0u32;
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                size_of::<SERVICE_STATUS_PROCESS>(),
            )
        };
        unsafe {
            QueryServiceStatusEx(service.0, SC_STATUS_PROCESS_INFO, Some(buffer), &mut needed)
        }
        .map_err(|e| format!("QueryServiceStatusEx failed: {e}"))?;
        let state = state_from_raw(status.dwCurrentState.0);
        Ok(ServiceStatus {
            installed: true,
            message: format!("service state: {:?}", state),
            state,
        })
    }

    fn open_manager(access: u32) -> Result<ScHandle, String> {
        let manager = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), access) }
            .map_err(|e| format!("OpenSCManagerW failed: {e}"))?;
        Ok(ScHandle(manager))
    }

    fn open_service(manager: &ScHandle, access: u32) -> Result<ScHandle, String> {
        let name = wide(SERVICE_NAME);
        let service = unsafe { OpenServiceW(manager.0, PCWSTR(name.as_ptr()), access) }
            .map_err(|e| format!("OpenServiceW failed: {e}"))?;
        Ok(ScHandle(service))
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_binary_path_quotes_exe_and_adds_flag() {
        assert_eq!(
            service_binary_path(r"C:\Program Files\DiskPulse\diskpulse.exe"),
            r#""C:\Program Files\DiskPulse\diskpulse.exe" --service"#
        );
    }

    #[test]
    fn unknown_raw_state_maps_to_unknown() {
        assert_eq!(state_from_raw(9999), ServiceState::Unknown);
    }

    #[test]
    fn service_rejects_cleanup_messages() {
        let response = handle_service_message(r#"{"command":"cleanup","drive":"C"}"#).unwrap();
        let response: ServiceResponse = serde_json::from_str(&response).unwrap();
        assert!(!response.ok);
        assert!(response.message.contains("not allowed"));
    }

    #[test]
    fn service_accepts_ping_messages() {
        let response = handle_service_message(r#"{"command":"ping"}"#).unwrap();
        let response: ServiceResponse = serde_json::from_str(&response).unwrap();
        assert!(response.ok);
        assert_eq!(response.message, "pong");
    }
}
