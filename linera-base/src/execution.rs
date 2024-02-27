// Copyright (c) Facebook, Inc. and its affiliates.
// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Data-types used in the execution of Linera applications.

use crate::{
    data_types::BlockHeight,
    identifiers::{Account, ChainId, MessageId, Owner},
};

/// The context of an application when it is executing an operation.
#[derive(Clone, Copy, Debug)]
pub struct OperationContext {
    /// The current chain id.
    pub chain_id: ChainId,
    /// The authenticated signer of the operation, if any.
    pub authenticated_signer: Option<Owner>,
    /// The current block height.
    pub height: BlockHeight,
    /// The current index of the operation.
    pub index: u32,
    /// The index of the next message to be created.
    pub next_message_index: u32,
}

impl OperationContext {
    /// Returns the [`Account`] that should receive the refund of a grant provided for the
    /// execution of this context.
    pub fn refund_grant_to(&self) -> Option<Account> {
        Some(Account {
            chain_id: self.chain_id,
            owner: self.authenticated_signer,
        })
    }

    /// Returns the next [`MessageId`] to use for the next message to be sent.
    pub fn next_message_id(&self) -> MessageId {
        MessageId {
            chain_id: self.chain_id,
            height: self.height,
            index: self.next_message_index,
        }
    }
}
