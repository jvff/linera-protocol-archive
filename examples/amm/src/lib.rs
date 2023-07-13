use async_graphql::{Request, Response};
use linera_sdk::{
    base::{ContractAbi, ServiceAbi},
    views::MapView,
};
use linera_views::views::ViewError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub struct AMMAbi;

impl ContractAbi for AMMAbi {
    type InitializationArgument = ();
    type Parameters = ();
    type Operation = Operation;
    type ApplicationCall = ();
    type Message = ();
    type SessionCall = ();
    type Response = ();
    type SessionState = ();
}

impl ServiceAbi for AMMAbi {
    type Query = Request;
    type QueryResponse = Response;
    type Parameters = ();
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Operation {
    Swap {
        input_token: String,
        output_token: String,
        amount: u64,
    },
    AddLiquidity {
        token: String,
        amount: u64,
    },
    RemoveLiquidity {
        token: String,
        amount: u64,
    },
}

pub fn calculate_output_amount(
    input_amount: u64,
    input_pool_balance: u64,
    output_pool_balance: u64,
) -> Result<u64, AMMError> {
    if input_pool_balance == 0 || output_pool_balance == 0 {
        return Err(AMMError::InvalidPoolBalanceError);
    }

    let input_pool_balance_float = input_pool_balance as f64;
    let output_pool_balance_float = output_pool_balance as f64;

    let input_amount_float = input_amount as f64;
    let output_amount_float =
        (input_amount_float * output_pool_balance_float) / input_pool_balance_float;

    Ok(output_amount_float as u64)
}

pub async fn get_pool_balance(
    token_pool: &MapView<String, u64>,
    token: &String,
) -> Result<u64, AMMError> {
    if let Some(balance) = token_pool.get(token).await? {
        Ok(balance)
    } else {
        Err(AMMError::TokenNotFoundInPoolError)
    }
}

pub fn update_pool_balance(
    token_pool: &mut MapView<String, u64>,
    token: &String,
    new_balance: u64,
) -> Result<(), AMMError> {
    Ok(token_pool.insert(token, new_balance)?)
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum AMMError {
    #[error("Insufficient liquidity in the pool")]
    InsufficientLiquidityError,

    #[error("Invalid pool balance")]
    InvalidPoolBalanceError,

    #[error("Token not found in the pool")]
    TokenNotFoundInPoolError,

    #[error("Amm application doesn't support any cross-chain messages")]
    MessagesNotSupported,

    #[error("Amm application doesn't support any cross-application sessions")]
    SessionsNotSupported,

    #[error("Amm application doesn't support any application calls")]
    ApplicationCallsNotSupported,

    /// Invalid query.
    #[error("Invalid query")]
    InvalidQuery(#[from] serde_json::Error),

    #[error(transparent)]
    BcsError(#[from] bcs::Error),

    #[error(transparent)]
    ViewError(#[from] ViewError),
}
