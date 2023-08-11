#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{AmmError, ApplicationCall, Message, Operation, OperationType};
use async_trait::async_trait;
use fungible::{Account, AccountOwner, Destination, FungibleTokenAbi};
use linera_sdk::{
    base::{Amount, ApplicationId, SessionId, WithContractAbi},
    contract::system_api,
    ensure, ApplicationCallResult, CalleeContext, Contract, ExecutionResult, MessageContext,
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
        self.total_shares.set(Amount::ZERO);
        Ok(ExecutionResult::default())
    }

    async fn execute_operation(
        &mut self,
        context: &OperationContext,
        operation: Self::Operation,
    ) -> Result<ExecutionResult<Self::Message>, AmmError> {
        let mut result = ExecutionResult::default();
        match operation {
            Operation::ExecuteOperation { operation } => {
                if context.chain_id == system_api::current_application_id().creation.chain_id {
                    self.execute_order_local(operation).await?;
                } else {
                    self.execute_order_remote(&mut result, operation).await?;
                }
            }
        }

        Ok(result)
    }

    async fn execute_message(
        &mut self,
        context: &MessageContext,
        message: Self::Message,
    ) -> Result<ExecutionResult<Self::Message>, AmmError> {
        ensure!(
            context.chain_id == system_api::current_application_id().creation.chain_id,
            AmmError::AmmChainOnly
        );
        match message {
            Message::ExecuteOperation { operation } => {
                self.execute_order_local(operation).await?;
            }
        }
        Ok(ExecutionResult::default())
    }

    async fn handle_application_call(
        &mut self,
        context: &CalleeContext,
        argument: Self::ApplicationCall,
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallResult<Self::Message, Self::Response, Self::SessionState>, AmmError>
    {
        let mut result = ApplicationCallResult::default();
        match argument {
            ApplicationCall::ExecuteOperation { operation } => {
                if context.chain_id == system_api::current_application_id().creation.chain_id {
                    self.execute_order_local(operation).await?;
                } else {
                    self.execute_order_remote(&mut result.execution_result, operation)
                        .await?;
                }
            }
        }

        Ok(result)
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
    async fn execute_order_local(&mut self, operation: OperationType) -> Result<(), AmmError> {
        match operation {
            OperationType::Swap {
                owner,
                input_token_idx,
                output_token_idx,
                input_amount,
            } => {
                if input_token_idx == output_token_idx {
                    return Err(AmmError::EqualTokensError);
                }

                let input_pool_balance = self.get_pool_balance(input_token_idx).await?;
                let output_pool_balance = self.get_pool_balance(output_token_idx).await?;

                let output_amount = amm::calculate_output_amount(
                    input_amount,
                    input_pool_balance,
                    output_pool_balance,
                )?;

                self.receive_from_account(&owner, input_token_idx, &input_amount)
                    .await?;
                self.send_to(&owner, output_token_idx, &output_amount).await
            }
            OperationType::AddLiquidity {
                owner,
                token0_amount,
                token1_amount,
            } => {
                tracing::info!("AddLiquidity");
                let balance0 = self.get_pool_balance(0).await?;
                tracing::info!("1");
                let balance1 = self.get_pool_balance(1).await?;
                tracing::info!("2");

                let total_shares = self.total_shares.get();
                tracing::info!("3");

                if *total_shares > Amount::ZERO
                    && balance0.saturating_div(balance1)
                        != token0_amount.saturating_div(token1_amount)
                {
                    return Err(AmmError::BalanceRatioAlteredError);
                }
                tracing::info!("4");

                if *total_shares > Amount::ZERO
                    && Amount::try_from(token0_amount.saturating_div(balance0))?
                        .try_mul((*total_shares).into())?
                        != Amount::try_from(token1_amount.saturating_div(balance1))?
                            .try_mul((*total_shares).into())?
                {
                    return Err(AmmError::TotalSharesRatioAlteredError);
                }
                tracing::info!("5");

                let new_shares = if *total_shares == Amount::ZERO {
                    Amount::try_from(
                        (<Amount as Into<u128>>::into(token0_amount.try_mul(token1_amount.into())?)
                            as f64)
                            .sqrt() as u128,
                    )?
                } else {
                    Amount::try_from(token0_amount.saturating_div(balance0))?
                        .try_mul((*total_shares).into())?
                };
                tracing::info!("6");

                self.receive_from_account(&owner, 0, &token0_amount).await?;
                tracing::info!("7");
                self.receive_from_account(&owner, 1, &token1_amount).await?;
                tracing::info!("8");

                Ok((*self.total_shares.get_mut()).try_add_assign(new_shares)?)
            }
            OperationType::RemoveLiquidity {
                owner,
                shares_amount,
            } => {
                let balance0 = self.get_pool_balance(0).await?;
                let balance1 = self.get_pool_balance(1).await?;

                let total_shares = self.total_shares.get();

                let shares_pct_to_remove = shares_amount.saturating_div(*total_shares);

                let token0_amount = balance0.try_mul(shares_pct_to_remove)?;
                let token1_amount = balance1.try_mul(shares_pct_to_remove)?;

                if token0_amount > balance0 || token1_amount > balance1 {
                    return Err(AmmError::InsufficientLiquidityError);
                }

                self.send_to(&owner, 0, &token0_amount).await?;
                self.send_to(&owner, 1, &token1_amount).await?;
                Ok((*self.total_shares.get_mut()).try_sub_assign(shares_amount)?)
            }
        }
    }

    async fn execute_order_remote(
        &mut self,
        result: &mut ExecutionResult<Message>,
        operation: OperationType,
    ) -> Result<(), AmmError> {
        let chain_id = system_api::current_application_id().creation.chain_id;
        let message = Message::ExecuteOperation {
            operation: operation.clone(),
        };
        result.messages.push((chain_id.into(), true, message));
        Ok(())
    }

    async fn get_pool_balance(&mut self, token_idx: u32) -> Result<Amount, AmmError> {
        let pool_owner = AccountOwner::Application(system_api::current_application_id());
        self.balance(pool_owner, token_idx).await
    }

    fn fungible_id(token_idx: u32) -> Result<ApplicationId<FungibleTokenAbi>, AmmError> {
        let parameter = Self::parameters()?;
        Ok(parameter.tokens[token_idx as usize])
    }

    async fn transfer(
        &mut self,
        owner: AccountOwner,
        amount: Amount,
        destination: Destination,
        token_idx: u32,
    ) -> Result<(), AmmError> {
        let transfer = fungible::ApplicationCall::Transfer {
            owner,
            amount,
            destination,
        };
        let token = Self::fungible_id(token_idx).expect("failed to get the token");
        self.call_application(true, token, &transfer, vec![])
            .await?;
        Ok(())
    }

    async fn balance(&mut self, owner: AccountOwner, token_idx: u32) -> Result<Amount, AmmError> {
        let balance = fungible::ApplicationCall::Balance { owner };
        let token = Self::fungible_id(token_idx).expect("failed to get the token");
        Ok(self
            .call_application(true, token, &balance, vec![])
            .await?
            .0)
    }

    async fn receive_from_account(
        &mut self,
        owner: &AccountOwner,
        token_idx: u32,
        amount: &Amount,
    ) -> Result<(), AmmError> {
        let account = Account {
            chain_id: system_api::current_chain_id(),
            owner: AccountOwner::Application(system_api::current_application_id()),
        };
        let destination = Destination::Account(account);
        self.transfer(*owner, *amount, destination, token_idx).await
    }

    async fn send_to(
        &mut self,
        owner: &AccountOwner,
        token_idx: u32,
        amount: &Amount,
    ) -> Result<(), AmmError> {
        let account = Account {
            chain_id: system_api::current_chain_id(),
            owner: *owner,
        };
        let destination = Destination::Account(account);
        let owner_app = AccountOwner::Application(system_api::current_application_id());
        self.transfer(owner_app, *amount, destination, token_idx)
            .await
    }
}
