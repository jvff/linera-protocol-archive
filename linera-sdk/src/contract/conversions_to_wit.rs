// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types declared in [`linera-sdk`] to types generated by
//! [`wit-bindgen-guest-rust`].

use super::{contract_system_api as wit_system_api, wit_types};
use crate::{ApplicationCallResult, ExecutionResult, OutgoingMessage, SessionCallResult};
use linera_base::{
    crypto::CryptoHash,
    identifiers::{ApplicationId, ChannelName, Destination, MessageId, SessionId},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, task::Poll};

impl From<CryptoHash> for wit_system_api::CryptoHash {
    fn from(hash_value: CryptoHash) -> Self {
        let parts = <[u64; 4]>::from(hash_value);

        wit_system_api::CryptoHash {
            part1: parts[0],
            part2: parts[1],
            part3: parts[2],
            part4: parts[3],
        }
    }
}

impl From<CryptoHash> for wit_types::CryptoHash {
    fn from(crypto_hash: CryptoHash) -> Self {
        let parts = <[u64; 4]>::from(crypto_hash);

        wit_types::CryptoHash {
            part1: parts[0],
            part2: parts[1],
            part3: parts[2],
            part4: parts[3],
        }
    }
}

impl From<ApplicationId> for wit_system_api::ApplicationId {
    fn from(application_id: ApplicationId) -> wit_system_api::ApplicationId {
        wit_system_api::ApplicationId {
            bytecode_id: application_id.bytecode_id.message_id.into(),
            creation: application_id.creation.into(),
        }
    }
}

impl From<SessionId> for wit_system_api::SessionId {
    fn from(session_id: SessionId) -> Self {
        wit_system_api::SessionId {
            application_id: session_id.application_id.into(),
            index: session_id.index,
        }
    }
}

impl From<MessageId> for wit_system_api::MessageId {
    fn from(message_id: MessageId) -> Self {
        wit_system_api::MessageId {
            chain_id: message_id.chain_id.0.into(),
            height: message_id.height.0,
            index: message_id.index,
        }
    }
}

impl From<log::Level> for wit_system_api::LogLevel {
    fn from(level: log::Level) -> Self {
        match level {
            log::Level::Trace => wit_system_api::LogLevel::Trace,
            log::Level::Debug => wit_system_api::LogLevel::Debug,
            log::Level::Info => wit_system_api::LogLevel::Info,
            log::Level::Warn => wit_system_api::LogLevel::Warn,
            log::Level::Error => wit_system_api::LogLevel::Error,
        }
    }
}

impl<Message, Value, SessionState> From<ApplicationCallResult<Message, Value, SessionState>>
    for wit_types::ApplicationCallResult
where
    Message: Serialize + DeserializeOwned + Debug,
    Value: Serialize,
    SessionState: Serialize,
{
    fn from(result: ApplicationCallResult<Message, Value, SessionState>) -> Self {
        // TODO(#743): Do we need explicit error handling?
        let value = bcs::to_bytes(&result.value)
            .expect("failed to serialize Value for ApplicationCallResult");

        let create_sessions = result
            .create_sessions
            .into_iter()
            .map(|v| {
                bcs::to_bytes(&v)
                    .expect("failed to serialize session state for ApplicationCallResult")
            })
            .collect();

        wit_types::ApplicationCallResult {
            value,
            execution_result: result.execution_result.into(),
            create_sessions,
        }
    }
}

impl<Message, Value, SessionState> From<SessionCallResult<Message, Value, SessionState>>
    for wit_types::SessionCallResult
where
    Message: Serialize + DeserializeOwned + Debug,
    Value: Serialize,
    SessionState: Serialize,
{
    fn from(result: SessionCallResult<Message, Value, SessionState>) -> Self {
        let new_state = result.new_state.as_ref().map(|state| {
            // TODO(#743): Do we need explicit error handling?
            bcs::to_bytes(state).expect("session type serialization failed")
        });
        wit_types::SessionCallResult {
            inner: result.inner.into(),
            new_state,
        }
    }
}

impl<Message> From<OutgoingMessage<Message>> for wit_types::OutgoingMessage
where
    Message: Debug + Serialize + DeserializeOwned,
{
    fn from(message: OutgoingMessage<Message>) -> Self {
        Self {
            destination: message.destination.into(),
            authenticated: message.authenticated,
            is_skippable: message.is_skippable,
            // TODO(#743): Do we need explicit error handling?
            message: bcs::to_bytes(&message.message).expect("message serialization failed"),
        }
    }
}

impl<Message> From<ExecutionResult<Message>> for wit_types::ExecutionResult
where
    Message: Debug + Serialize + DeserializeOwned,
{
    fn from(result: ExecutionResult<Message>) -> Self {
        let messages = result
            .messages
            .into_iter()
            .map(wit_types::OutgoingMessage::from)
            .collect();

        let subscribe = result
            .subscribe
            .into_iter()
            .map(|(subscription, chain_id)| (subscription.into(), chain_id.0.into()))
            .collect();

        let unsubscribe = result
            .unsubscribe
            .into_iter()
            .map(|(subscription, chain_id)| (subscription.into(), chain_id.0.into()))
            .collect();

        wit_types::ExecutionResult {
            messages,
            subscribe,
            unsubscribe,
        }
    }
}

impl From<Destination> for wit_types::Destination {
    fn from(destination: Destination) -> Self {
        match destination {
            Destination::Recipient(chain_id) => {
                wit_types::Destination::Recipient(chain_id.0.into())
            }
            Destination::Subscribers(subscription) => {
                wit_types::Destination::Subscribers(subscription.into())
            }
        }
    }
}

impl From<ChannelName> for wit_types::ChannelName {
    fn from(name: ChannelName) -> Self {
        wit_types::ChannelName {
            name: name.into_bytes(),
        }
    }
}

impl<Message, Value, SessionState>
    From<Poll<Result<ApplicationCallResult<Message, Value, SessionState>, String>>>
    for wit_types::PollApplicationCallResult
where
    Message: Serialize + DeserializeOwned + Debug,
    Value: Serialize,
    SessionState: Serialize,
{
    fn from(
        poll: Poll<Result<ApplicationCallResult<Message, Value, SessionState>, String>>,
    ) -> Self {
        use wit_types::PollApplicationCallResult;
        match poll {
            Poll::Pending => PollApplicationCallResult::Pending,
            Poll::Ready(Ok(result)) => PollApplicationCallResult::Ready(Ok(result.into())),
            Poll::Ready(Err(message)) => PollApplicationCallResult::Ready(Err(message)),
        }
    }
}

impl<Message, Value, SessionState>
    From<Poll<Result<SessionCallResult<Message, Value, SessionState>, String>>>
    for wit_types::PollSessionCallResult
where
    Message: Serialize + DeserializeOwned + Debug,
    Value: Serialize,
    SessionState: Serialize,
{
    fn from(poll: Poll<Result<SessionCallResult<Message, Value, SessionState>, String>>) -> Self {
        use wit_types::PollSessionCallResult;
        match poll {
            Poll::Pending => PollSessionCallResult::Pending,
            Poll::Ready(Ok(result)) => PollSessionCallResult::Ready(Ok(result.into())),
            Poll::Ready(Err(message)) => PollSessionCallResult::Ready(Err(message)),
        }
    }
}
