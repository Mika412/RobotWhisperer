use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{WsError, WsMsg};

type Inner = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug)]
pub struct WsConnection {
    inner: Inner,
    subprotocol: Option<String>,
    closed: bool,
}

impl WsConnection {
    pub fn selected_subprotocol(&self) -> Option<&str> {
        self.subprotocol.as_deref()
    }

    pub async fn send(&mut self, msg: WsMsg) -> Result<(), WsError> {
        if self.closed {
            return Err(WsError::Closed);
        }
        let wire = match msg {
            WsMsg::Text(s) => WsMessage::Text(s),
            WsMsg::Binary(b) => WsMessage::Binary(b),
            WsMsg::Close => WsMessage::Close(None),
        };
        self.inner
            .send(wire)
            .await
            .map_err(|err| WsError::Send(err.to_string()))
    }

    pub async fn next(&mut self) -> Option<Result<WsMsg, WsError>> {
        if self.closed {
            return None;
        }
        loop {
            match self.inner.next().await {
                None => {
                    self.closed = true;
                    return Some(Ok(WsMsg::Close));
                }
                Some(Ok(WsMessage::Text(s))) => return Some(Ok(WsMsg::Text(s))),
                Some(Ok(WsMessage::Binary(b))) => return Some(Ok(WsMsg::Binary(b))),
                Some(Ok(WsMessage::Close(_))) => {
                    self.closed = true;
                    return Some(Ok(WsMsg::Close));
                }
                Some(Ok(WsMessage::Ping(_)))
                | Some(Ok(WsMessage::Pong(_)))
                | Some(Ok(WsMessage::Frame(_))) => continue,
                Some(Err(err)) => {
                    self.closed = true;
                    return Some(Err(WsError::Recv(err.to_string())));
                }
            }
        }
    }

    pub async fn close(&mut self) {
        let _ = self.inner.close(None).await;
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while let Some(msg) = self.inner.next().await {
                match msg {
                    Err(_) => break,
                    _ => continue,
                }
            }
        })
        .await;
        self.closed = true;
    }
}

pub(crate) async fn connect(
    url: &str,
    timeout_dur: Duration,
    subprotocols: &[&str],
) -> Result<WsConnection, WsError> {
    let parsed = url::Url::parse(url).map_err(|err| WsError::InvalidUrl(err.to_string()))?;
    let mut request = parsed
        .as_str()
        .into_client_request()
        .map_err(|err| WsError::Connect(format!("ws request build: {err}")))?;
    if !subprotocols.is_empty() {
        let joined = subprotocols.join(",");
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str(&joined)
                .map_err(|err| WsError::Connect(format!("subprotocols header: {err}")))?,
        );
        if !request.headers().contains_key("Sec-WebSocket-Key") {
            request.headers_mut().insert(
                "Sec-WebSocket-Key",
                HeaderValue::from_str(&generate_key()).expect("generated ws key is ascii"),
            );
        }
    }
    let (inner, response) = timeout(timeout_dur, tokio_tungstenite::connect_async(request))
        .await
        .map_err(|_| WsError::Timeout)?
        .map_err(|err| WsError::Connect(err.to_string()))?;
    let subprotocol = response
        .headers()
        .get("Sec-WebSocket-Protocol")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    Ok(WsConnection {
        inner,
        subprotocol,
        closed: false,
    })
}
