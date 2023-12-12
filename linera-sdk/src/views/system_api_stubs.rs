// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Stubs for the interface of the system API available for views.
//!
//! This allows building the crate for non-Wasm targets.

use async_trait::async_trait;
use linera_views::{
    batch::Batch,
    common::{ContextFromStore, KeyValueStore},
    views::ViewError,
};

const MESSAGE: &str = "Attempt to call a contract system API when not running as a Wasm guest";

/// A type to interface with the key value storage provided to applications.
#[derive(Default, Clone)]
pub struct AppStateStore;

#[async_trait]
impl KeyValueStore for AppStateStore {
    // The AppStateStore of the system_api does not have limits
    // on the size of its values.
    const MAX_VALUE_SIZE: usize = usize::MAX;
    const MAX_KEY_SIZE: usize = usize::MAX;

    type Error = ViewError;
    type Keys = Vec<Vec<u8>>;
    type KeyValues = Vec<(Vec<u8>, Vec<u8>)>;

    fn max_stream_queries(&self) -> usize {
        panic!("{MESSAGE}");
    }

    async fn contains_key(&self, _key: &[u8]) -> Result<bool, Self::Error> {
        panic!("{MESSAGE}");
    }

    async fn read_multi_values_bytes(
        &self,
        _keys: Vec<Vec<u8>>,
    ) -> Result<Vec<Option<Vec<u8>>>, Self::Error> {
        panic!("{MESSAGE}");
    }

    async fn read_value_bytes(&self, _key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        panic!("{MESSAGE}");
    }

    async fn find_keys_by_prefix(&self, _key_prefix: &[u8]) -> Result<Self::Keys, ViewError> {
        panic!("{MESSAGE}");
    }

    async fn find_key_values_by_prefix(
        &self,
        _key_prefix: &[u8],
    ) -> Result<Self::KeyValues, ViewError> {
        panic!("{MESSAGE}");
    }

    async fn write_batch(&self, _batch: Batch, _base_key: &[u8]) -> Result<(), ViewError> {
        panic!("{MESSAGE}");
    }

    async fn clear_journal(&self, _base_key: &[u8]) -> Result<(), Self::Error> {
        panic!("{MESSAGE}");
    }
}

/// Implementation of [`linera_views::common::Context`] to be used for data storage
/// by Linera applications.
pub type ViewStorageContext = ContextFromStore<(), AppStateStore>;
