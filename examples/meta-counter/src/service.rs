// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::MetaCounter;
use async_graphql::{Request, Response};
use linera_sdk::{base::WithServiceAbi, Service, ServiceRuntime, SimpleStateStorage};
use meta_counter::MetaCounterAbi as Abi;
use std::sync::Arc;
use thiserror::Error;

linera_sdk::service!(MetaCounter);

impl WithServiceAbi for MetaCounter {
    type Abi = Abi;
}

impl Service for MetaCounter {
    type Error = Error;
    type Storage = SimpleStateStorage<Self>;

    async fn handle_query(
        self: Arc<Self>,
        runtime: &ServiceRuntime<Abi>,
        request: Request,
    ) -> Result<Response, Self::Error> {
        let counter_id = runtime.application_parameters();
        Self::query_application(counter_id, &request)
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal query failed: {0}")]
    InternalQuery(String),

    /// Invalid query argument in meta-counter app: could not deserialize GraphQL request.
    #[error("Invalid query argument in meta-counter app: could not deserialize GraphQL request.")]
    InvalidQuery(#[from] serde_json::Error),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::InternalQuery(s)
    }
}
