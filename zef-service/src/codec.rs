use bytes::{Buf, BufMut, BytesMut};
use std::{io, mem, ops::DerefMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};
use zef_base::rpc;

/// The size of the frame prefix that contains the payload size.
const PREFIX_SIZE: u8 = mem::size_of::<u32>() as u8;

/// An encoder/decoder of [`rpc::Message`]s for the RPC protocol.
///
/// The frames are length-delimited by a [`u32`] prefix, and the payload is deserialized by
/// [`bincode`].
#[derive(Clone, Copy, Debug)]
pub struct Codec;

impl Encoder<rpc::Message> for Codec {
    type Error = Error;

    fn encode(&mut self, message: rpc::Message, buffer: &mut BytesMut) -> Result<(), Self::Error> {
        let mut frame_buffer = buffer.split_off(buffer.len());

        self.write_dummy_prefix(&mut frame_buffer);
        self.serialize_message(&mut frame_buffer, message)?;
        self.write_real_prefix(&mut frame_buffer)?;

        buffer.unsplit(frame_buffer);

        Ok(())
    }
}

impl Codec {
    pub fn write_dummy_prefix(self, frame_buffer: &mut BytesMut) {
        frame_buffer.put_u32_le(0);
    }

    pub fn serialize_message(
        self,
        frame_buffer: &mut BytesMut,
        message: rpc::Message,
    ) -> Result<(), Error> {
        let mut frame_writer = mem::take(frame_buffer).writer();

        bincode::serialize_into(&mut frame_writer, &message)
            .map_err(|error| Error::Serialization(*error))?;

        *frame_buffer = frame_writer.into_inner();

        Ok(())
    }

    pub fn write_real_prefix(self, frame_buffer: &mut BytesMut) -> Result<(), Error> {
        let frame_size = frame_buffer.len();
        let payload_size = frame_size - PREFIX_SIZE as usize;

        let mut start_of_frame = frame_buffer.deref_mut();

        start_of_frame.put_u32_le(
            payload_size
                .try_into()
                .map_err(|_| Error::MessageTooBig { size: payload_size })?,
        );

        Ok(())
    }
}

impl Decoder for Codec {
    type Item = rpc::Message;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if buffer.len() < PREFIX_SIZE.into() {
            return Ok(None);
        }

        let payload_size = self.read_payload_size(&buffer);
        let frame_size = PREFIX_SIZE as usize + payload_size;

        if buffer.len() < frame_size {
            buffer.reserve(frame_size);
            return Ok(None);
        }

        self.deserialize_message(buffer).map(Some)
    }
}

impl Codec {
    pub fn read_payload_size(self, buffer: &[u8]) -> usize {
        buffer
            .get_u32_le()
            .try_into()
            .expect("u32 should fit in a usize")
    }

    pub fn deserialize_message(self, buffer: &mut BytesMut) -> Result<rpc::Message, Error> {
        let _prefix = buffer.split_to(PREFIX_SIZE.into());
        let payload = buffer.split_to(payload_size);

        bincode::deserialize(&payload).map_err(|error| Error::Deserialization(*error))
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
}
