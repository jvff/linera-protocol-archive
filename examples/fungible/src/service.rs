// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::FungibleToken;
use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use fungible::Operation;
use linera_sdk::{
    base::{AccountOwner, Amount, WithServiceAbi},
    graphql::GraphQLMutationRoot,
    views::MapView,
    Service, ServiceRuntime, ViewStateStorage,
};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Clone)]
pub struct FungibleTokenService {
    state: Arc<FungibleToken>,
    runtime: Arc<Mutex<ServiceRuntime<fungible::FungibleTokenAbi>>>,
}

linera_sdk::service!(FungibleTokenService);

impl WithServiceAbi for FungibleTokenService {
    type Abi = fungible::FungibleTokenAbi;
}

impl Service for FungibleTokenService {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;
    type State = FungibleToken;

    async fn new(
        state: Self::State,
        runtime: ServiceRuntime<Self::Abi>,
    ) -> Result<Self, Self::Error> {
        Ok(FungibleTokenService {
            state: Arc::new(state),
            runtime: Arc::new(Mutex::new(runtime)),
        })
    }

    async fn handle_query(&self, request: Request) -> Result<Response, Self::Error> {
        let schema =
            Schema::build(self.clone(), Operation::mutation_root(), EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}

#[Object]
impl FungibleTokenService {
    async fn accounts(&self) -> &MapView<AccountOwner, Amount> {
        &self.state.accounts
    }

    async fn ticker_symbol(&self) -> Result<String, async_graphql::Error> {
        let runtime = self
            .runtime
            .try_lock()
            .expect("Services only run in a single-thread");
        Ok(runtime.application_parameters().ticker_symbol)
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid query argument; could not deserialize GraphQL request.
    #[error(
        "Invalid query argument; Fungible application only supports JSON encoded GraphQL queries"
    )]
    InvalidQuery(#[from] serde_json::Error),
}
