// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functions and types to interface with the system API available to application views.

pub use self::linera::app::view_system_api as wit;
use crate::util::yield_once;
use async_trait::async_trait;
use linera_base::ensure;
use linera_views::{
    batch::{Batch, WriteOperation},
    common::{ContextFromStore, KeyValueStore},
    views::ViewError,
};

// Import the system interface.
wit_bindgen::generate!({
    path: "linera-sdk/wit",
    inline:
        "package linera:app-gen;\
        world view-system-api-only { import linera:app/view-system-api; }",
    world: "view-system-api-only",
});

/// We need to have a maximum key size that handles all possible underlying
/// sizes. The constraint so far is DynamoDb which has a key length of 1024.
/// That key length is decreased by 4 due to the use of a value splitting.
/// Then the `KeyValueStore` needs to handle some base_key and so we
/// reduce to 900. Depending on the size, the error can occur in system_api
/// or in the `KeyValueStoreView`.
const MAX_KEY_SIZE: usize = 900;

/// A type to interface with the key value storage provided to applications.
#[derive(Default, Clone)]
pub struct AppStateStore;

impl AppStateStore {
    async fn find_keys_by_prefix_load(&self, key_prefix: &[u8]) -> Vec<Vec<u8>> {
        let promise = wit::find_keys_new(key_prefix);
        yield_once().await;
        wit::find_keys_wait(promise)
    }

    async fn find_key_values_by_prefix_load(&self, key_prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let promise = wit::find_key_values_new(key_prefix);
        yield_once().await;
        wit::find_key_values_wait(promise)
    }
}

#[async_trait]
impl KeyValueStore for AppStateStore {
    // The AppStateStore of the system_api does not have limits
    // on the size of its values.
    const MAX_VALUE_SIZE: usize = usize::MAX;
    const MAX_KEY_SIZE: usize = MAX_KEY_SIZE;
    type Error = ViewError;
    type Keys = Vec<Vec<u8>>;
    type KeyValues = Vec<(Vec<u8>, Vec<u8>)>;

    fn max_stream_queries(&self) -> usize {
        1
    }

    async fn contains_key(&self, key: &[u8]) -> Result<bool, Self::Error> {
        ensure!(key.len() <= Self::MAX_KEY_SIZE, ViewError::KeyTooLong);
        let promise = wit::contains_key_new(key);
        yield_once().await;
        Ok(wit::contains_key_wait(promise))
    }

    async fn read_multi_values_bytes(
        &self,
        keys: Vec<Vec<u8>>,
    ) -> Result<Vec<Option<Vec<u8>>>, Self::Error> {
        for key in &keys {
            ensure!(key.len() <= Self::MAX_KEY_SIZE, ViewError::KeyTooLong);
        }
        let promise = wit::read_multi_values_bytes_new(keys.as_slice());
        yield_once().await;
        Ok(wit::read_multi_values_bytes_wait(promise))
    }

    async fn read_value_bytes(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        ensure!(key.len() <= Self::MAX_KEY_SIZE, ViewError::KeyTooLong);
        let promise = wit::read_value_bytes_new(key);
        yield_once().await;
        Ok(wit::read_value_bytes_wait(promise))
    }

    async fn find_keys_by_prefix(&self, key_prefix: &[u8]) -> Result<Self::Keys, ViewError> {
        ensure!(
            key_prefix.len() <= Self::MAX_KEY_SIZE,
            ViewError::KeyTooLong
        );
        let keys = self.find_keys_by_prefix_load(key_prefix).await;
        Ok(keys)
    }

    async fn find_key_values_by_prefix(
        &self,
        key_prefix: &[u8],
    ) -> Result<Self::KeyValues, ViewError> {
        ensure!(
            key_prefix.len() <= Self::MAX_KEY_SIZE,
            ViewError::KeyTooLong
        );
        let key_values = self.find_key_values_by_prefix_load(key_prefix).await;
        Ok(key_values)
    }

    async fn write_batch(&self, batch: Batch, _base_key: &[u8]) -> Result<(), ViewError> {
        let mut operations = Vec::new();
        for operation in batch.operations {
            match operation {
                WriteOperation::Delete { key } => {
                    ensure!(key.len() <= Self::MAX_KEY_SIZE, ViewError::KeyTooLong);
                    operations.push(wit::WriteOperation::Delete(key));
                }
                WriteOperation::Put { key, value } => {
                    ensure!(key.len() <= Self::MAX_KEY_SIZE, ViewError::KeyTooLong);
                    operations.push(wit::WriteOperation::Put((key, value)))
                }
                WriteOperation::DeletePrefix { key_prefix } => {
                    ensure!(
                        key_prefix.len() <= Self::MAX_KEY_SIZE,
                        ViewError::KeyTooLong
                    );
                    operations.push(wit::WriteOperation::DeletePrefix(key_prefix))
                }
            }
        }
        wit::write_batch(&operations);
        Ok(())
    }

    async fn clear_journal(&self, _base_key: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Implementation of [`linera_views::common::Context`] to be used for data storage
/// by Linera applications.
pub type ViewStorageContext = ContextFromStore<(), AppStateStore>;
