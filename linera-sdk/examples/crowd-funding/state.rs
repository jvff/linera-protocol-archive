// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use fungible::AccountOwner;
use linera_sdk::{ApplicationId, Timestamp};
use serde::{Deserialize, Serialize};
use linera_views::register_view::RegisterView;
use linera_views::map_view::MapView;
use linera_views::views::ContainerView;
use linera_views::common::Context;
use linera_views::views::View;

/// The parameters required to create a crowd-funding campaign.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Parameters {
    /// The receiver of the pledges of a successful campaign.
    pub owner: AccountOwner,
    /// The token to use for pledges.
    pub token: ApplicationId,
    /// The deadline of the campaign, after which it can be cancelled if it hasn't met its target.
    pub deadline: Timestamp,
    /// The funding target of the campaign.
    pub target: u128,
}

/// The status of a crowd-funding campaign.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum Status {
    /// The campaign is active and can receive pledges.
    #[default]
    Active,
    /// The campaign has ended successfully and still receive additional pledges.
    Complete,
    /// The campaign was cancelled, all pledges have been returned and no more pledges can be made.
    Cancelled,
}

/// The crowd-funding campaign's state.
#[derive(ContainerView)]
pub struct CrowdFunding<C> {
    /// The status of the campaign.
    pub status: RegisterView<C,Status>,
    /// The map of pledges that will be collected if the campaign succeeds.
    pub pledges: MapView<C, AccountOwner, u128>,
    /// The parameters that determine the details the campaign.
    pub parameters: RegisterView<C, Option<Parameters>>,
}

#[allow(dead_code)]
impl Status {
    /// Returns `true` if the campaign status is [`Status::Complete`].
    pub fn is_complete(&self) -> bool {
        matches!(self, Status::Complete)
    }
}

impl<C> CrowdFunding<C> {
    /// Retrieves the campaign [`Parameters`] stored in the application's state.
    pub fn parameters(&self) -> &Parameters {
        self.parameters
            .get()
            .expect("Application was not initialized")
    }
}

// Work-around to pretend that `fungible` is an external crate, exposing the Fungible Token
// application's interface.
use super::fungible;
