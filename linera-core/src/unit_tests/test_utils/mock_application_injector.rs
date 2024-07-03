// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Test helpers to allow injecting [`MockApplication`]s into tests.
//!
//! The [`MockApplicationInjector`] wraps a [`Storage`][`linera_storage::Storage`] implementation
//! so that it can intercept requests to load contracts and services, and provide custom
//! [`MockApplication`] instances. This allows testing a worker with custom applications
//! without having to write, build and publish Wasm applications.

use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use dashmap::DashMap;
use linera_base::{
    crypto::CryptoHash,
    data_types::{BlobState, HashedBlob},
    identifiers::{BlobId, ChainId},
};
use linera_chain::{
    data_types::{Certificate, HashedCertificateValue},
    ChainStateView,
};
use linera_execution::{test_utils::MockApplication, UserApplicationId, WasmRuntime};
use linera_storage::{ChainRuntimeContext, Clock};
use linera_views::{
    common::{ContextFromStore, KeyValueStore},
    views::ViewError,
};

/// A wrapper type that allows injecting custom [`MockApplication`]s into the wrapped
/// `Storage`.
#[derive(Clone)]
pub struct MockApplicationInjector<Storage> {
    storage: Storage,
    applications: Arc<DashMap<UserApplicationId, MockApplication>>,
}

impl<Storage> From<Storage> for MockApplicationInjector<Storage> {
    fn from(storage: Storage) -> Self {
        MockApplicationInjector {
            storage,
            applications: Arc::new(DashMap::new()),
        }
    }
}

impl<Storage> MockApplicationInjector<Storage> {
    pub fn applications(&self) -> &Arc<DashMap<UserApplicationId, MockApplication>> {
        &self.applications
    }
}

#[async_trait]
impl<Store, Storage> linera_storage::Storage for MockApplicationInjector<Storage>
where
    Store: KeyValueStore + Clone + Sync + 'static,
    Store::Error: From<bcs::Error> + Error + Send + Sync + 'static,
    Storage: linera_storage::Storage<Context = ContextFromStore<(), Store>, ContextError = Store::Error>
        + Clone
        + Send
        + Sync
        + 'static,
    ViewError: From<Store::Error>,
{
    type Context = ContextFromStore<ChainRuntimeContext<Self>, Store>;
    type ContextError = Storage::ContextError;

    fn clock(&self) -> &dyn Clock {
        self.storage.clock()
    }

    async fn load_chain(
        &self,
        chain_id: ChainId,
    ) -> Result<ChainStateView<Self::Context>, ViewError> {
        self.storage.load_chain(chain_id).await
    }

    async fn contains_hashed_certificate_value(
        &self,
        value_hash: CryptoHash,
    ) -> Result<bool, ViewError> {
        self.storage
            .contains_hashed_certificate_value(value_hash)
            .await
    }

    async fn contains_blob(&self, blob_id: BlobId) -> Result<bool, ViewError> {
        self.contains_blob(blob_id).await
    }

    async fn read_hashed_certificate_value(
        &self,
        value_hash: CryptoHash,
    ) -> Result<HashedCertificateValue, ViewError> {
        self.read_hashed_certificate_value(value_hash).await
    }

    async fn read_hashed_blob(&self, blob_id: BlobId) -> Result<HashedBlob, ViewError> {
        self.read_hashed_blob(blob_id).await
    }

    async fn read_blob_state(&self, blob_id: BlobId) -> Result<BlobState, ViewError> {
        self.read_blob_state(blob_id).await
    }

    async fn read_hashed_certificate_values_downward(
        &self,
        first_value_hash: CryptoHash,
        count: u32,
    ) -> Result<std::vec::Vec<HashedCertificateValue>, ViewError> {
        self.read_hashed_certificate_values_downward(first_value_hash, count)
            .await
    }

    async fn write_hashed_certificate_value(
        &self,
        value: &HashedCertificateValue,
    ) -> Result<(), ViewError> {
        self.write_hashed_certificate_value(value).await
    }

    async fn write_hashed_blob(
        &self,
        blob: &HashedBlob,
        hash: &CryptoHash,
    ) -> Result<(), ViewError> {
        self.write_hashed_blob(blob, hash).await
    }

    async fn write_hashed_certificate_values(
        &self,
        values: &[HashedCertificateValue],
    ) -> Result<(), ViewError> {
        self.write_hashed_certificate_values(values).await
    }

    async fn write_hashed_blobs(
        &self,
        blobs: &[HashedBlob],
        hash: &CryptoHash,
    ) -> Result<(), ViewError> {
        self.write_hashed_blobs(blobs, hash).await
    }

    async fn contains_certificate(&self, certificate_hash: CryptoHash) -> Result<bool, ViewError> {
        self.contains_certificate(certificate_hash).await
    }

    async fn read_certificate(
        &self,
        certificate_hash: CryptoHash,
    ) -> Result<Certificate, ViewError> {
        self.read_certificate(certificate_hash).await
    }

    async fn write_certificate(&self, certificate: &Certificate) -> Result<(), ViewError> {
        self.write_certificate(certificate).await
    }

    async fn write_certificates(&self, certificate: &[Certificate]) -> Result<(), ViewError> {
        self.write_certificates(certificate).await
    }

    fn wasm_runtime(&self) -> Option<WasmRuntime> {
        self.storage.wasm_runtime()
    }
}
