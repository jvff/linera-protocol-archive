// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types declared in [`linera-sdk`] to types generated by [`wit-bindgen-guest-rust`].

use super::{service_system_api as wit_system_api, wit_types};
use linera_base::{
    crypto::CryptoHash,
    identifiers::{ApplicationId, MessageId},
};

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
    fn from(hash_value: CryptoHash) -> Self {
        let parts = <[u64; 4]>::from(hash_value);

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

impl From<MessageId> for wit_system_api::MessageId {
    fn from(message_id: MessageId) -> Self {
        wit_system_api::MessageId {
            chain_id: message_id.chain_id.0.into(),
            height: message_id.height.0,
            index: message_id.index,
        }
    }
}
