// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the service.

use super::service_system_api as wit;
use linera_base::{abi::ServiceAbi, data_types::BlockHeight, identifiers::ChainId};
use std::cell::Cell;

/// The runtime available during execution of a query.
pub struct ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    application_parameters: Cell<Option<Abi::Parameters>>,
    chain_id: Cell<Option<ChainId>>,
    next_block_height: Cell<Option<BlockHeight>>,
}

impl<Abi> Default for ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    fn default() -> Self {
        ServiceRuntime {
            application_parameters: Cell::new(None),
            chain_id: Cell::new(None),
            next_block_height: Cell::new(None),
        }
    }
}

impl<Abi> Clone for ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    fn clone(&self) -> Self {
        fn clone_cell<T: Clone>(cell: &Cell<Option<T>>) -> Cell<Option<T>> {
            let value = cell.take();
            let new_cell = Cell::new(value.clone());
            cell.set(value);
            new_cell
        }

        ServiceRuntime {
            application_parameters: clone_cell(&self.application_parameters),
            chain_id: clone_cell(&self.chain_id),
            next_block_height: clone_cell(&self.next_block_height),
        }
    }
}

impl<Abi> ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    /// Returns the application parameters provided when the application was created.
    pub fn application_parameters(&self) -> Abi::Parameters {
        Self::fetch_value_through_cache(&self.application_parameters, || {
            let bytes = wit::application_parameters();
            serde_json::from_slice(&bytes).expect("Application parameters must be deserializable")
        })
    }

    /// Returns the ID of the current chain.
    pub fn chain_id(&self) -> ChainId {
        Self::fetch_value_through_cache(&self.chain_id, || wit::chain_id().into())
    }

    /// Returns the height of the next block that can be added to the current chain.
    pub fn next_block_height(&self) -> BlockHeight {
        Self::fetch_value_through_cache(&self.next_block_height, || wit::next_block_height().into())
    }

    /// Loads a value from the `cell` cache or fetches it and stores it in the cache.
    fn fetch_value_through_cache<T>(cell: &Cell<Option<T>>, fetch: impl FnOnce() -> T) -> T
    where
        T: Clone,
    {
        let value = cell.take().unwrap_or_else(fetch);
        cell.set(Some(value.clone()));
        value
    }
}
