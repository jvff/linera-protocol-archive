// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_arch = "wasm32")]

mod state;

use self::state::CrowdFunding;
use crate::boilerplate::system_api::ReadableWasmContext;
use async_trait::async_trait;
use linera_sdk::{QueryContext, Service};
use linera_views::views::ViewError;
use serde::Deserialize;
use thiserror::Error;

/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = CrowdFunding<ReadableWasmContext>;

#[async_trait]
impl Service for ApplicationState {
    type Error = Error;

    async fn query_application(
        &self,
        _context: &QueryContext,
        argument: &[u8],
    ) -> Result<Vec<u8>, Self::Error> {
        let query = bcs::from_bytes(argument)?;

        let response = match query {
            Query::Status => bcs::to_bytes(&self.status.get()),
            Query::Pledged => bcs::to_bytes(&self.pledged().await),
            Query::Target => bcs::to_bytes(&self.parameters().target),
            Query::Deadline => bcs::to_bytes(&self.parameters().deadline),
            Query::Owner => bcs::to_bytes(&self.parameters().owner),
        }?;

        Ok(response)
    }
}

impl ApplicationState {
    /// Returns the total amount of tokens pledged to this campaign.
    async fn pledged(&self) -> u128 {
        let mut total_pledge = 0;
        self.pledges
            .for_each_raw_index_value(|_index: Vec<u8>, value: u128| -> Result<(), ViewError> {
                total_pledge += value;
                Ok(())
            })
            .await
            .expect("for_each_raw_index_value failed");
        total_pledge
    }
}

/// Queries that can be made to the [`CrowdFunding`] application service.
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Query {
    /// The current [`Status`] of the crowd-funding campaign.
    Status,
    /// The total amount pledged to the crowd-funding campaign.
    Pledged,
    /// The crowd-funding campaign's target.
    Target,
    /// The crowd-funding campaign's deadline.
    Deadline,
    /// The recipient of the pledged amount.
    Owner,
}

/// An error that can occur during the service execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid account query.
    #[error("Invalid account specified in query parameter")]
    InvalidQuery(#[from] bcs::Error),
}

#[path = "../boilerplate/service/mod.rs"]
mod boilerplate;

// Work-around to pretend that `fungible` is an external crate, exposing the Fungible Token
// application's interface.
#[path = "../fungible/interface.rs"]
#[allow(dead_code)]
mod fungible;
