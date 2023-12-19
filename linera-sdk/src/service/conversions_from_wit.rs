// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit-bindgen-guest-rust`] to types declared in [`linera-sdk`].

use super::{system_api::private::wit as wit_system_api, wit_types};
use crate::QueryContext;
use linera_base::{
    crypto::CryptoHash,
    data_types::{Amount, BlockHeight},
    identifiers::{ApplicationId, BytecodeId, ChainId, MessageId},
};

impl From<wit_types::QueryContext> for QueryContext {
    fn from(application_context: wit_types::QueryContext) -> Self {
        QueryContext {
            chain_id: application_context.chain_id.into(),
        }
    }
}

impl From<wit_types::ChainId> for ChainId {
    fn from(chain_id: wit_types::ChainId) -> Self {
        ChainId(chain_id.inner0.into())
    }
}

impl From<wit_types::CryptoHash> for CryptoHash {
    fn from(hash_value: wit_types::CryptoHash) -> Self {
        CryptoHash::from([
            hash_value.part1,
            hash_value.part2,
            hash_value.part3,
            hash_value.part4,
        ])
    }
}

impl From<wit_system_api::ChainId> for ChainId {
    fn from(chain_id: wit_system_api::ChainId) -> Self {
        ChainId(chain_id.inner0.into())
    }
}

impl From<wit_system_api::CryptoHash> for CryptoHash {
    fn from(hash_value: wit_system_api::CryptoHash) -> Self {
        CryptoHash::from([
            hash_value.part1,
            hash_value.part2,
            hash_value.part3,
            hash_value.part4,
        ])
    }
}

impl From<wit_system_api::ApplicationId> for ApplicationId {
    fn from(application_id: wit_system_api::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: BytecodeId::new(application_id.bytecode_id.message_id.into()),
            creation: application_id.creation.into(),
        }
    }
}

impl From<wit_system_api::MessageId> for MessageId {
    fn from(message_id: wit_system_api::MessageId) -> Self {
        MessageId {
            chain_id: message_id.chain_id.into(),
            height: BlockHeight(message_id.height.inner0),
            index: message_id.index,
        }
    }
}

impl From<wit_system_api::Amount> for Amount {
    fn from(balance: wit_system_api::Amount) -> Self {
        let (lower_half, upper_half) = balance.inner0;
        let value = ((upper_half as u128) << 64) | (lower_half as u128);
        Amount::from_atto(value)
    }
}
