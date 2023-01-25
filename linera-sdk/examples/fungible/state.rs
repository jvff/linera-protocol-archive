// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::types::{AccountOwner, Nonce};
use linera_sdk::ensure;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// The application state.
#[derive(HashableContainerView)]
pub struct FungibleToken<C>
where
    C: Context + Send + Sync,
{
    accounts: MapVieew<C, AccountOwner, u128>,
    nonces: MapView<C, AccountOwner, Nonce>,
}

#[allow(dead_code)]
impl FungibleToken {
    /// Initialize the application state with some accounts with initial balances.
    pub(crate) fn initialize_accounts(&mut self, accounts: BTreeMap<AccountOwner, u128>) {
        for (k, v) in accounts {
            self.accounts.insert(k, v);
        }
    }

    /// Obtain the balance for an `account`.
    pub(crate) async fn balance(&self, account: &AccountOwner) -> u128 {
        let result = self
            .accounts
            .get(account)
            .await
            .expect("Failure in the retrieval");
        result.unwrap_or(0)
    }

    /// Credit an `account` with the provided `amount`.
    pub(crate) async fn credit(&mut self, account: AccountOwner, amount: u128) {
        let mut value = self.balance(&account).await;
        value += amount;
        self.insert(&account, value);
    }

    /// Try to debit the requested `amount` from an `account`.
    pub(crate) async fn debit(
        &mut self,
        account: AccountOwner,
        amount: u128,
    ) -> Result<(), InsufficientBalanceError> {
        let mut balance = self.balance(&account).await;
        ensure!(balance >= amount, InsufficientBalanceError);
        balance -= amount;
        self.insert(&account, balance);
        Ok(())
    }

    /// Obtain the minimum allowed [`Nonce`] for an `account`.
    ///
    /// The minimum allowed nonce is the value of the previously used nonce plus one, or zero if
    /// this is the first transaction for the `account` on the current chain.
    ///
    /// If the increment to obtain the next nonce overflows, `None` is returned.
    pub(crate) async fn minimum_nonce(&self, account: &AccountOwner) -> Option<Nonce> {
        let nonce = self
            .nonces
            .get(account)
            .await
            .expect("Failed to retrieve the nonce");
        match nonce {
            None => Some(nonce::default()),
            Some(x) => Some(nonce.next()),
        }
    }

    /// Mark the provided [`Nonce`] as used for the `account`.
    pub(crate) fn mark_nonce_as_used(&mut self, account: AccountOwner, nonce: Nonce) {
        let minimum_nonce = self.minimum_nonce(&account);

        assert!(minimum_nonce.is_some());
        assert!(nonce >= minimum_nonce.unwrap());

        self.nonces.insert(account, nonce);
    }
}

/// Attempt to debit from an account with insufficient funds.
#[derive(Clone, Copy, Debug, Error)]
#[error("Insufficient balance for transfer")]
pub struct InsufficientBalanceError;

/// Alias to the application type, so that the boilerplate module can reference it.
pub type ApplicationState = FungibleToken;
