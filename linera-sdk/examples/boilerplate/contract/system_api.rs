// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{super::ApplicationState, writable_system as system};
use futures::future;
use linera_sdk::{ApplicationId, ChainId, SystemBalance};
use std::future::Future;
use async_trait::async_trait;
use linera_views::{views::ViewError, common::{Batch, SimpleTypeIterator, KeyValueOperations, WriteOperation}};
use crate::boilerplate::writable_system;
use crate::boilerplate::writable_system::{PollReadKeyBytes, PollFindStrippedKeys, PollFindStrippedKeyValues, PollWriteBatch};

pub struct WasmContainer {
}

#[async_trait]
impl KeyValueOperations for WasmContainer {
    type Error = ViewError;
    type KeyIterator = SimpleTypeIterator<Vec<u8>, ViewError>;
    type KeyValueIterator = SimpleTypeIterator<(Vec<u8>, Vec<u8>), ViewError>;

    async fn read_key_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, ViewError> {
        let future = system::ReadKeyBytes::new(key);
        loop {
            let answer : PollReadKeyBytes = future::poll_fn(|_context| future.poll().into()).await;
            match answer {
                PollReadKeyBytes::Ready(answer) => {
                    return match answer {
                        Ok(answer) => Ok(answer),
                        Err(error) => Err(ViewError::WasmHostGuestError(error)),
                    };
                },
                PollReadKeyBytes::Pending => {},
            }
        }
    }

    async fn find_stripped_keys_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Self::KeyIterator, ViewError> {
        let future = system::FindStrippedKeys::new(key_prefix);
        loop {
            let answer : PollFindStrippedKeys = future::poll_fn(|_context| future.poll().into()).await;
            match answer {
                PollFindStrippedKeys::Ready(answer) => {
                    return match answer {
                        Ok(keys) => Ok(Self::KeyIterator::new(keys)),
                        Err(error) => Err(ViewError::WasmHostGuestError(error)),
                    };
                },
                PollFindStrippedKeys::Pending => {},
            }
        }
    }

    async fn find_stripped_key_values_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Self::KeyValueIterator, ViewError> {
        let future = system::FindStrippedKeyValues::new(key_prefix);
        loop {
            let answer : PollFindStrippedKeyValues = future::poll_fn(|_context| future.poll().into()).await;
            match answer {
                PollFindStrippedKeyValues::Ready(answer) => {
                    return match answer {
                        Ok(key_values) => Ok(Self::KeyValueIterator::new(key_values)),
                        Err(error) => Err(ViewError::WasmHostGuestError(error)),
                    };
                },
                PollFindStrippedKeyValues::Pending => {},
            }
        }
    }

    async fn write_batch(&mut self, batch: Batch) -> Result<(), ViewError> {
        let mut list_oper = Vec::new();
        for op in &batch.operations {
            match op {
                WriteOperation::Delete { key } => {
                    list_oper.push(writable_system::WriteOperation::Delete(key));
                },
                WriteOperation::Put { key, value } => list_oper.push(writable_system::WriteOperation::Put((key,value))),
                WriteOperation::DeletePrefix { key_prefix } => list_oper.push(writable_system::WriteOperation::Deleteprefix(&key_prefix)),
            }
        }
        let future = system::WriteBatch::new(&list_oper);
        loop {
            let answer : PollWriteBatch = future::poll_fn(|_context| future.poll().into()).await;
            match answer {
                PollWriteBatch::Ready(answer) => {
                    return match answer {
                        Ok(_) => Ok(()),
                        Err(error) => Err(ViewError::WasmHostGuestError(error)),
                    };
                },
                PollWriteBatch::Pending => {},
            }
        }
    }

}



#[allow(dead_code)]
impl ApplicationState {
    /// Load the contract state, without locking it for writes.
    pub async fn load() -> Self {
        let future = system::Load::new();
        Self::load_using(future::poll_fn(|_context| future.poll().into())).await
    }

    /// Load the contract state and lock it for writes.
    pub async fn load_and_lock() -> Self {
        let future = system::LoadAndLock::new();
        Self::load_using(future::poll_fn(|_context| future.poll().into())).await
    }

    /// Helper function to load the contract state or create a new one if it doesn't exist.
    pub async fn load_using(future: impl Future<Output = Result<Vec<u8>, String>>) -> Self {
        let bytes = future.await.expect("Failed to load contract state");
        if bytes.is_empty() {
            Self::default()
        } else {
            bcs::from_bytes(&bytes).expect("Invalid contract state")
        }
    }

    /// Save the contract state and unlock it.
    pub async fn store_and_unlock(self) {
        system::store_and_unlock(&bcs::to_bytes(&self).expect("State serialization failed"));
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
}
