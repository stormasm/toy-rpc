use async_trait::async_trait;
use futures::{Sink, SinkExt, Stream, StreamExt};
use cfg_if::cfg_if;
use futures::stream::{SplitSink, SplitStream};
use tungstenite::Message as WsMessage;

use std::marker::PhantomData;

use super::{PayloadRead, PayloadWrite};
use crate::{GracefulShutdown, error::Error};

cfg_if!{
    if #[cfg(feature = "http_tide")] {
        pub(crate) struct CannotSink {}
        mod tide_ws;
    } else if #[cfg(feature = "http_warp")] {
        mod warp_ws;
    }
}
pub(crate) struct CanSink {}

// #[pin_project]
pub struct WebSocketConn<S, N> {
    // #[pin]
    pub inner: S,
    can_sink: PhantomData<N>,
}

pub struct StreamHalf<S, Mode> {
    inner: S,
    can_sink: PhantomData<Mode>,
}

pub struct SinkHalf<S, Mode> {
    inner: S,
    can_sink: PhantomData<Mode>,
}


impl<S, E> WebSocketConn<S, CanSink>
where
    S: Stream<Item = Result<WsMessage, E>> + Sink<WsMessage> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            can_sink: PhantomData,
        }
    }

    pub fn split(
        self,
    ) -> (
        SinkHalf<SplitSink<S, WsMessage>, CanSink>,
        StreamHalf<SplitStream<S>, CanSink>,
    ) {
        let (writer, reader) = self.inner.split();

        let readhalf = StreamHalf {
            inner: reader,
            can_sink: PhantomData,
        };
        let writehalf = SinkHalf {
            inner: writer,
            can_sink: PhantomData,
        };
        (writehalf, readhalf)
    }
}

#[async_trait]
impl<S, E> PayloadRead for StreamHalf<S, CanSink>
where
    S: Stream<Item = Result<WsMessage, E>> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn read_payload(&mut self) -> Option<Result<Vec<u8>, Error>> {
        match self.inner.next().await? {
            Err(e) => return Some(Err(Error::TransportError { msg: e.to_string() })),
            Ok(msg) => {
                if let WsMessage::Binary(bytes) = msg {
                    return Some(Ok(bytes));
                } else if let WsMessage::Close(_) = msg {
                    return None;
                }

                Some(Err(Error::TransportError {
                    msg: "Expecting WebSocket::Message::Binary, but found something else"
                        .to_string(),
                }))
            }
        }
    }
}

#[async_trait]
impl<S, E> PayloadWrite for SinkHalf<S, CanSink>
where
    S: Sink<WsMessage, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), Error> {
        let msg = WsMessage::Binary(payload.into());

        self.inner
            .send(msg)
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() })
    }
}

#[async_trait]
impl<S, E> GracefulShutdown for SinkHalf<S, CanSink>
where 
    S: Sink<WsMessage, Error = E> + Send + Sync + Unpin,
    E: std::error::Error + 'static,
{
    async fn close(&mut self) {
        let msg = WsMessage::Close(None);

        match self.inner
            .send(msg)
            .await
            .map_err(|e| Error::TransportError { msg: e.to_string() }) {
                Ok(()) => { },
                Err(e) => log::error!("Error closing WebSocket {}", e.to_string()),
            };
    }
}