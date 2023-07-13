use async_graphql::{scalar, Request, Response};
use fungible::FungibleTokenAbi;
use linera_sdk::base::{ApplicationId, ContractAbi, ServiceAbi};
use linera_views::views::ViewError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub struct AmmAbi;

impl ContractAbi for AmmAbi {
    type InitializationArgument = ();
    type Parameters = Parameters;
    type Operation = Operation;
    type ApplicationCall = ();
    type Message = ();
    type SessionCall = ();
    type Response = ();
    type SessionState = ();
}

impl ServiceAbi for AmmAbi {
    type Query = Request;
    type QueryResponse = Response;
    type Parameters = Parameters;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Parameters {
    /// The token0 and token1 used for the amm
    pub tokens: [ApplicationId<FungibleTokenAbi>; 2],
}

scalar!(Parameters);

#[derive(Debug, Serialize, Deserialize)]
pub enum Operation {
    Swap {
        input_token_idx: u32,
        output_token_idx: u32,
        input_amount: u64,
    },
    AddLiquidity {
        token0_amount: u64,
        token1_amount: u64,
    },
    RemoveLiquidity {
        shares_amount: u64,
    },
}

pub fn calculate_output_amount(
    input_amount: u64,
    input_pool_balance: u64,
    output_pool_balance: u64,
) -> Result<u64, AmmError> {
    if input_pool_balance == 0 || output_pool_balance == 0 {
        return Err(AmmError::InvalidPoolBalanceError);
    }

    let input_pool_balance_float = input_pool_balance as f64;
    let output_pool_balance_float = output_pool_balance as f64;

    let input_amount_float = input_amount as f64;
    let output_amount_float = (input_amount_float * output_pool_balance_float)
        / (input_pool_balance_float + input_amount_float);

    Ok(output_amount_float as u64)
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AmmError {
    #[error("Insufficient liquidity in the pool")]
    InsufficientLiquidityError,

    #[error("Swap tokens must be different")]
    EqualTokensError,

    #[error("Adding liquidity cannot alter balance ratio")]
    BalanceRatioAlteredError,

    #[error("Adding liquidity cannot alter total shares ratio")]
    TotalSharesRatioAlteredError,

    #[error("Invalid pool balance")]
    InvalidPoolBalanceError,

    #[error("Token not found in the pool")]
    TokenNotFoundInPoolError,

    #[error("AMM application doesn't support any cross-chain messages")]
    MessagesNotSupported,

    #[error("AMM application doesn't support any cross-application sessions")]
    SessionsNotSupported,

    #[error("AMM application doesn't support any application calls")]
    ApplicationCallsNotSupported,

    /// Invalid query.
    #[error("Invalid query")]
    InvalidQuery(#[from] serde_json::Error),

    #[error(transparent)]
    BcsError(#[from] bcs::Error),

    #[error(transparent)]
    ViewError(#[from] ViewError),
}
