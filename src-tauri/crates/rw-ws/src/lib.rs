#![deny(missing_debug_implementations)]

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("connect failed: {0}")]
    Connect(String),
    #[error("send failed: {0}")]
    Send(String),
    #[error("recv failed: {0}")]
    Recv(String),
    #[error("connection closed")]
    Closed,
    #[error("connect timeout")]
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsMsg {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(not(target_family = "wasm"))]
pub use native::WsConnection;

#[cfg(target_family = "wasm")]
mod wasm;
#[cfg(target_family = "wasm")]
pub use wasm::WsConnection;

pub async fn connect(
    url: &str,
    timeout: Duration,
    subprotocols: &[&str],
) -> Result<WsConnection, WsError> {
    #[cfg(not(target_family = "wasm"))]
    {
        native::connect(url, timeout, subprotocols).await
    }
    #[cfg(target_family = "wasm")]
    {
        wasm::connect(url, timeout, subprotocols).await
    }
}
