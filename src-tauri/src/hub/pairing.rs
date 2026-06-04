use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingToken {
    pub code: String,
    pub device_name: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Default)]
pub struct PairingManager {
    tokens: HashMap<String, PairingToken>,
}

impl PairingManager {
    pub fn create_token(&mut self, device_name: String, ttl_seconds: u64) -> PairingToken {
        self.prune_expired();
        let now = now_epoch_ms();
        let code = self.next_unique_code(now);
        let token = PairingToken {
            code: code.clone(),
            device_name,
            expires_at_epoch_ms: now.saturating_add(ttl_seconds.saturating_mul(1000)),
        };
        self.tokens.insert(code, token.clone());
        token
    }

    pub fn consume_token(&mut self, code: &str) -> Option<PairingToken> {
        let token = self.tokens.remove(code)?;
        if token.expires_at_epoch_ms <= now_epoch_ms() {
            return None;
        }
        Some(token)
    }

    fn next_unique_code(&self, now: u64) -> String {
        for attempt in 0..1_000_000_u64 {
            let counter = TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
            let raw = now
                .wrapping_add(counter.wrapping_mul(37))
                .wrapping_add(attempt.wrapping_mul(10_007));
            let code = format!("{:06}", raw % 1_000_000);
            if !self.tokens.contains_key(&code) {
                return code;
            }
        }
        "000000".into()
    }

    fn prune_expired(&mut self) {
        let now = now_epoch_ms();
        self.tokens
            .retain(|_, token| token.expires_at_epoch_ms > now);
    }
}

pub fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
