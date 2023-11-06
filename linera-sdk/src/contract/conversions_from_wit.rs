// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit-bindgen-guest-rust`] to types declared in [`linera-sdk`].

use super::{
    contract_system_api::{self as wit_system_api},
    wit_types,
};
use linera_base::{
    crypto::CryptoHash,
    data_types::{Amount, BlockHeight},
    identifiers::{ApplicationId, BytecodeId, ChainId, MessageId, Owner, SessionId},
};

use crate::{CalleeContext, MessageContext, OperationContext};

impl From<wit_types::OperationContext> for OperationContext {
    fn from(application_context: wit_types::OperationContext) -> Self {
        OperationContext {
            chain_id: ChainId(application_context.chain_id.into()),
            authenticated_signer: application_context.authenticated_signer.map(Owner::from),
            height: BlockHeight(application_context.height),
            index: application_context.index,
        }
    }
}

impl From<wit_types::MessageContext> for MessageContext {
    fn from(application_context: wit_types::MessageContext) -> Self {
        MessageContext {
            chain_id: ChainId(application_context.chain_id.into()),
            authenticated_signer: application_context.authenticated_signer.map(Owner::from),
            height: BlockHeight(application_context.height),
            message_id: application_context.message_id.into(),
        }
    }
}

impl From<wit_types::MessageId> for MessageId {
    fn from(message_id: wit_types::MessageId) -> Self {
        MessageId {
            chain_id: ChainId(message_id.chain_id.into()),
            height: BlockHeight(message_id.height),
            index: message_id.index,
        }
    }
}

impl From<wit_types::CalleeContext> for CalleeContext {
    fn from(application_context: wit_types::CalleeContext) -> Self {
        CalleeContext {
            chain_id: ChainId(application_context.chain_id.into()),
            authenticated_signer: application_context.authenticated_signer.map(Owner::from),
            authenticated_caller_id: application_context
                .authenticated_caller_id
                .map(ApplicationId::from),
        }
    }
}

impl From<wit_types::ApplicationId> for ApplicationId {
    fn from(application_id: wit_types::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: BytecodeId::new(application_id.bytecode_id.into()),
            creation: application_id.creation.into(),
        }
    }
}

impl From<wit_types::SessionId> for SessionId {
    fn from(session_id: wit_types::SessionId) -> Self {
        SessionId {
            application_id: session_id.application_id.into(),
            index: session_id.index,
        }
    }
}

impl From<wit_types::CryptoHash> for Owner {
    fn from(crypto_hash: wit_types::CryptoHash) -> Self {
        Owner(crypto_hash.into())
    }
}

impl From<wit_types::CryptoHash> for CryptoHash {
    fn from(crypto_hash: wit_types::CryptoHash) -> Self {
        CryptoHash::from([
            crypto_hash.part1,
            crypto_hash.part2,
            crypto_hash.part3,
            crypto_hash.part4,
        ])
    }
}

impl From<wit_system_api::MessageId> for MessageId {
    fn from(message_id: wit_system_api::MessageId) -> Self {
        MessageId {
            chain_id: ChainId(message_id.chain_id.into()),
            height: BlockHeight(message_id.height),
            index: message_id.index,
        }
    }
}

impl From<wit_system_api::ApplicationId> for ApplicationId {
    fn from(application_id: wit_system_api::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: BytecodeId::new(application_id.bytecode_id.into()),
            creation: application_id.creation.into(),
        }
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

impl From<wit_system_api::Amount> for Amount {
    fn from(balance: wit_system_api::Amount) -> Self {
        let value = ((balance.upper_half as u128) << 64) | (balance.lower_half as u128);
        Amount::from_atto(value)
    }
}

impl From<LockResult> for Poll<bool> {
    fn from(lock_result: LockResult) -> Poll<bool> {
        match lock_result {
            LockResult::Locked => Poll::Ready(true),
            LockResult::NotLocked => Poll::Ready(false),
        }
    }
}

impl From<wit_system_api::CallResult> for (Vec<u8>, Vec<SessionId>) {
    fn from(call_result: wit_system_api::CallResult) -> (Vec<u8>, Vec<SessionId>) {
        let value = call_result.value;

        let sessions = call_result
            .sessions
            .into_iter()
            .map(SessionId::from)
            .collect();

        (value, sessions)
    }
}

impl From<wit_system_api::SessionId> for SessionId {
    fn from(session_id: wit_system_api::SessionId) -> SessionId {
        SessionId {
            application_id: session_id.application_id.into(),
            index: session_id.index,
        }
    }
}
