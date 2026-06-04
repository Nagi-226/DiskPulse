use super::router::{handle_request, HubRequest, HubResponse};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug)]
pub struct HubRuntime {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl HubRuntime {
    pub fn start(port: u16) -> Result<Self, String> {
        if port == 0 {
            return Err("Hub port must be greater than 0".into());
        }

        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("diskpulse-hub-ws".into())
            .spawn(move || run_ws_server(port, worker_stop))
            .map_err(|e| format!("Failed to spawn hub server: {e}"))?;

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

fn run_ws_server(port: u16, stop: Arc<AtomicBool>) {
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
                tokio::spawn(handle_connection(stream));
            }
        }
    });
}

async fn handle_connection(stream: tokio::net::TcpStream) {
    let Ok(mut ws) = tokio_tungstenite::accept_async(stream).await else {
        return;
    };

    while let Some(message) = ws.next().await {
        let response = match message {
            Ok(Message::Text(text)) => process_text_request(&text),
            Ok(Message::Binary(bytes)) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => process_text_request(&text),
                Err(e) => HubResponse {
                    id: String::new(),
                    ok: false,
                    payload: serde_json::Value::Null,
                    error: Some(format!("Invalid UTF-8 request: {e}")),
                },
            },
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => HubResponse {
                id: String::new(),
                ok: false,
                payload: serde_json::Value::Null,
                error: Some(format!("WebSocket error: {e}")),
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

fn process_text_request(text: &str) -> HubResponse {
    match serde_json::from_str::<HubRequest>(text) {
        Ok(request) => handle_request(request),
        Err(e) => HubResponse {
            id: String::new(),
            ok: false,
            payload: serde_json::Value::Null,
            error: Some(format!("Invalid hub request JSON: {e}")),
        },
    }
}
