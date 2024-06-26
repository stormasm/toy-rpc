//! Broker on the server side

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::protocol::InboundBody;
use crate::pubsub::SeqId;
use crate::service::{ArcAsyncServiceCall, HandlerResult};

use crate::{error::Error, message::MessageId};

use std::collections::HashMap;
use std::marker::PhantomData;

use brw::{Broker, Running};
use flume::Sender;
use futures::sink::{Sink, SinkExt};

use crate::pubsub::{AckModeAuto, AckModeNone};
use crate::server::pubsub::PubSubResponder;

use super::pubsub::PubSubItem;
use super::writer::ServerWriterItem;
use super::ClientId;

#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
use ::async_std::task::JoinHandle;
#[cfg(any(
    feature = "docs",
    all(feature = "tokio_runtime", not(feature = "async_std_runtime"),)
))]
use ::tokio::task::JoinHandle;

pub(crate) enum ServerBrokerItem {
    Request {
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        duration: Duration,
        deserializer: Box<InboundBody>,
    },
    Response {
        id: MessageId,
        result: HandlerResult,
    },
    Cancel(MessageId),
    // A new publish from the client publisher
    Publish {
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    },
    // A new subscribe from the client subscriber
    Subscribe {
        id: MessageId,
        topic: String,
    },
    Unsubscribe {
        id: MessageId,
        topic: String,
    },
    // A publication message to the client subscriber
    Publication {
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    },
    // The server broker should only receive Ack from the client
    InboundAck {
        seq_id: SeqId,
    },
    Stopping,
    Stop,
}

pub(crate) struct ServerBroker<AckMode> {
    pub client_id: ClientId,
    pub executions: HashMap<MessageId, JoinHandle<()>>,
    pub pubsub_broker: Sender<PubSubItem>,

    ack_mode: PhantomData<AckMode>,
}

impl<AckMode> ServerBroker<AckMode> {
    pub fn new(client_id: ClientId, pubsub_broker: Sender<PubSubItem>) -> Self {
        Self {
            client_id,
            executions: HashMap::new(),
            pubsub_broker,
            ack_mode: PhantomData,
        }
    }

    fn handle_request<'a>(
        &'a mut self,
        ctx: &'a Arc<brw::Context<ServerBrokerItem>>,
        call: ArcAsyncServiceCall,
        id: MessageId,
        method: String,
        duration: Duration,
        deserializer: Box<InboundBody>,
    ) -> Result<(), Error> {
        let fut = call(method, deserializer);
        let _broker = ctx.broker.clone();
        let handle = spawn_timed_request_execution(_broker, duration, id, fut);
        self.executions.insert(id, handle);
        Ok(())
    }

    async fn handle_response<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        id: MessageId,
        result: HandlerResult,
    ) -> Result<(), Error>
    where
        W: Sink<ServerWriterItem, Error = flume::SendError<ServerWriterItem>> + Send + Unpin,
    {
        self.executions.remove(&id);
        let msg = ServerWriterItem::Response { id, result };
        writer.send(msg).await.map_err(|err| err.into())
    }

    async fn handle_cancel(&mut self, id: MessageId) -> Result<(), Error> {
        if let Some(handle) = self.executions.remove(&id) {
            #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
            handle.abort();
            #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
            handle.cancel().await;
        }
        Ok(())
    }

    async fn handle_publish_inner(
        &mut self,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error> {
        let content = Arc::new(content);
        let msg = PubSubItem::Publish {
            client_id: self.client_id,
            msg_id: id,
            topic,
            content,
        };
        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_subscribe<'a>(
        &'a mut self,
        ctx: &'a Arc<brw::Context<ServerBrokerItem>>,
        id: MessageId,
        topic: String,
    ) -> Result<(), Error> {
        log::debug!("Message ID: {}, Subscribe to topic: {}", &id, &topic);
        let sender = PubSubResponder::Sender(ctx.broker.clone());
        let msg = PubSubItem::Subscribe {
            client_id: self.client_id,
            topic,
            sender,
        };

        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_unsubscribe(&mut self, id: MessageId, topic: String) -> Result<(), Error> {
        log::debug!("Message ID: {}, Unsubscribe from topic: {}", &id, &topic);
        let msg = PubSubItem::Unsubscribe {
            client_id: self.client_id,
            topic,
        };

        self.pubsub_broker
            .send_async(msg)
            .await
            .map_err(|err| err.into())
    }

    async fn handle_publication<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        seq_id: SeqId,
        topic: String,
        content: Arc<Vec<u8>>,
    ) -> Result<(), Error>
    where
        W: Sink<ServerWriterItem, Error = flume::SendError<ServerWriterItem>> + Send + Unpin,
    {
        // Publication is the PubSub message from server to client
        let msg = ServerWriterItem::Publication {
            seq_id,
            topic,
            content,
        };
        writer.send(msg).await.map_err(|err| err.into())
    }

    async fn handle_inbound_ack(&mut self, seq_id: SeqId) -> Result<(), Error> {
        let item = PubSubItem::Ack {
            seq_id,
            client_id: self.client_id,
        };
        self.pubsub_broker
            .send_async(item)
            .await
            .map_err(|err| err.into())
    }
}

impl ServerBroker<AckModeNone> {
    // Publish is the PubSub message from client to server
    async fn handle_publish<'w, W>(
        &'w mut self,
        _: &'w mut W,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error>
    where
        W: Sink<ServerWriterItem, Error = flume::SendError<ServerWriterItem>> + Send + Unpin,
    {
        self.handle_publish_inner(id, topic, content).await
    }
}

impl ServerBroker<AckModeAuto> {
    async fn auto_ack<'w, W>(&'w self, writer: &'w mut W, id: MessageId) -> Result<(), Error>
    where
        W: Sink<ServerWriterItem, Error = flume::SendError<ServerWriterItem>> + Send + Unpin,
    {
        writer
            .send(ServerWriterItem::Ack { id })
            .await
            .map_err(|err| err.into())
    }

    // Publish is the PubSub message from client to server
    async fn handle_publish<'w, W>(
        &'w mut self,
        writer: &'w mut W,
        id: MessageId,
        topic: String,
        content: Vec<u8>,
    ) -> Result<(), Error>
    where
        W: Sink<ServerWriterItem, Error = flume::SendError<ServerWriterItem>> + Send + Unpin,
    {
        self.handle_publish_inner(id, topic, content).await?;
        self.auto_ack(writer, id).await
    }
}

macro_rules! impl_server_broker_for_ack_modes {
    ($($ack_mode:ty),*) => {
        $(
            #[async_trait::async_trait]
            impl Broker for ServerBroker<$ack_mode> {
                type Item = ServerBrokerItem;
                type WriterItem = ServerWriterItem;
                type Ok = ();
                type Error = Error;

                async fn op<W>(
                    &mut self,
                    ctx: &Arc<brw::Context<Self::Item>>,
                    item: Self::Item,
                    mut writer: W,
                ) -> Running<Result<Self::Ok, Self::Error>, Option<Self::Error>>
                where
                    W: Sink<Self::WriterItem, Error = flume::SendError<Self::WriterItem>> + Send + Unpin,
                {
                    let result = match item {
                        ServerBrokerItem::Request {
                            call,
                            id,
                            method,
                            duration,
                            deserializer,
                        } => {
                            self.handle_request(ctx, call, id, method, duration, deserializer)
                        },
                        ServerBrokerItem::Response { id, result } => {
                           self.handle_response(&mut writer, id, result).await
                        },
                        ServerBrokerItem::Cancel(id) => {
                            self.handle_cancel(id).await
                        },
                        ServerBrokerItem::Publish { id, topic, content } => {
                            self.handle_publish(&mut writer, id, topic, content).await
                        },
                        ServerBrokerItem::Subscribe { id, topic } => {
                            self.handle_subscribe(ctx, id, topic).await
                        },
                        ServerBrokerItem::Unsubscribe { id, topic } => {
                            self.handle_unsubscribe(id, topic).await
                        },
                        ServerBrokerItem::Publication { seq_id, topic, content } => {
                            self.handle_publication(&mut writer, seq_id, topic, content).await
                        },
                        ServerBrokerItem::InboundAck {seq_id} => {
                            self.handle_inbound_ack(seq_id).await
                        },
                        ServerBrokerItem::Stopping => {
                            for (_, handle) in self.executions.drain() {
                                log::debug!("Stopping execution as client is disconnected");
                                #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime")))]
                                handle.abort();
                                #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
                                handle.cancel().await;
                            }

                            let result = writer.send(ServerWriterItem::Stopping).await
                                .map_err(Into::into);

                            ctx.broker.send_async(ServerBrokerItem::Stop).await
                                .map_err(Into::into)
                                .and(result)
                        }
                        ServerBrokerItem::Stop => {
                            if let Err(err) = writer.send(ServerWriterItem::Stop).await {
                                log::debug!("{}", err);
                            }
                            log::debug!("Client connection is closed");
                            return Running::Stop(None)
                        }
                    };

                    Running::Continue(result)
                }
            }
        )*
    };
}

impl_server_broker_for_ack_modes!(AckModeNone, AckModeAuto);

/// Spawn the execution in a async_std task and return the JoinHandle
#[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
fn spawn_timed_request_execution(
    broker: Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output = HandlerResult> + Send + 'static,
) -> ::async_std::task::JoinHandle<()> {
    ::async_std::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        broker
            .send_async(ServerBrokerItem::Response { id, result })
            .await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

/// Spawn the execution in a tokio task and return the JoinHandle
#[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime"),))]
fn spawn_timed_request_execution(
    broker: Sender<ServerBrokerItem>,
    duration: Duration,
    id: MessageId,
    fut: impl Future<Output = HandlerResult> + Send + 'static,
) -> ::tokio::task::JoinHandle<()> {
    ::tokio::task::spawn(async move {
        let result = execute_timed_call(id, duration, fut).await;
        broker
            .send_async(ServerBrokerItem::Response { id, result })
            .await
            .unwrap_or_else(|e| log::error!("{}", e));
    })
}

pub(crate) async fn execute_call(
    id: MessageId,
    fut: impl Future<Output = HandlerResult>,
) -> HandlerResult {
    let result: HandlerResult = fut.await.map_err(|err| {
        log::error!(
            "Error found executing request id: {}, error msg: {}",
            &id,
            &err
        );
        match err {
            // if serde cannot parse request, the argument is likely mistaken
            Error::ParseError(e) => {
                log::error!("ParseError {:?}", e);
                Error::InvalidArgument
            }
            e => e,
        }
    });
    result
}

pub(crate) async fn execute_timed_call(
    id: MessageId,
    duration: Duration,
    fut: impl Future<Output = HandlerResult>,
) -> HandlerResult {
    #[cfg(all(feature = "async_std_runtime", not(feature = "tokio_runtime")))]
    match ::async_std::future::timeout(duration, execute_call(id, fut)).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Request {} reached timeout (err: {})", id, err);
            Err(Error::Timeout(id))
        }
    }

    #[cfg(all(feature = "tokio_runtime", not(feature = "async_std_runtime"),))]
    match ::tokio::time::timeout(duration, execute_call(id, fut)).await {
        Ok(res) => res,
        Err(err) => {
            log::error!("Request {} reached timeout (err: {})", id, err);
            Err(Error::Timeout(id))
        }
    }
}
