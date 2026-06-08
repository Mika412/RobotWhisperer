use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use ws_stream_wasm::{WsMessage, WsMeta, WsStream};

use crate::{WsError, WsMsg};

#[derive(Debug)]
pub struct WsConnection {
    inner: WsStream,
    meta: WsMeta,
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
            WsMsg::Close => {
                self.closed = true;
                let _ = self.meta.close().await;
                return Ok(());
            }
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
        match self.inner.next().await {
            None => {
                self.closed = true;
                Some(Ok(WsMsg::Close))
            }
            Some(WsMessage::Text(s)) => Some(Ok(WsMsg::Text(s))),
            Some(WsMessage::Binary(b)) => Some(Ok(WsMsg::Binary(b))),
        }
    }

    pub async fn close(&mut self) {
        self.closed = true;
        let _ = self.meta.close().await;
    }
}

pub(crate) async fn connect(
    url: &str,
    timeout_dur: Duration,
    subprotocols: &[&str],
) -> Result<WsConnection, WsError> {
    let protocols: Option<Vec<&str>> = if subprotocols.is_empty() {
        None
    } else {
        Some(subprotocols.to_vec())
    };
    let connect_fut = WsMeta::connect(url, protocols);
    let timeout_fut = gloo_timers::future::TimeoutFuture::new(
        timeout_dur.as_millis().min(i32::MAX as u128) as u32,
    );

    futures_util::pin_mut!(connect_fut);
    futures_util::pin_mut!(timeout_fut);
    match futures_util::future::select(connect_fut, timeout_fut).await {
        futures_util::future::Either::Left((Ok((meta, stream)), _)) => {
            let protocol = meta.protocol();
            let subprotocol = if protocol.is_empty() {
                None
            } else {
                Some(protocol)
            };
            Ok(WsConnection {
                inner: stream,
                meta,
                subprotocol,
                closed: false,
            })
        }
        futures_util::future::Either::Left((Err(err), _)) => Err(WsError::Connect(err.to_string())),
        futures_util::future::Either::Right(_) => Err(WsError::Timeout),
    }
}
