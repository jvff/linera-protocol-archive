// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the service.

use super::service_system_api as wit;
use linera_base::{
    abi::ServiceAbi,
    data_types::{Amount, BlockHeight, Timestamp},
    identifiers::{ApplicationId, ChainId, Owner},
};
use std::cell::Cell;

/// The runtime available during execution of a query.
pub struct ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    application_parameters: Cell<Option<Abi::Parameters>>,
    application_id: Cell<Option<ApplicationId<Abi>>>,
    chain_id: Cell<Option<ChainId>>,
    next_block_height: Cell<Option<BlockHeight>>,
    timestamp: Cell<Option<Timestamp>>,
    chain_balance: Cell<Option<Amount>>,
}

impl<Abi> Default for ServiceRuntime<Abi>
where
    Abi: ServiceAbi,
{
    fn default() -> Self {
        ServiceRuntime {
            application_parameters: Cell::new(None),
            application_id: Cell::new(None),
            chain_id: Cell::new(None),
            next_block_height: Cell::new(None),
            timestamp: Cell::new(None),
            chain_balance: Cell::new(None),
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
            application_id: clone_cell(&self.application_id),
            chain_id: clone_cell(&self.chain_id),
            next_block_height: clone_cell(&self.next_block_height),
            timestamp: clone_cell(&self.timestamp),
            chain_balance: clone_cell(&self.chain_balance),
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

    /// Returns the ID of the current application.
    pub fn application_id(&self) -> ApplicationId<Abi> {
        Self::fetch_value_through_cache(&self.application_id, || {
            ApplicationId::from(wit::application_id()).with_abi()
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

    /// Retrieves the current system time, i.e. the timestamp of the block in which this is called.
    pub fn system_time(&self) -> Timestamp {
        Self::fetch_value_through_cache(&self.timestamp, || wit::read_system_timestamp().into())
    }

    /// Returns the current chain balance.
    pub fn chain_balance(&self) -> Amount {
        Self::fetch_value_through_cache(&self.chain_balance, || wit::read_chain_balance().into())
    }

    /// Returns the balance of one of the chain owners.
    pub fn owner_balance(&self, owner: Owner) -> Amount {
        wit::read_owner_balance(owner.into()).into()
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
