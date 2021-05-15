use std::sync::atomic::Ordering;
use ::async_std::task;
use futures::{AsyncRead, AsyncWrite};

use crate::codec::split::ClientCodecSplit;

use super::*;

/// Call of a RPC request. The result can be obtained by `.await`ing the `Call`.
/// The call can be cancelled with `cancel()` method.
///
/// # Example
///
#[pin_project]
pub struct Call<Res> {
    id: MessageId,
    cancel: oneshot::Sender<MessageId>,
    #[pin]
    done: oneshot::Receiver<Result<Res, Error>>,
    handle: task::JoinHandle<Result<(), Error>>,
}

impl<Res> Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    pub fn cancel(self) {
        let handle = self.handle;
        match self.cancel.send(self.id) {
            Ok(_) => {
                log::info!("Call is canceled");
            }
            Err(_) => {
                log::error!("Failed to cancel")
            }
        }

        match task::block_on(handle) {
            Ok(_) => { },
            Err(err) => log::error!("{:?}", err)
        };
    }
}

impl<Res> Future for Call<Res>
where
    Res: serde::de::DeserializeOwned,
{
    type Output = Result<Res, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let done: Pin<&mut oneshot::Receiver<Result<Res, Error>>> = this.done;

        match done.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(res) => match res {
                Ok(r) => Poll::Ready(r),
                Err(_canceled) => Poll::Ready(Err(Error::Canceled(Some(this.id.clone())))),
            },
        }
    }
}

cfg_if! {
    if #[cfg(any(
        any(feature = "docs", doc),
        all(
            feature = "serde_bincode",
            not(feature = "serde_json"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_cbor",
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_json",
            not(feature = "serde_bincode"),
            not(feature = "serde_cbor"),
            not(feature = "serde_rmp"),
        ),
        all(
            feature = "serde_rmp",
            not(feature = "serde_cbor"),
            not(feature = "serde_json"),
            not(feature = "serde_bincode"),
        )
    ))] {
        use ::async_std::net::{TcpStream, ToSocketAddrs};

        impl Client<NotConnected> {
            pub async fn dial(addr: impl ToSocketAddrs) 
                -> Result<Client<Connected>, Error> 
            {
                let stream = TcpStream::connect(addr).await?;
                Ok(Self::with_stream(stream))
            }

            pub fn with_stream<T>(stream: T) -> Client<Connected>
            where
                T: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
            {
                let codec = DefaultCodec::new(stream);
                Self::with_codec(codec)
            }
        }
    }
}

// // seems like it still works even without this impl
// impl<Mode, Handle: Future> Drop for Client<Mode, Handle> {
//     fn drop(&mut self) {
//         log::debug!("Dropping client");

//         if self.reader_stop.send(()).is_err() {
//             log::error!("Failed to send stop signal to reader loop")
//         }
//         if self.writer_stop.send(()).is_err() {
//             log::error!("Failed to send stop signal to writer loop")
//         }

//         task::block_on(async {
//             if let Some(reader) =self.reader_handle.take() {
//                 reader.await;
//             }
            
//             if let Some(writer) = self.writer_handle.take() {
//                 writer.await;
//             }
//         })
//     }
// }

impl Client<NotConnected> {
    pub fn with_codec<C>(codec: C) -> Client<Connected>
    where
        C: ClientCodecSplit + Send + Sync + 'static,
    {
        // let codec: Box<dyn ClientCodec> = Box::new(codec);
        let (writer, reader) = codec.split();
        let (req_sender, req_recver) = flume::unbounded();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let (reader_stop, stop) = flume::bounded(1);
        task::spawn(reader_loop(reader, pending.clone(), stop));
        // let reader_handle = Some(handle);

        let (writer_stop, stop) = flume::bounded(1);
        task::spawn(writer_loop(writer, req_recver, stop));
        // let writer_handle = Some(handle);


        Client::<Connected> {
            count: AtomicMessageId::new(0),
            pending,
            requests: req_sender,
            reader_stop,
            writer_stop,
            // reader_handle,
            // writer_handle,

            marker: PhantomData,
        }
    }
}

impl Client<Connected> {
    pub fn call_blocking<Req, Res>(
        &self,
        service_method: impl ToString,
        args: Req,
    ) -> Result<Res, Error>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        let call = self.call(service_method, args);
        task::block_on(call)
    }

    pub fn call<Req, Res>(&self, service_method: impl ToString, args: Req) -> Call<Res>
    where
        Req: serde::Serialize + Send + Sync + 'static,
        Res: serde::de::DeserializeOwned + Send + 'static,
    {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        let service_method = service_method.to_string();
        let header = RequestHeader { id, service_method };
        let body = Box::new(args) as RequestBody;

        // create oneshot channel
        let (done_tx, done_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let pending = self.pending.clone();
        let request_tx = self.requests.clone();
        let handle = task::spawn(handle_call(
            pending, header, body, request_tx, cancel_rx, done_tx,
        ));

        // create Call
        let call = Call::<Res> {
            id,
            cancel: cancel_tx,
            done: done_rx,
            handle,
        };
        call
    }
}
