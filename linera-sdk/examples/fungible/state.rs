// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use ed25519_dalek::PublicKey;
use linera_sdk::{ensure, ApplicationId};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};
use thiserror::Error;

/// The application state.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FungibleToken {
    accounts: BTreeMap<AccountOwner, u128>,
}

/// An account owner.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AccountOwner {
    /// An account protected by a private key.
    Key(PublicKey),
    /// An account for an application.
    Application(ApplicationId),
}

impl PartialOrd for AccountOwner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AccountOwner {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (AccountOwner::Key(_), AccountOwner::Application(_)) => Ordering::Less,
            (AccountOwner::Application(_), AccountOwner::Key(_)) => Ordering::Greater,
            (AccountOwner::Key(first), AccountOwner::Key(second)) => {
                first.as_bytes().cmp(second.as_bytes())
            }
            (AccountOwner::Application(first), AccountOwner::Application(second)) => {
                first.cmp(second)
            }
        }
    }
}

#[allow(dead_code)]
impl FungibleToken {
    /// Obtain the balance for an `account`.
    pub(crate) fn balance(&self, account: &AccountOwner) -> u128 {
        self.accounts.get(&account).copied().unwrap_or(0)
    }

    /// Credit an `account` with the provided `amount`.
    pub(crate) fn credit(&mut self, account: AccountOwner, amount: u128) {
        *self.accounts.entry(account).or_default() += amount;
    }

    /// Try to debit the requested `amount` from an `account`.
    pub(crate) fn debit(
        &mut self,
        account: AccountOwner,
        amount: u128,
    ) -> Result<(), InsufficientBalanceError> {
        let balance = self
            .accounts
            .get_mut(&account)
            .ok_or(InsufficientBalanceError)?;

        ensure!(*balance >= amount, InsufficientBalanceError);

        *balance -= amount;

        Ok(())
    }
}

/// Attempt to debit from an account with insufficient funds.
#[derive(Clone, Copy, Debug, Error)]
#[error("Insufficient balance for transfer")]
pub struct InsufficientBalanceError;

/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = FungibleToken;
