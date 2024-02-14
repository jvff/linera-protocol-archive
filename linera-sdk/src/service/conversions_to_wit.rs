// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types declared in [`linera-sdk`] to types generated by [`wit-bindgen`].

<<<<<<< HEAD
use linera_base::{
    crypto::CryptoHash,
    identifiers::{ApplicationId, MessageId, Owner},
};

use super::wit_system_api;

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

=======
use super::wit::{service_entrypoints as wit_entrypoints, service_system_api as wit_system_api};
use linera_base::{
    crypto::CryptoHash,
    data_types::{Amount, BlockHeight, Timestamp},
    identifiers::{ApplicationId, BytecodeId, ChainId, MessageId},
};

>>>>>>> 3e89af2a50 (WIP)
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

<<<<<<< HEAD
impl From<ApplicationId> for wit_system_api::ApplicationId {
    fn from(application_id: ApplicationId) -> wit_system_api::ApplicationId {
        wit_system_api::ApplicationId {
            bytecode_id: application_id.bytecode_id.message_id.into(),
=======
impl From<wit_system_api::Amount> for Amount {
    fn from(amount: wit_system_api::Amount) -> Self {
        let (lower_half, upper_half) = amount.inner0;

        Amount::from_attos(((upper_half as u128) << 64) | lower_half as u128)
    }
}

impl From<wit_system_api::Timestamp> for Timestamp {
    fn from(timestamp: wit_system_api::Timestamp) -> Self {
        Timestamp::from(timestamp.inner0)
    }
}

impl From<BlockHeight> for wit_system_api::BlockHeight {
    fn from(block_height: BlockHeight) -> Self {
        wit_system_api::BlockHeight {
            inner0: block_height.0,
        }
    }
}

impl From<ChainId> for wit_system_api::ChainId {
    fn from(chain_id: ChainId) -> Self {
        wit_system_api::ChainId {
            inner0: chain_id.0.into(),
        }
    }
}

impl From<ApplicationId> for wit_system_api::ApplicationId {
    fn from(application_id: ApplicationId) -> Self {
        wit_system_api::ApplicationId {
            bytecode_id: application_id.bytecode_id.into(),
>>>>>>> 3e89af2a50 (WIP)
            creation: application_id.creation.into(),
        }
    }
}

<<<<<<< HEAD
impl From<MessageId> for wit_system_api::MessageId {
    fn from(message_id: MessageId) -> Self {
        wit_system_api::MessageId {
            chain_id: message_id.chain_id.0.into(),
            height: message_id.height.0,
=======
impl From<BytecodeId> for wit_system_api::BytecodeId {
    fn from(bytecode_id: BytecodeId) -> Self {
        wit_system_api::BytecodeId {
            message_id: bytecode_id.message_id.into(),
        }
    }
}

impl From<MessageId> for wit_system_api::MessageId {
    fn from(message_id: MessageId) -> Self {
        wit_system_api::MessageId {
            chain_id: message_id.chain_id.into(),
            height: message_id.height.into(),
>>>>>>> 3e89af2a50 (WIP)
            index: message_id.index,
        }
    }
}
<<<<<<< HEAD

impl From<Owner> for wit_system_api::CryptoHash {
    fn from(owner: Owner) -> Self {
        wit_system_api::CryptoHash::from(owner.0)
    }
}
=======
>>>>>>> 3e89af2a50 (WIP)
