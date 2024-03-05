// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the contract.

use super::contract_system_api as wit;
use linera_base::{
    data_types::BlockHeight,
    identifiers::{ApplicationId, ChainId, MessageId, Owner},
};

/// The common runtime to interface with the host executing the contract.
///
/// It automatically caches read-only values received from the host.
#[derive(Clone, Debug, Default)]
pub struct Runtime {
    application_id: Option<ApplicationId>,
    chain_id: Option<ChainId>,
    authenticated_signer: Option<Option<Owner>>,
    block_height: Option<BlockHeight>,
    transaction_index: Option<u32>,
    message_is_bouncing: Option<Option<bool>>,
    message_id: Option<Option<MessageId>>,
}

/// The runtime available during execution of an cross-application calls.
#[derive(Clone, Debug, Default)]
pub struct CalleeRuntime {
    common: Runtime,
    authenticated_caller_id: Option<Option<ApplicationId>>,
}

impl Runtime {
    /// Returns the ID of the current application.
    pub fn application_id(&mut self) -> ApplicationId {
        *self
            .application_id
            .get_or_insert_with(|| wit::application_id().into())
    }

    /// Returns the ID of the current chain.
    pub fn chain_id(&mut self) -> ChainId {
        *self.chain_id.get_or_insert_with(|| wit::chain_id().into())
    }

    /// Returns the authenticated signer for this execution, if there is one.
    pub fn authenticated_signer(&mut self) -> Option<Owner> {
        *self
            .authenticated_signer
            .get_or_insert_with(|| wit::authenticated_signer().map(Owner::from))
    }

    /// Returns the height of the current block that is executing.
    pub fn block_height(&mut self) -> BlockHeight {
        *self
            .block_height
            .get_or_insert_with(|| wit::block_height().into())
    }

    /// Returns the index of the current transaction.
    pub fn transaction_index(&mut self) -> u32 {
        *self
            .transaction_index
            .get_or_insert_with(wit::transaction_index)
    }

    /// Returns the ID of the incoming message that is being handled, or [`None`] if not executing
    /// an incoming message.
    pub fn message_id(&mut self) -> Option<MessageId> {
        *self
            .message_id
            .get_or_insert_with(|| wit::message_id().map(MessageId::from))
    }

    /// Returns [`true`] if the incoming message was rejected from the original destination and is
    /// now bouncing back, or [`None`] if not executing an incoming message.
    pub fn message_is_bouncing(&mut self) -> Option<bool> {
        *self
            .is_bouncing
            .get_or_insert_with(wit::message_is_bouncing())
    }
}

impl CalleeRuntime {
    /// Returns the authenticated caller ID, if the caller configured it.
    pub fn authenticated_caller_id(&mut self) -> Option<ApplicationId> {
        *self.authenticated_caller_id.get_or_insert_with(|| {
            wit::authenticated_caller_id()
                .expect("No callee information available in the current context")
                .map(|caller_id| caller_id.into())
        })
    }
}
