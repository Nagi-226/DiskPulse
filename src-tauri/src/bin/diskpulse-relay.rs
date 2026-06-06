use diskpulse_lib::relay::RelayRuntime;
use std::time::Duration;

fn main() {
    let port = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("DISKPULSE_RELAY_PORT").ok())
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(19741);

    let Ok(_runtime) = RelayRuntime::start(port) else {
        eprintln!("failed to start diskpulse relay on 127.0.0.1:{port}");
        std::process::exit(1);
    };

    println!("diskpulse relay listening on ws://127.0.0.1:{port}");
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
