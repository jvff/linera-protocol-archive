// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::Storage;
use async_trait::async_trait;
use futures::lock::Mutex;
use std::{collections::HashMap, sync::Arc};
use zef_base::{
    chain::ChainState,
    crypto::HashValue,
    error::Error,
    messages::{Certificate, ChainId},
};

#[cfg(test)]
use zef_base::{committee::Committee, crypto::PublicKey, manager::ChainManager};

#[cfg(test)]
#[path = "unit_tests/memory_storage_tests.rs"]
mod memory_storage_tests;

/// Vanilla in-memory key-value store.
#[derive(Debug, Clone, Default)]
pub struct InMemoryStore {
    chains: HashMap<ChainId, Arc<ChainState>>,
    certificates: HashMap<HashValue, Arc<Certificate>>,
}

/// The corresponding vanilla client.
#[derive(Clone, Default)]
pub struct InMemoryStoreClient(Arc<Mutex<InMemoryStore>>);

impl InMemoryStoreClient {
    /// Create a distinct copy of the data.
    pub async fn copy(&self) -> Self {
        let store = self.0.clone().lock().await.clone();
        Self(Arc::new(Mutex::new(store)))
    }
}

#[async_trait]
impl Storage for InMemoryStoreClient {
    async fn read_chain_or_default(&mut self, id: ChainId) -> Result<Arc<ChainState>, Error> {
        let store = self.0.clone();
        let chain = store
            .lock()
            .await
            .chains
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Arc::new(ChainState::new(id)));
        Ok(chain)
    }

    async fn write_chain(&mut self, value: ChainState) -> Result<(), Error> {
        let store = self.0.clone();
        store
            .lock()
            .await
            .chains
            .insert(value.state.chain_id, Arc::new(value));
        Ok(())
    }

    async fn remove_chain(&mut self, id: ChainId) -> Result<(), Error> {
        let store = self.0.clone();
        store.lock().await.chains.remove(&id);
        Ok(())
    }

    async fn read_certificate(&mut self, hash: HashValue) -> Result<Arc<Certificate>, Error> {
        let store = self.0.clone();
        let value = store.lock().await.certificates.get(&hash).cloned();
        value.ok_or(Error::MissingCertificate { hash })
    }

    async fn write_certificate(&mut self, value: Certificate) -> Result<(), Error> {
        let store = self.0.clone();
        store
            .lock()
            .await
            .certificates
            .insert(value.hash, Arc::new(value));
        Ok(())
    }
}
