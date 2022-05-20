use bytes::{Buf, BufMut, BytesMut};
use std::io;
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};
use zef_base::message::Message;

#[derive(Clone, Copy, Debug)]
pub struct Codec;

impl Encoder<Message> for Codec {
    type Error = Error;

    fn encode(&mut self, message: Message, buffer: &mut BytesMut) -> Result<(), Self::Error> {
        bincode::serialize_into(&mut buffer.writer(), &message)
            .map_err(|error| Error::Serialization(*error))
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match bincode::deserialize_from(buffer.reader()) {
            Ok(message) => Ok(Some(message)),
            Err(boxed_error) => match *boxed_error {
                bincode::ErrorKind::Custom(_) => {
                    // An error from `serde`, likely that the message is incomplete
                    Ok(None)
                }
                error => Err(Error::Deserialization(error)),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error in the underlying transport")]
    Io(#[from] io::Error),

    #[error("Failed to deserialize an incoming message")]
    Deserialization(#[source] bincode::ErrorKind),

    #[error("Failed to serialize outgoing message")]
    Serialization(#[source] bincode::ErrorKind),
}
