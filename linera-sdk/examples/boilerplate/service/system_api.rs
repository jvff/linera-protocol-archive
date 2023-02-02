// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{super::ApplicationState, queryable_system as system};
use async_trait::async_trait;
use futures::future;
use linera_sdk::{ApplicationId, ChainId, SystemBalance, Timestamp};
use linera_views::{
    common::{Batch, ContextFromDb, KeyValueOperations, SimpleTypeIterator},
    views::{View, ViewError},
};
use std::task::Poll;

#[derive(Clone)]
pub struct ReadableWasmContainer;

impl ReadableWasmContainer {
    pub fn new() -> Self {
        ReadableWasmContainer {}
    }

    async fn find_stripped_keys_by_prefix_load(
        &self,
        key_prefix: &[u8],
    ) -> Result<Vec<Vec<u8>>, ViewError> {
        let future = system::FindStrippedKeys::new(key_prefix);
        future::poll_fn(|_context| future.poll().into()).await
    }

    async fn find_stripped_key_values_by_prefix_load(
        &self,
        key_prefix: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>, ViewError> {
        let future = system::FindStrippedKeyValues::new(key_prefix);
        future::poll_fn(|_context| future.poll().into()).await
    }
}

#[async_trait]
impl KeyValueOperations for ReadableWasmContainer {
    type Error = ViewError;
    type KeyIterator = SimpleTypeIterator<Vec<u8>, ViewError>;
    type KeyValueIterator = SimpleTypeIterator<(Vec<u8>, Vec<u8>), ViewError>;

    async fn read_key_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, ViewError> {
        let future = system::ReadKeyBytes::new(key);
        future::poll_fn(|_context| future.poll().into()).await
    }

    async fn find_stripped_keys_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Self::KeyIterator, ViewError> {
        let keys = self.find_stripped_keys_by_prefix_load(key_prefix).await?;
        Ok(Self::KeyIterator::new(keys))
    }

    async fn find_stripped_key_values_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Self::KeyValueIterator, ViewError> {
        let key_values = self
            .find_stripped_key_values_by_prefix_load(key_prefix)
            .await?;
        Ok(Self::KeyValueIterator::new(key_values))
    }

    async fn write_batch(&self, _batch: Batch) -> Result<(), ViewError> {
        Ok(())
    }
}

pub type ReadableWasmContext = ContextFromDb<(), ReadableWasmContainer>;

trait ReadableWasmContextExt {
    fn new() -> Self;
}

impl ReadableWasmContextExt for ReadableWasmContext {
    fn new() -> Self {
        Self {
            db: ReadableWasmContainer::new(),
            base_key: Vec::new(),
            extra: (),
        }
    }
}

impl ApplicationState {
    /// Load the service state, without locking it for writes.
    pub async fn lock_and_load() -> Self {
        let future = system::Lock::new();
        future::poll_fn(|_context| -> Poll<Result<(), ViewError>> { future.poll().into() })
            .await
            .expect("Failed to lock contract state");
        Self::load_using().await
    }

    /// Load the service state, without locking it for writes.
    pub async fn unlock(self) {
        let future = system::Unlock::new();
        future::poll_fn(|_context| future.poll().into()).await;
    }

    /// Helper function to load the service state or create a new one if it doesn't exist.
    pub async fn load_using() -> Self {
        let context = ReadableWasmContext::new();
        Self::load(context)
            .await
            .expect("Failed to load contract state")
    }
}

/// Retrieve the current chain ID.
#[allow(dead_code)]
pub fn current_chain_id() -> ChainId {
    ChainId(system::chain_id().into())
}

/// Retrieve the current application ID.
#[allow(dead_code)]
pub fn current_application_id() -> ApplicationId {
    system::application_id().into()
}

/// Retrieve the current system balance.
#[allow(dead_code)]
pub fn current_system_balance() -> SystemBalance {
    system::read_system_balance().into()
}

/// Retrieves the current system time.
#[allow(dead_code)]
pub fn current_system_time() -> Timestamp {
    system::read_system_timestamp().into()
}
