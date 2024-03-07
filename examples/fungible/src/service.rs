// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::FungibleToken;
use async_graphql::{ComplexObject, EmptySubscription, Request, Response, Schema};
use fungible::{FungibleTokenAbi as Abi, Operation};
use linera_sdk::{
    base::WithServiceAbi, graphql::GraphQLMutationRoot, Service, ServiceRuntime, ViewStateStorage,
};
use std::sync::Arc;
use thiserror::Error;

linera_sdk::service!(FungibleToken);

impl WithServiceAbi for FungibleToken {
    type Abi = Abi;
}

impl Service for FungibleToken {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;

    async fn handle_query(
        self: Arc<Self>,
        _runtime: &ServiceRuntime<Abi>,
        request: Request,
    ) -> Result<Response, Self::Error> {
        let schema =
            Schema::build(self.clone(), Operation::mutation_root(), EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}

// Implements additional fields not derived from struct members of FungibleToken.
#[ComplexObject]
impl FungibleToken {
    async fn ticker_symbol(&self) -> Result<String, async_graphql::Error> {
        let runtime = ServiceRuntime::<Abi>::default();
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
