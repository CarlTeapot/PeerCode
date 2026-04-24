use std::time::Duration;

use serde::Deserialize;

const RAW_CONFIG: &str = include_str!("../peercode.config.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub websocket: WebsocketConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub show_gateway_logs: bool,
    pub show_cloudflared_logs: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebsocketConfig {
    pub connect_timeout_ms: u64,
}

impl WebsocketConfig {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }
}

impl AppConfig {
    pub fn load() -> Self {
        toml::from_str(RAW_CONFIG).expect("invalid peercode.config.toml")
    }
}
