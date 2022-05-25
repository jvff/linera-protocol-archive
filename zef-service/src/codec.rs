use bytes::{Buf, BufMut, BytesMut};
use std::{io, mem, ops::DerefMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};
use zef_base::rpc;

/// An encoder/decoder of [`rpc::Message`]s for the RPC protocol.
pub type Codec = LengthDelimitedCodec<BincodeCodec>;

/// An encoder/decoder of length delimited frames.
///
/// The frames are then processed by the `InnerCodec`.
#[derive(Clone, Copy, Debug, Default)]
pub struct LengthDelimitedCodec<InnerCodec> {
    inner: InnerCodec,
}

impl<InnerCodec, Message> Encoder<Message> for LengthDelimitedCodec<InnerCodec>
where
    InnerCodec: Encoder<Message>,
    Error: From<InnerCodec::Error>,
{
    type Error = Error;

    fn encode(&mut self, message: Message, buffer: &mut BytesMut) -> Result<(), Self::Error> {
        let prefix_size = mem::size_of::<u32>();

        buffer.put_u32_le(0);

        self.inner.encode(message, buffer)?;

        let frame_size = buffer.len();
        let payload_size = frame_size - prefix_size;

        let mut start_of_buffer = buffer.deref_mut();

        start_of_buffer.put_u32_le(
            payload_size
                .try_into()
                .map_err(|_| Error::MessageTooBig { size: payload_size })?,
        );

        Ok(())
    }
}

impl<InnerCodec> Decoder for LengthDelimitedCodec<InnerCodec>
where
    InnerCodec: Decoder,
    Error: From<InnerCodec::Error>,
{
    type Item = InnerCodec::Item;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let prefix_size = mem::size_of::<u32>();

        if buffer.len() < prefix_size {
            return Ok(None);
        }

        let payload_size = u32::from_le_bytes(
            buffer[..prefix_size]
                .try_into()
                .expect("Incorrect prefix size to select size bytes"),
        );

        let frame_size =
            u32::try_from(prefix_size).expect("Prefix size is too large") + payload_size;

        if buffer.len().try_into().unwrap_or(u32::MAX) < frame_size {
            buffer.reserve(frame_size.try_into().expect("u32 should fit in a usize"));
            return Ok(None);
        }

        let _prefix = buffer.split_to(prefix_size);
        let mut payload =
            buffer.split_to(payload_size.try_into().expect("u32 should fit in a usize"));

        match self.inner.decode(&mut payload) {
            Ok(Some(message)) => Ok(Some(message)),
            Ok(None) => Err(Error::FrameWithIncompleteMessage),
            Err(error) => Err(error.into()),
        }
    }
}

/// The encoder/decoder of [`rpc::Message`]s that handles the serialization of messages.
#[derive(Clone, Copy, Debug, Default)]
pub struct BincodeCodec;

impl Encoder<rpc::Message> for BincodeCodec {
    type Error = Error;

    fn encode(&mut self, message: rpc::Message, buffer: &mut BytesMut) -> Result<(), Self::Error> {
        bincode::serialize_into(&mut buffer.writer(), &message)
            .map_err(|error| Error::Serialization(*error))
    }
}

impl Decoder for BincodeCodec {
    type Item = rpc::Message;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match bincode::deserialize_from(buffer.reader()) {
            Ok(message) => Ok(Some(message)),
            Err(boxed_error) => match *boxed_error {
                bincode::ErrorKind::Io(io_error)
                    if io_error.kind() == io::ErrorKind::UnexpectedEof =>
                {
                    Ok(None)
                }
                bincode::ErrorKind::Io(io_error) => Err(Error::Io(io_error)),
                error => Err(Error::Deserialization(error)),
            },
        }
    }
}

/// Errors that can arise during transmission or reception of [`rpc::Message`]s.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error in the underlying transport")]
    Io(#[from] io::Error),

    #[error("Failed to deserialize an incoming message")]
    Deserialization(#[source] bincode::ErrorKind),

    #[error("Failed to serialize outgoing message")]
    Serialization(#[source] bincode::ErrorKind),

    #[error("Message is too big to fit in a protocol frame: \
        message is {size} bytes but can't be larger than {max} bytes.",
        max = u32::MAX)]
    MessageTooBig { size: usize },

    #[error("Frame contains an incomplete message")]
    FrameWithIncompleteMessage,
}
