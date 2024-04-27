// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

use std::sync::{Arc, Mutex};

use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use fungible::{Operation, Parameters};
use linera_sdk::{
    base::{AccountOwner, WithServiceAbi},
    graphql::GraphQLMutationRoot,
    EmptyState, Service, ServiceRuntime,
};
use native_fungible::{AccountEntry, TICKER_SYMBOL};

#[derive(Clone)]
pub struct NativeFungibleTokenService {
    runtime: Arc<Mutex<ServiceRuntime<Self>>>,
}

linera_sdk::service!(NativeFungibleTokenService);

impl WithServiceAbi for NativeFungibleTokenService {
    type Abi = fungible::FungibleTokenAbi;
}

impl Service for NativeFungibleTokenService {
    type State = EmptyState;
    type Parameters = Parameters;

    async fn new(_state: Self::State, runtime: ServiceRuntime<Self>) -> Self {
        NativeFungibleTokenService {
            runtime: Arc::new(Mutex::new(runtime)),
        }
    }

    async fn handle_query(&self, request: Request) -> Response {
        let schema =
            Schema::build(self.clone(), Operation::mutation_root(), EmptySubscription).finish();
        schema.execute(request).await
    }
}

struct Accounts {
    runtime: Arc<Mutex<ServiceRuntime<NativeFungibleTokenService>>>,
}

#[Object]
impl Accounts {
    // Define a field that lets you query by key
    async fn entry(&self, key: AccountOwner) -> AccountEntry {
        let owner = match key {
            AccountOwner::User(owner) => owner,
            AccountOwner::Application(_) => panic!("Applications not supported yet"),
        };
        let runtime = self
            .runtime
            .try_lock()
            .expect("Services only run in a single thread");

        AccountEntry {
            key,
            value: runtime.owner_balance(owner),
        }
    }

    async fn entries(&self) -> Vec<AccountEntry> {
        let runtime = self
            .runtime
            .try_lock()
            .expect("Services only run in a single thread");
        runtime
            .owner_balances()
            .into_iter()
            .map(|(owner, amount)| AccountEntry {
                key: AccountOwner::User(owner),
                value: amount,
            })
            .collect()
    }

    async fn keys(&self) -> Vec<AccountOwner> {
        let runtime = self
            .runtime
            .try_lock()
            .expect("Services only run in a single thread");
        runtime
            .balance_owners()
            .into_iter()
            .map(AccountOwner::User)
            .collect()
    }
}

// Implements additional fields not derived from struct members of FungibleToken.
#[Object]
impl NativeFungibleTokenService {
    async fn ticker_symbol(&self) -> Result<String, async_graphql::Error> {
        Ok(String::from(TICKER_SYMBOL))
    }

    async fn accounts(&self) -> Result<Accounts, async_graphql::Error> {
        Ok(Accounts {
            runtime: self.runtime.clone(),
        })
    }
}
