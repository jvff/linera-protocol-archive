use crate::AccountOwner;
use linera_sdk::base::Amount;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct InitialStateBuilder {
    account_balances: BTreeMap<AccountOwner, Amount>,
}

impl InitialStateBuilder {
    pub fn with_account(mut self, account: AccountOwner, balance: impl Into<Amount>) -> Self {
        self.account_balances.insert(account, balance.into());
        self
    }

    pub fn build(&self) -> Vec<u8> {
        bcs::to_bytes(&self.account_balances).expect("Failed to serialize initial state")
    }
}
