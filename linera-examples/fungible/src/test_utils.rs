//! Helper types for writing tests.

use crate::AccountOwner;
use linera_sdk::base::Amount;
use std::collections::BTreeMap;

/// A builder type for constructing the initial state of the application.
#[derive(Debug, Default)]
pub struct InitialStateBuilder {
    account_balances: BTreeMap<AccountOwner, Amount>,
}

impl InitialStateBuilder {
    /// Adds an account to the initial state of the application.
    pub fn with_account(mut self, account: AccountOwner, balance: impl Into<Amount>) -> Self {
        self.account_balances.insert(account, balance.into());
        self
    }

    /// Returns the serialized initial state of the application, ready to used as the
    /// initialization argument.
    pub fn build(&self) -> Vec<u8> {
        bcs::to_bytes(&self.account_balances).expect("Failed to serialize initial state")
    }
}
