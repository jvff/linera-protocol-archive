// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{super::ApplicationState, writable_system as system};
use futures::future;
use linera_sdk::{ApplicationId, ChainId, SessionId, SystemBalance, Timestamp};
use async_trait::async_trait;
use linera_views::{views::ViewError, common::{Batch, ContextFromDb, SimpleTypeIterator, KeyValueOperations, WriteOperation}};
use crate::boilerplate::writable_system;
use linera_views::views::{View, ContainerView};

#[derive(Clone)]
pub struct WasmContainer;

impl WasmContainer {
    pub fn new() -> Self {
        WasmContainer { }
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
    ) -> Result<Vec<(Vec<u8>,Vec<u8>)>, ViewError> {
        let future = system::FindStrippedKeyValues::new(key_prefix);
        future::poll_fn(|_context| future.poll().into()).await
    }

}

#[async_trait]
impl KeyValueOperations for WasmContainer {
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
        let key_values = self.find_stripped_key_values_by_prefix_load(key_prefix).await?;
        Ok(Self::KeyValueIterator::new(key_values))
    }

    async fn write_batch(&self, batch: Batch) -> Result<(), ViewError> {
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
        future::poll_fn(|_context| future.poll().into()).await
    }

}

pub type WasmContext = ContextFromDb<(), WasmContainer>;

trait WasmContextExt {
    fn new() -> Self;
}

impl WasmContextExt for WasmContext {
    fn new() -> Self {
        Self {
            db: WasmContainer::new(),
            base_key: Vec::new(),
            extra: (),
        }
    }
}

#[allow(dead_code)]
impl ApplicationState {
    /// Load the contract state and lock it for writes.
    pub async fn load_and_lock() -> Self {
        let future = system::Lock::new();
        future::poll_fn(|_context| future.poll().into()).await;
        Self::load_using().await
    }

    /// Helper function to load the contract state or create a new one if it doesn't exist.
    pub async fn load_using() -> Self {
        let context = WasmContext::new();
        Self::load(context).await.expect("Failed to load contract state")
    }

    /// Save the contract state and unlock it.
    pub async fn store_and_unlock(mut self) {
        self.save().await.expect("save operation failed");
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

/// Calls another application.
#[allow(dead_code)]
pub async fn call_application(
    authenticated: bool,
    application: ApplicationId,
    argument: &[u8],
    forwarded_sessions: Vec<SessionId>,
) -> Result<(Vec<u8>, Vec<SessionId>), String> {
    let forwarded_sessions: Vec<_> = forwarded_sessions
        .into_iter()
        .map(system::SessionId::from)
        .collect();

    let future = system::TryCallApplication::new(
        authenticated,
        application.into(),
        argument,
        &forwarded_sessions,
    );

    future::poll_fn(|_context| future.poll().into()).await
}

/// Calls another application's session.
#[allow(dead_code)]
pub async fn call_session(
    authenticated: bool,
    session: SessionId,
    argument: &[u8],
    forwarded_sessions: Vec<SessionId>,
) -> Result<(Vec<u8>, Vec<SessionId>), String> {
    let forwarded_sessions: Vec<_> = forwarded_sessions
        .into_iter()
        .map(system::SessionId::from)
        .collect();

    let future =
        system::TryCallSession::new(authenticated, session.into(), argument, &forwarded_sessions);

    future::poll_fn(|_context| future.poll().into()).await
}
