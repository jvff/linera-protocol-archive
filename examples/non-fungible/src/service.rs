// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::NonFungibleToken;
use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use async_trait::async_trait;
use fungible::Account;
use linera_sdk::{
    base::{AccountOwner, WithServiceAbi},
    Service, ServiceRuntime, ViewStateStorage,
};
use non_fungible::{NftOutput, Operation, TokenId};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use thiserror::Error;

linera_sdk::service!(NonFungibleToken);

impl WithServiceAbi for NonFungibleToken {
    type Abi = non_fungible::NonFungibleTokenAbi;
}

#[async_trait]
impl Service for NonFungibleToken {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;

    async fn handle_query(
        self: Arc<Self>,
        _runtime: &ServiceRuntime,
        request: Request,
    ) -> Result<Response, Self::Error> {
        let schema = Schema::build(
            QueryRoot {
                non_fungible_token: self.clone(),
            },
            MutationRoot,
            EmptySubscription,
        )
        .finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}

struct QueryRoot {
    non_fungible_token: Arc<NonFungibleToken>,
}

#[Object]
impl QueryRoot {
    async fn nft(&self, token_id: String) -> Option<NftOutput> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};
        let token_id_vec = STANDARD_NO_PAD.decode(&token_id).unwrap();
        let nft = self
            .non_fungible_token
            .nfts
            .get(&TokenId { id: token_id_vec })
            .await
            .unwrap();

        if let Some(nft) = nft {
            Some(NftOutput {
                token_id,
                owner: nft.owner,
                name: nft.name,
                minter: nft.minter,
                payload: nft.payload,
            })
        } else {
            None
        }
    }

    async fn nfts(&self) -> BTreeMap<String, NftOutput> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};

        let mut nfts = BTreeMap::new();
        self.non_fungible_token
            .nfts
            .for_each_index_value(|token_id, nft| {
                let token_id_str = STANDARD_NO_PAD.encode(token_id.id);
                nfts.insert(
                    token_id_str.clone(),
                    NftOutput {
                        token_id: token_id_str,
                        owner: nft.owner,
                        name: nft.name,
                        minter: nft.minter,
                        payload: nft.payload,
                    },
                );

                Ok(())
            })
            .await
            .unwrap();

        nfts
    }

    async fn owned_token_ids_by_owner(&self, owner: AccountOwner) -> BTreeSet<String> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};

        let token_ids = self
            .non_fungible_token
            .owned_token_ids
            .get(&owner)
            .await
            .unwrap();

        token_ids
            .into_iter()
            .flatten()
            .map(|token_id| STANDARD_NO_PAD.encode(token_id.id))
            .collect()
    }

    async fn owned_token_ids(&self) -> BTreeMap<AccountOwner, BTreeSet<String>> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};

        let mut owners = BTreeMap::new();
        self.non_fungible_token
            .owned_token_ids
            .for_each_index_value(|owner, token_ids| {
                let new_token_ids = token_ids
                    .into_iter()
                    .map(|token_id| STANDARD_NO_PAD.encode(token_id.id))
                    .collect::<BTreeSet<_>>();

                owners.insert(owner, new_token_ids);
                Ok(())
            })
            .await
            .unwrap();

        owners
    }

    async fn owned_nfts(&self, owner: AccountOwner) -> BTreeMap<String, NftOutput> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};

        let mut result = BTreeMap::new();
        let owned_token_ids = self
            .non_fungible_token
            .owned_token_ids
            .get(&owner)
            .await
            .unwrap();

        for token_id in owned_token_ids.into_iter().flatten() {
            let nft = self
                .non_fungible_token
                .nfts
                .get(&token_id)
                .await
                .unwrap()
                .unwrap();
            let token_id_str = STANDARD_NO_PAD.encode(token_id.id);

            result.insert(
                token_id_str.clone(),
                NftOutput {
                    token_id: token_id_str,
                    owner: nft.owner,
                    name: nft.name,
                    minter: nft.minter,
                    payload: nft.payload,
                },
            );
        }

        result
    }
}

struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn mint(&self, name: String, payload: Vec<u8>) -> Vec<u8> {
        bcs::to_bytes(&Operation::Mint { name, payload }).unwrap()
    }

    async fn transfer(
        &self,
        source_owner: AccountOwner,
        token_id: String,
        target_account: Account,
    ) -> Vec<u8> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};
        bcs::to_bytes(&Operation::Transfer {
            source_owner,
            token_id: TokenId {
                id: STANDARD_NO_PAD.decode(token_id).unwrap(),
            },
            target_account,
        })
        .unwrap()
    }

    async fn claim(
        &self,
        source_account: Account,
        token_id: String,
        target_account: Account,
    ) -> Vec<u8> {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine as _};
        bcs::to_bytes(&Operation::Claim {
            source_account,
            token_id: TokenId {
                id: STANDARD_NO_PAD.decode(token_id).unwrap(),
            },
            target_account,
        })
        .unwrap()
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid query argument; could not deserialize GraphQL request.
    #[error(
        "Invalid query argument; Fungible application only supports JSON encoded GraphQL queries"
    )]
    InvalidQuery(#[from] serde_json::Error),
}
