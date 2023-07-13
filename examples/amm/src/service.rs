#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{AmmError, Operation};
use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use async_trait::async_trait;
use linera_sdk::{base::WithServiceAbi, QueryContext, Service, ViewStateStorage};
use std::sync::Arc;

linera_sdk::service!(Amm);

impl WithServiceAbi for Amm {
    type Abi = amm::AmmAbi;
}

#[async_trait]
impl Service for Amm {
    type Error = AmmError;
    type Storage = ViewStateStorage<Self>;

    async fn query_application(
        self: Arc<Self>,
        _context: &QueryContext,
        request: Request,
    ) -> Result<Response, AmmError> {
        let schema = Schema::build(self.clone(), MutationRoot, EmptySubscription).finish();
        let response = schema.execute(request).await;
        Ok(response)
    }
}
struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn add_liquidity(&self, token0_amount: u64, token1_amount: u64) -> Vec<u8> {
        bcs::to_bytes(&Operation::AddLiquidity {
            token0_amount,
            token1_amount,
        })
        .unwrap()
    }

    async fn remove_liquidity(&self, shares_amount: u64) -> Vec<u8> {
        bcs::to_bytes(&Operation::RemoveLiquidity { shares_amount }).unwrap()
    }

    async fn swap(
        &self,
        input_token_idx: u32,
        output_token_idx: u32,
        input_amount: u64,
    ) -> Vec<u8> {
        bcs::to_bytes(&Operation::Swap {
            input_token_idx,
            output_token_idx,
            input_amount,
        })
        .unwrap()
    }
}
