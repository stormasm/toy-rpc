use async_trait::async_trait;
use erased_serde as erased;
use serde;

use crate::Error;
use crate::rpc::{MessageId, Metadata, RequestHeader, ResponseHeader};

#[cfg(all(feature = "codec-json", not(feauture = "codec-bincode")))]
mod json;

#[cfg(all(feature = "codec-json", not(feauture = "codec-bincode")))]
pub use crate::codec::json::Codec as DefaultCodec;

#[cfg(all(feature = "codec-bincode", not(feature = "codec-json")))]
mod bincode;

#[cfg(all(feature = "codec-bincode", not(feature = "codec-json")))]
pub use crate::codec::bincode::Codec as DefaultCodec;

#[async_trait]
pub trait ServerCodec: Send + Sync {
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>>;
    async fn read_request_body<'c>(
        &'c mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer + Send + Sync + 'c>, Error>>;

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
}

#[async_trait]
pub trait ClientCodec: Send + Sync {
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>>;
    async fn read_response_body<'c>(
        &'c mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer + Send + Sync + 'c>, Error>>;

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
}

#[async_trait]
pub trait CodecRead: Unmarshal {
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned;

    async fn read_body<'c>(
        &'c mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'c> + Send + Sync + 'c>, Error>>;
}

#[async_trait]
pub trait CodecWrite: Marshal {
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where
        H: serde::Serialize + Metadata + Send;

    async fn write_body(
        &mut self,
        message_id: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error>;
}

pub trait Marshal {
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error>;
}

pub trait Unmarshal {
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error>;
}

#[async_trait]
impl<T> ServerCodec for T
where
    T: CodecRead + CodecWrite + Send + Sync,
{
    async fn read_request_header(&mut self) -> Option<Result<RequestHeader, Error>> {
        self.read_header().await
    }

    async fn read_request_body<'c>(
        &'c mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'c> + Send + Sync + 'c>, Error>> {
        self.read_body().await
    }

    async fn write_response(
        &mut self,
        header: ResponseHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let id = header.get_id();

        let h = self.write_header(header).await?;
        let b = self.write_body(id, body).await?;

        Ok(h + b)
    }
}

#[async_trait]
impl<T> ClientCodec for T
where
    T: CodecRead + CodecWrite + Send + Sync,
{
    async fn read_response_header(&mut self) -> Option<Result<ResponseHeader, Error>> {
        self.read_header().await
    }

    async fn read_response_body<'c>(
        &'c mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'c> + Send + Sync + 'c>, Error>> {
        self.read_body().await
    }

    async fn write_request(
        &mut self,
        header: RequestHeader,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let id = header.get_id();

        let h = self.write_header(header).await?;
        let b = self.write_body(id, body).await?;

        Ok(h + b)
    }
}

pub(crate) struct DeserializerOwned<D> {
    inner: D,
}

impl<D> DeserializerOwned<D> {
    pub fn new(inner: D) -> Self {
        Self { inner }
    }
}
