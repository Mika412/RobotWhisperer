use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
pub struct IngestHub {
    sender: broadcast::Sender<Arc<Vec<u8>>>,
    port: Arc<std::sync::OnceLock<u16>>,
}

impl std::fmt::Debug for IngestHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngestHub")
            .field("port", &self.port.get())
            .field("receivers", &self.sender.receiver_count())
            .finish()
    }
}

impl IngestHub {
    pub fn port(&self) -> Option<u16> {
        self.port.get().copied()
    }

    pub fn send(&self, frame: Vec<u8>) {
        let _ = self.sender.send(Arc::new(frame));
    }
}

pub fn start() -> IngestHub {
    let (sender, _rx) = broadcast::channel::<Arc<Vec<u8>>>(2048);
    let port = Arc::new(std::sync::OnceLock::new());
    let hub = IngestHub {
        sender: sender.clone(),
        port: port.clone(),
    };

    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
            Ok(l) => l,
            Err(err) => {
                tracing::error!(?err, "ingest_ws: failed to bind loopback listener");
                return;
            }
        };
        match listener.local_addr() {
            Ok(addr) => {
                let _ = port.set(addr.port());
                tracing::info!(port = addr.port(), "ingest_ws: listening on loopback");
            }
            Err(err) => {
                tracing::error!(?err, "ingest_ws: local_addr failed");
                return;
            }
        }

        loop {
            let (stream, _peer) = match listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    tracing::warn!(?err, "ingest_ws: accept failed");
                    continue;
                }
            };
            let rx = sender.subscribe();
            tauri::async_runtime::spawn(handle_client(stream, rx));
        }
    });

    hub
}

async fn handle_client(stream: tokio::net::TcpStream, mut rx: broadcast::Receiver<Arc<Vec<u8>>>) {
    let _ = stream.set_nodelay(true);
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(err) => {
            tracing::warn!(?err, "ingest_ws: handshake failed");
            return;
        }
    };
    tracing::info!("ingest_ws: worker connected");
    let (mut write, mut read) = ws.split();

    loop {
        tokio::select! {
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            frame = rx.recv() => {
                match frame {
                    Ok(bytes) => {
                        if write.send(Message::Binary(bytes.as_ref().clone())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::debug!(lagged = n, "ingest_ws: client lagged, frames dropped");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    tracing::info!("ingest_ws: worker disconnected");
}
