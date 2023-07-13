#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{get_pool_balance, update_pool_balance, AMMError};
use async_trait::async_trait;
use linera_sdk::{
    base::{SessionId, WithContractAbi},
    ApplicationCallResult, CalleeContext, Contract, ExecutionResult, MessageContext,
    OperationContext, SessionCallResult, ViewStateStorage,
};

linera_sdk::contract!(Amm);

impl WithContractAbi for Amm {
    type Abi = amm::AMMAbi;
}

#[async_trait]
impl Contract for Amm {
    type Error = AMMError;
    type Storage = ViewStateStorage<Self>;

    async fn initialize(
        &mut self,
        _context: &OperationContext,
        _argument: (),
    ) -> Result<ExecutionResult<Self::Message>, AMMError> {
        Ok(ExecutionResult::default())
    }

    async fn execute_operation(
        &mut self,
        _context: &OperationContext,
        operation: Self::Operation,
    ) -> Result<ExecutionResult<Self::Message>, AMMError> {
        match operation {
            Self::Operation::Swap {
                input_token,
                output_token,
                amount,
            } => {
                let input_pool_balance = get_pool_balance(&self.token_pool, &input_token).await?;
                let output_pool_balance = get_pool_balance(&self.token_pool, &output_token).await?;

                let output_amount =
                    amm::calculate_output_amount(amount, input_pool_balance, output_pool_balance)?;

                update_pool_balance(
                    &mut self.token_pool,
                    &input_token,
                    input_pool_balance - amount,
                )?;
                update_pool_balance(
                    &mut self.token_pool,
                    &output_token,
                    output_pool_balance + output_amount,
                )?;

                Ok(ExecutionResult::default())
            }
            Self::Operation::AddLiquidity { token, amount } => {
                let existing_balance = get_pool_balance(&self.token_pool, &token).await?;
                let new_balance = existing_balance + amount;
                update_pool_balance(&mut self.token_pool, &token, new_balance)?;

                Ok(ExecutionResult::default())
            }
            Self::Operation::RemoveLiquidity { token, amount } => {
                let existing_balance = get_pool_balance(&self.token_pool, &token).await?;
                if amount > existing_balance {
                    return Err(AMMError::InsufficientLiquidityError);
                }

                let new_balance = existing_balance - amount;
                update_pool_balance(&mut self.token_pool, &token, new_balance)?;

                Ok(ExecutionResult::default())
            }
        }
    }

    async fn execute_message(
        &mut self,
        _context: &MessageContext,
        _message: (),
    ) -> Result<ExecutionResult<Self::Message>, AMMError> {
        Err(AMMError::MessagesNotSupported)
    }

    async fn handle_application_call(
        &mut self,
        _context: &CalleeContext,
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult<Self::Message, Self::Response, Self::SessionState>, AMMError>
    {
        Err(AMMError::ApplicationCallsNotSupported)
    }

    async fn handle_session_call(
        &mut self,
        _context: &CalleeContext,
        _session: (),
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult<Self::Message, Self::Response, Self::SessionState>, AMMError>
    {
        Err(AMMError::SessionsNotSupported)
    }
}
