// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit-bindgen`] to types declared in [`linera-sdk`].

use super::wit::{contract_entrypoints as wit_entrypoints, contract_system_api as wit_system_api};
use linera_base::{
    crypto::CryptoHash,
    data_types::{Amount, BlockHeight, Timestamp},
    identifiers::{Account, ApplicationId, BytecodeId, ChainId, MessageId, Owner, SessionId},
};

use crate::{CalleeContext, MessageContext, OperationContext};

impl From<wit_entrypoints::OperationContext> for OperationContext {
    fn from(context: wit_entrypoints::OperationContext) -> Self {
        OperationContext {
            chain_id: context.chain_id.into(),
            authenticated_signer: context.authenticated_signer.map(Owner::from),
            height: BlockHeight(context.height.inner0),
            index: context.index,
            next_message_index: context.next_message_index,
        }
    }
}

impl From<wit_entrypoints::MessageContext> for MessageContext {
    fn from(context: wit_entrypoints::MessageContext) -> Self {
        MessageContext {
            chain_id: context.chain_id.into(),
            is_bouncing: context.is_bouncing,
            authenticated_signer: context.authenticated_signer.map(Owner::from),
            refund_grant_to: context.refund_grant_to.map(Account::from),
            height: BlockHeight(context.height.inner0),
            certificate_hash: context.certificate_hash.into(),
            message_id: context.message_id.into(),
        }
    }
}

impl From<wit_entrypoints::MessageId> for MessageId {
    fn from(message_id: wit_entrypoints::MessageId) -> Self {
        MessageId {
            chain_id: message_id.chain_id.into(),
            height: BlockHeight(message_id.height.inner0),
            index: message_id.index,
        }
    }
}

impl From<wit_entrypoints::CalleeContext> for CalleeContext {
    fn from(context: wit_entrypoints::CalleeContext) -> Self {
        CalleeContext {
            chain_id: context.chain_id.into(),
            authenticated_signer: context.authenticated_signer.map(Owner::from),
            authenticated_caller_id: context.authenticated_caller_id.map(ApplicationId::from),
        }
    }
}

impl From<wit_entrypoints::ApplicationId> for ApplicationId {
    fn from(application_id: wit_entrypoints::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: application_id.bytecode_id.into(),
            creation: application_id.creation.into(),
        }
    }
}

impl From<wit_entrypoints::BytecodeId> for BytecodeId {
    fn from(bytecode_id: wit_entrypoints::BytecodeId) -> Self {
        BytecodeId::new(bytecode_id.message_id.into())
    }
}

impl From<wit_entrypoints::SessionId> for SessionId {
    fn from(session_id: wit_entrypoints::SessionId) -> Self {
        SessionId {
            application_id: session_id.application_id.into(),
            index: session_id.index,
        }
    }
}

impl From<wit_entrypoints::Account> for Account {
    fn from(account: wit_entrypoints::Account) -> Self {
        Account {
            chain_id: account.chain_id.into(),
            owner: account.owner.map(Owner::from),
        }
    }
}

impl From<wit_entrypoints::Owner> for Owner {
    fn from(owner: wit_entrypoints::Owner) -> Self {
        Owner(owner.inner0.into())
    }
}

impl From<wit_entrypoints::ChainId> for ChainId {
    fn from(chain_id: wit_entrypoints::ChainId) -> Self {
        ChainId(chain_id.inner0.into())
    }
}

impl From<wit_entrypoints::CryptoHash> for CryptoHash {
    fn from(crypto_hash: wit_entrypoints::CryptoHash) -> Self {
        CryptoHash::from([
            crypto_hash.part1,
            crypto_hash.part2,
            crypto_hash.part3,
            crypto_hash.part4,
        ])
    }
}

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

impl From<wit_system_api::MessageId> for MessageId {
    fn from(message_id: wit_system_api::MessageId) -> Self {
        MessageId {
            chain_id: message_id.chain_id.into(),
            height: BlockHeight(message_id.height.inner0),
            index: message_id.index,
        }
    }
}

impl From<wit_system_api::ApplicationId> for ApplicationId {
    fn from(application_id: wit_system_api::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: application_id.bytecode_id.into(),
            creation: application_id.creation.into(),
        }
    }
}

impl From<wit_system_api::BytecodeId> for BytecodeId {
    fn from(bytecode_id: wit_system_api::BytecodeId) -> Self {
        BytecodeId::new(bytecode_id.message_id.into())
    }
}

impl From<wit_system_api::SessionId> for SessionId {
    fn from(session_id: wit_system_api::SessionId) -> Self {
        SessionId {
            application_id: session_id.application_id.into(),
            index: session_id.index,
        }
    }
}

impl From<wit_system_api::ChainId> for ChainId {
    fn from(chain_id: wit_system_api::ChainId) -> Self {
        ChainId(chain_id.inner0.into())
    }
}

impl From<wit_system_api::CryptoHash> for CryptoHash {
    fn from(crypto_hash: wit_system_api::CryptoHash) -> Self {
        CryptoHash::from([
            crypto_hash.part1,
            crypto_hash.part2,
            crypto_hash.part3,
            crypto_hash.part4,
        ])
    }
}
