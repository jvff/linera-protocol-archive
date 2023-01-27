// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_arch = "wasm32")]

mod state;
mod types;

use self::state::FungibleToken;
use async_trait::async_trait;
use linera_sdk::{QueryContext, Service};
use thiserror::Error;
use crate::boilerplate::system_api::ReadableWasmContext;
/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = FungibleToken<ReadableWasmContext>;

#[async_trait]
impl Service for ApplicationState {
    type Error = Error;

    async fn query_application(
        &self,
        _context: &QueryContext,
        argument: &[u8],
    ) -> Result<Vec<u8>, Self::Error> {
        let account = bcs::from_bytes(argument)?;
        let balance = self.balance(&account).await;

        Ok(bcs::to_bytes(&balance).expect("Serialization of `u128` should not fail"))
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid account query.
    #[error("Invalid account specified in query parameter")]
    InvalidAccount(#[from] bcs::Error),
}

#[path = "../boilerplate/service/mod.rs"]
mod boilerplate;
