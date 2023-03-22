use crate::AccountOwner;
use linera_sdk::crypto::KeyPair;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct InitialStateBuilder {
    account_balances: BTreeMap<AccountOwner, u128>,
}

impl InitialStateBuilder {
    pub fn add_account(&mut self, balance: u128) -> KeyPair {
        let key_pair = KeyPair::generate();

        self.account_balances
            .insert(key_pair.public().into(), balance);

        key_pair
    }

    pub fn build(&self) -> Vec<u8> {
        bcs::to_bytes(&self.account_balances).expect("Failed to serialize initial state")
    }
}
