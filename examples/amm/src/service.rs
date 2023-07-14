#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{get_pool_balance, AMMError, Operation};
use async_graphql::{Context, EmptySubscription, Object, Request, Response, Schema};
use async_trait::async_trait;
use linera_sdk::{base::WithServiceAbi, QueryContext, Service, ViewStateStorage};
use std::sync::Arc;

linera_sdk::service!(Amm);

impl WithServiceAbi for Amm {
    type Abi = amm::AMMAbi;
}

#[async_trait]
impl Service for Amm {
    type Error = AMMError;
    type Storage = ViewStateStorage<Self>;

    async fn query_application(
        self: Arc<Self>,
        _context: &QueryContext,
        request: Request,
    ) -> Result<Response, AMMError> {
        let schema = Schema::build(self.clone(), MutationRoot, EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}
struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn add_liquidity(&self, token: String, amount: u64) -> Vec<u8> {
        bcs::to_bytes(&Operation::AddLiquidity { token, amount }).unwrap()
    }

    async fn remove_liquidity(&self, token: String, amount: u64) -> Vec<u8> {
        bcs::to_bytes(&Operation::RemoveLiquidity { token, amount }).unwrap()
    }

    async fn swap(&self, input_token: String, output_token: String, amount: u64) -> Vec<u8> {
        bcs::to_bytes(&Operation::Swap {
            input_token,
            output_token,
            amount,
        })
        .unwrap()
    }
}
