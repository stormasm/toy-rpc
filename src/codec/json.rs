use async_trait::async_trait;
use erased_serde as erased;
use futures::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
};
use serde::de::Visitor;
use std::io::Cursor; // serde doesn't support AsyncRead

use super::{CodecRead, CodecWrite, DeserializerOwned, Marshal, Unmarshal};
use crate::error::Error;
use crate::macros::impl_inner_deserializer;
use crate::message::{MessageId, Metadata};

impl<'de, R> serde::Deserializer<'de> for DeserializerOwned<serde_json::Deserializer<R>>
where
    R: serde_json::de::Read<'de>,
{
    type Error = <&'de mut serde_json::Deserializer<R> as serde::Deserializer<'de>>::Error;

    // the rest is simply calling self.inner.deserialize_xxx()
    // use a macro to generate the code
    impl_inner_deserializer!();
}

pub struct Codec<R, W>
where
// R: AsyncBufRead + Send + Sync,
// W: AsyncWrite + Send + Sync,
{
    reader: R,
    writer: W,
}

impl<T> Codec<BufReader<T>, BufWriter<T>>
where
    T: AsyncRead + AsyncWrite + Send + Sync + Unpin + Clone,
{
    pub fn new(stream: T) -> Self {
        Self::with_reader_writer(BufReader::new(stream.clone()), BufWriter::new(stream))
    }
}

impl<R, W> Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + AsyncWriteExt + Send + Sync + Unpin,
{
    pub fn with_reader_writer(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

#[async_trait]
impl<R, W> CodecRead for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
    where
        H: serde::de::DeserializeOwned,
    {
        let mut buf = String::new();
        match self.reader.read_line(&mut buf).await {
            Ok(_) => Some(Self::unmarshal(buf.as_bytes())),
            Err(_) => None,
        }
    }

    async fn read_body(
        &mut self,
    ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
        let mut buf = String::new();

        let de = match self.reader.read_line(&mut buf).await {
            Ok(_) => serde_json::Deserializer::from_reader(Cursor::new(buf.into_bytes())),
            Err(e) => return Some(Err(e.into())),
        };

        // wrap the deserializer as DeserializerOwned
        let de_owned = DeserializerOwned::new(de);

        Some(Ok(Box::new(erased::Deserializer::erase(de_owned))))
    }
}

#[async_trait]
impl<R, W> CodecWrite for Codec<R, W>
where
    R: AsyncBufRead + Send + Sync + Unpin,
    W: AsyncWrite + Send + Sync + Unpin,
{
    async fn write_header<H>(&mut self, header: H) -> Result<usize, Error>
    where
        H: serde::Serialize + Metadata + Send,
    {
        let _ = header.get_id();
        let buf = Self::marshal(&header)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }

    async fn write_body(
        &mut self,
        _: MessageId,
        body: &(dyn erased::Serialize + Send + Sync),
    ) -> Result<usize, Error> {
        let buf = Self::marshal(&body)?;

        let bytes_sent = self.writer.write(&buf).await?;
        self.writer.write(b"\n").await?;
        self.writer.flush().await?;
        Ok(bytes_sent)
    }
}

impl<R, W> Marshal for Codec<R, W>
where
    R: Send + Sync,
    W: Send + Sync,
{
    fn marshal<S: serde::Serialize>(val: &S) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(val).map_err(|e| e.into())
    }
}

impl<R, W> Unmarshal for Codec<R, W>
where
    R: Send + Sync,
    W: Send + Sync,
{
    fn unmarshal<'de, D: serde::Deserialize<'de>>(buf: &'de [u8]) -> Result<D, Error> {
        serde_json::from_slice(buf).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::message::RequestHeader;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct FooRequest {
        pub a: u32,
        pub b: u32,
    }

    #[test]
    fn json_request() {
        let header = RequestHeader {
            id: 0,
            service_method: "service.method".to_string(),
        };
        let body = FooRequest { a: 3, b: 6 };

        let header_buf = serde_json::to_string(&header).unwrap();
        let body_buf = serde_json::to_string(&body).unwrap();

        println!("header:\n{}", header_buf);
        println!("body:\n{}", body_buf);
    }
}
