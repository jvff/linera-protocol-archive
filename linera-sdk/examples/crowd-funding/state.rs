// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use fungible::AccountOwner;
use linera_sdk::{ApplicationId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CrowdFunding {
    /// The status of the campaign.
    pub status: Status,
    /// The map of pledges that will be collected if the campaign succeeds.
    pub pledges: BTreeMap<AccountOwner, u128>,
    /// The parameters that determine the details the campaign.
    pub parameters: Parameters,
}

impl Default for Parameters {
    fn default() -> Self {
        Parameters {
            owner: AccountOwner::Application(ApplicationId::default()),
            token: ApplicationId::default(),
            deadline: Timestamp::default(),
            target: 0_u128,
        }
    }
}

#[allow(dead_code)]
impl Status {
    /// Returns `true` if the campaign status is [`Status::Complete`].
    pub fn is_complete(&self) -> bool {
        matches!(self, Status::Complete)
    }

    /// Returns `true` if the campaign status is [`Status::Cancelled`].
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Status::Cancelled)
    }
}

/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = CrowdFunding;

// Work-around to pretend that `fungible` is an external crate, exposing the Fungible Token
// application's interface.
use super::fungible;
