#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::AmmError;
use async_trait::async_trait;
use linera_sdk::{
    base::{SessionId, WithContractAbi},
    ApplicationCallResult, CalleeContext, Contract, ExecutionResult, MessageContext,
    OperationContext, SessionCallResult, ViewStateStorage,
};

linera_sdk::contract!(Amm);

impl WithContractAbi for Amm {
    type Abi = amm::AmmAbi;
}

#[async_trait]
impl Contract for Amm {
    type Error = AmmError;
    type Storage = ViewStateStorage<Self>;

    async fn initialize(
        &mut self,
        _context: &OperationContext,
        _argument: (),
    ) -> Result<ExecutionResult<Self::Message>, AmmError> {
        self.balance0.set(0);
        self.balance1.set(0);
        self.total_shares.set(0);
        Ok(ExecutionResult::default())
    }

    async fn execute_operation(
        &mut self,
        _context: &OperationContext,
        operation: Self::Operation,
    ) -> Result<ExecutionResult<Self::Message>, AmmError> {
        match operation {
            Self::Operation::Swap {
                input_token_idx,
                output_token_idx,
                input_amount,
            } => {
                if input_token_idx == output_token_idx {
                    return Err(AmmError::EqualTokensError);
                }

                let input_pool_balance = *self.get_pool_balance(input_token_idx)?;
                let output_pool_balance = *self.get_pool_balance(output_token_idx)?;

                let output_amount = amm::calculate_output_amount(
                    input_amount,
                    input_pool_balance,
                    output_pool_balance,
                )?;

                self.decrease_pool_balance(input_token_idx, input_amount)?;
                self.increase_pool_balance(output_token_idx, output_amount)?;

                Ok(ExecutionResult::default())
            }
            Self::Operation::AddLiquidity {
                token0_amount,
                token1_amount,
            } => {
                let balance0 = self.balance0.get_mut();
                let balance1 = self.balance1.get_mut();

                let balance0_float = *balance0 as f64;
                let balance1_float = *balance1 as f64;

                let token0_amount_float = token0_amount as f64;
                let token1_amount_float = token1_amount as f64;

                let total_shares = self.total_shares.get_mut();
                let total_shares_float = *total_shares as f64;

                if *total_shares > 0
                    && balance0_float / balance1_float != token0_amount_float / token1_amount_float
                {
                    return Err(AmmError::BalanceRatioAlteredError);
                }

                if *total_shares > 0
                    && (token0_amount_float / balance0_float) * total_shares_float
                        != (token1_amount_float / balance1_float) * total_shares_float
                {
                    return Err(AmmError::TotalSharesRatioAlteredError);
                }

                let new_shares = if *total_shares == 0 {
                    (token0_amount_float * token1_amount_float).sqrt() as u64
                } else {
                    ((token0_amount_float / balance0_float) * total_shares_float) as u64
                };

                *balance0 += token0_amount;
                *balance1 += token1_amount;

                *total_shares += new_shares;
                Ok(ExecutionResult::default())
            }
            Self::Operation::RemoveLiquidity { shares_amount } => {
                let balance0 = self.balance0.get_mut();
                let balance1 = self.balance1.get_mut();

                let balance0_float = *balance0 as f64;
                let balance1_float = *balance1 as f64;

                let shares_amount_float = shares_amount as f64;

                let total_shares = self.total_shares.get_mut();
                let total_shares_float = *total_shares as f64;

                let shares_pct_to_remove = shares_amount_float / total_shares_float;

                let token0_amount = (balance0_float * shares_pct_to_remove) as u64;
                let token1_amount = (balance1_float * shares_pct_to_remove) as u64;

                if token0_amount > *balance0 || token1_amount > *balance1 {
                    return Err(AmmError::InsufficientLiquidityError);
                }

                *balance0 -= token0_amount;
                *balance1 -= token1_amount;
                *total_shares -= shares_amount;

                Ok(ExecutionResult::default())
            }
        }
    }

    async fn execute_message(
        &mut self,
        _context: &MessageContext,
        _message: (),
    ) -> Result<ExecutionResult<Self::Message>, AmmError> {
        Err(AmmError::MessagesNotSupported)
    }

    async fn handle_application_call(
        &mut self,
        _context: &CalleeContext,
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult<Self::Message, Self::Response, Self::SessionState>, AmmError>
    {
        Err(AmmError::ApplicationCallsNotSupported)
    }

    async fn handle_session_call(
        &mut self,
        _context: &CalleeContext,
        _session: (),
        _argument: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallResult<Self::Message, Self::Response, Self::SessionState>, AmmError>
    {
        Err(AmmError::SessionsNotSupported)
    }
}

impl Amm {
    fn get_pool_balance(&self, token_idx: u32) -> Result<&u64, AmmError> {
        match token_idx {
            0 => Ok(self.balance0.get()),
            1 => Ok(self.balance1.get()),
            _ => Err(AmmError::TokenNotFoundInPoolError),
        }
    }

    fn increase_pool_balance(&mut self, token_idx: u32, amount: u64) -> Result<(), AmmError> {
        match token_idx {
            0 => {
                let balance0 = self.balance0.get_mut();
                *balance0 += amount;
                Ok(())
            }
            1 => {
                let balance1 = self.balance1.get_mut();
                *balance1 += amount;
                Ok(())
            }
            _ => Err(AmmError::TokenNotFoundInPoolError),
        }
    }

    fn decrease_pool_balance(&mut self, token_idx: u32, amount: u64) -> Result<(), AmmError> {
        match token_idx {
            0 => {
                let balance0 = self.balance0.get_mut();
                *balance0 -= amount;
                Ok(())
            }
            1 => {
                let balance1 = self.balance1.get_mut();
                *balance1 -= amount;
                Ok(())
            }
            _ => Err(AmmError::TokenNotFoundInPoolError),
        }
    }
}
