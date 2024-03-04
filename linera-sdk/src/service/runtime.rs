// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the service.

use super::service_system_api as wit;
use linera_base::{data_types::BlockHeight, identifiers::ChainId};

/// The runtime available during execution of a query.
#[derive(Clone, Debug, Default)]
pub struct QueryRuntime {
    chain_id: Option<ChainId>,
    next_block_height: Option<BlockHeight>,
}

impl QueryRuntime {
    /// Returns the ID of the current chain.
    pub fn chain_id(&mut self) -> ChainId {
        *self.chain_id.get_or_insert_with(|| wit::chain_id().into())
    }

    /// Returns the height of the next block that can be added to the current chain.
    pub fn next_block_height(&mut self) -> BlockHeight {
        *self
            .next_block_height
            .get_or_insert_with(|| wit::next_block_height().into())
    }
}
