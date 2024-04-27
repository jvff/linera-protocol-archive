// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use std::sync::Arc;

use amm::{Operation, Parameters};
use async_graphql::{EmptySubscription, Request, Response, Schema};
use linera_sdk::{base::WithServiceAbi, graphql::GraphQLMutationRoot, Service, ServiceRuntime};

use self::state::Amm;

pub struct AmmService {
    state: Arc<Amm>,
}

linera_sdk::service!(AmmService);

impl WithServiceAbi for AmmService {
    type Abi = amm::AmmAbi;
}

impl Service for AmmService {
    type State = Amm;
    type Parameters = Parameters;

    async fn new(state: Self::State, _runtime: ServiceRuntime<Self>) -> Self {
        AmmService {
            state: Arc::new(state),
        }
    }

    async fn handle_query(&self, request: Request) -> Response {
        let schema = Schema::build(
            self.state.clone(),
            Operation::mutation_root(),
            EmptySubscription,
        )
        .finish();
        schema.execute(request).await
    }
}
