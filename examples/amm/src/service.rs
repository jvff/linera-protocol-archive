// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{AmmAbi as Abi, AmmError, Operation};
use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use linera_sdk::{base::WithServiceAbi, Service, ServiceRuntime, ViewStateStorage};
use std::sync::Arc;

linera_sdk::service!(Amm);

impl WithServiceAbi for Amm {
    type Abi = Abi;
}

impl Service for Amm {
    type Error = AmmError;
    type Storage = ViewStateStorage<Self>;

    async fn handle_query(
        self: Arc<Self>,
        _runtime: &ServiceRuntime<Abi>,
        request: Request,
    ) -> Result<Response, AmmError> {
        let schema = Schema::build(self.clone(), MutationRoot, EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}

struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn operation(&self, operation: Operation) -> Vec<u8> {
        bcs::to_bytes(&operation).unwrap()
    }
}
