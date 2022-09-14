// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{chain::ChainStateView, Store};
use async_trait::async_trait;
use futures::Future;
use linera_base::{
    crypto::HashValue,
    messages::{Certificate, ChainId},
};
use linera_views::{
    dynamo_db::{
        Config, CreateTableError, DynamoDbContext, DynamoDbContextError, LocalStackError,
        TableName, TableStatus,
    },
    views::{MapView, View},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, OwnedMutexGuard};

struct DynamoDbStore {
    context: DynamoDbContext<()>,
    locks: HashMap<ChainId, Arc<Mutex<()>>>,
}

#[derive(Clone)]
pub struct DynamoDbStoreClient(Arc<Mutex<DynamoDbStore>>);

impl DynamoDbStoreClient {
    pub async fn new(table: TableName) -> Result<Self, CreateTableError> {
        Ok(DynamoDbStoreClient(Arc::new(Mutex::new(
            DynamoDbStore::new(table).await?,
        ))))
    }

    pub async fn from_config(
        config: impl Into<Config>,
        table: TableName,
    ) -> Result<Self, CreateTableError> {
        Ok(DynamoDbStoreClient(Arc::new(Mutex::new(
            DynamoDbStore::from_config(config.into(), table).await?,
        ))))
    }

    pub async fn with_localstack(table: TableName) -> Result<Self, LocalStackError> {
        Ok(DynamoDbStoreClient(Arc::new(Mutex::new(
            DynamoDbStore::with_localstack(table).await?,
        ))))
    }
}

impl DynamoDbStore {
    pub async fn new(table: TableName) -> Result<Self, CreateTableError> {
        Self::with_context(|lock, key_prefix, extra| {
            DynamoDbContext::new(table, lock, key_prefix, extra)
        })
        .await
    }

    pub async fn from_config(config: Config, table: TableName) -> Result<Self, CreateTableError> {
        Self::with_context(|lock, key_prefix, extra| {
            DynamoDbContext::from_config(config, table, lock, key_prefix, extra)
        })
        .await
    }

    pub async fn with_localstack(table: TableName) -> Result<Self, LocalStackError> {
        Self::with_context(|lock, key_prefix, extra| {
            DynamoDbContext::with_localstack(table, lock, key_prefix, extra)
        })
        .await
    }

    async fn with_context<F, E>(
        create_context: impl FnOnce(OwnedMutexGuard<()>, Vec<u8>, ()) -> F,
    ) -> Result<Self, E>
    where
        F: Future<Output = Result<(DynamoDbContext<()>, TableStatus), E>>,
    {
        let dummy_lock = Arc::new(Mutex::new(())).lock_owned().await;
        let empty_prefix = vec![];
        let dummy_extra = ();
        let (context, _) = create_context(dummy_lock, empty_prefix, dummy_extra).await?;
        Ok(Self {
            context,
            locks: HashMap::new(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
enum BaseKey {
    ChainState(ChainId),
    Certificate(HashValue),
}

#[async_trait]
impl Store for DynamoDbStoreClient {
    type Context = DynamoDbContext<ChainId>;
    type Error = DynamoDbContextError;

    async fn load_chain(
        &self,
        id: ChainId,
    ) -> Result<ChainStateView<Self::Context>, DynamoDbContextError> {
        let mut store = self.0.lock().await;
        let lock = store
            .locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        log::trace!("Acquiring lock on {:?}", id);
        dbg!(id);
        let chain_context = store.context.clone_with_sub_scope(
            lock.lock_owned().await,
            &BaseKey::ChainState(id),
            id,
        );
        ChainStateView::load(chain_context).await
    }

    async fn read_certificate(&self, hash: HashValue) -> Result<Certificate, DynamoDbContextError> {
        let store = self.0.lock().await;
        let mut certificates = MapView::load(store.context.clone()).await?;
        certificates
            .get(&BaseKey::Certificate(dbg!(hash)))
            .await?
            .ok_or_else(|| {
                DynamoDbContextError::NotFound(format!("certificate for hash {:?}", hash))
            })
    }

    async fn write_certificate(
        &self,
        certificate: Certificate,
    ) -> Result<(), DynamoDbContextError> {
        let store = self.0.lock().await;
        let mut certificates = MapView::load(store.context.clone()).await?;
        certificates.insert(BaseKey::Certificate(dbg!(certificate.hash)), certificate);
        certificates.commit(&mut ()).await
    }
}
