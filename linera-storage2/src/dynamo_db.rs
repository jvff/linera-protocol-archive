// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{chain::ChainStateView, Store};
use async_trait::async_trait;
use linera_base::{
    crypto::HashValue,
    messages::{Certificate, ChainId},
};
use linera_views::{
    dynamo_db::{CreateTableError, DynamoDbContext, DynamoDbContextError, TableName},
    views::{MapView, View},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

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
}

impl DynamoDbStore {
    pub async fn new(table: TableName) -> Result<Self, CreateTableError> {
        let dummy_lock = Arc::new(Mutex::new(()));
        let (context, _) =
            DynamoDbContext::new(table, dummy_lock.lock_owned().await, vec![], ()).await?;
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
        let lock = self
            .0
            .lock()
            .await
            .locks
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        log::trace!("Acquiring lock on {:?}", id);
        let table_name = "linera".parse().expect("Invalid hard-coded table name");
        let key_prefix = bcs::to_bytes(&BaseKey::ChainState(id))?;
        let (context, _) =
            DynamoDbContext::new(table_name, lock.lock_owned().await, key_prefix, id).await?;
        ChainStateView::load(context).await
    }

    async fn read_certificate(&self, hash: HashValue) -> Result<Certificate, DynamoDbContextError> {
        let store = self.0.lock().await;
        let mut certificates = MapView::load(store.context.clone()).await?;
        certificates
            .get(&BaseKey::Certificate(hash))
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
        certificates.insert(certificate.hash, certificate);
        certificates.commit().await
    }
}
