// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::Amm;
use amm::{AmmAbi as Abi, AmmError, ApplicationCall, Message, Operation};
use async_trait::async_trait;
use fungible::{Account, FungibleTokenAbi};
use linera_sdk::{
    base::{AccountOwner, Amount, ApplicationId, Owner, SessionId, WithContractAbi},
    ensure, ApplicationCallOutcome, Contract, ContractRuntime, ExecutionOutcome, OutgoingMessage,
    Resources, ViewStateStorage,
};
use num_bigint::BigUint;
use num_traits::{cast::FromPrimitive, ToPrimitive};

linera_sdk::contract!(Amm);

impl WithContractAbi for Amm {
    type Abi = Abi;
}

#[async_trait]
impl Contract for Amm {
    type Error = AmmError;
    type Storage = ViewStateStorage<Self>;

    async fn initialize(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        _argument: (),
    ) -> Result<ExecutionOutcome<Self::Message>, AmmError> {
        // Validate that the application parameters were configured correctly.
        let _ = runtime.application_parameters();

        Ok(ExecutionOutcome::default())
    }

    async fn execute_operation(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        operation: Self::Operation,
    ) -> Result<ExecutionOutcome<Self::Message>, AmmError> {
        let mut outcome = ExecutionOutcome::default();
        if runtime.chain_id() == runtime.application_id().creation.chain_id {
            self.execute_order_local(runtime, operation)?;
        } else {
            self.execute_order_remote(runtime, &mut outcome, operation)?;
        }

        Ok(outcome)
    }

    async fn execute_message(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        message: Self::Message,
    ) -> Result<ExecutionOutcome<Self::Message>, AmmError> {
        ensure!(
            runtime.chain_id() == runtime.application_id().creation.chain_id,
            AmmError::AmmChainOnly
        );

        match message {
            Message::Swap {
                owner,
                input_token_idx,
                input_amount,
            } => {
                Self::check_account_authentication(None, runtime.authenticated_signer(), owner)?;
                self.execute_swap(runtime, owner, input_token_idx, input_amount)?;
            }
        }

        Ok(ExecutionOutcome::default())
    }

    async fn handle_application_call(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        application_call: ApplicationCall,
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<ApplicationCallOutcome<Self::Message, Self::Response>, AmmError> {
        let mut outcome = ApplicationCallOutcome::default();
        match application_call {
            ApplicationCall::Swap {
                owner,
                input_token_idx,
                input_amount,
            } => {
                Self::check_account_authentication(
                    runtime.authenticated_caller_id(),
                    runtime.authenticated_signer(),
                    owner,
                )?;
                if runtime.chain_id() == runtime.application_id().creation.chain_id {
                    self.execute_swap(runtime, owner, input_token_idx, input_amount)?;
                } else {
                    self.execute_application_call_remote(
                        runtime,
                        &mut outcome.execution_outcome,
                        application_call,
                    )?;
                }
            }
        }

        Ok(outcome)
    }
}

impl Amm {
    /// authenticate the originator of the message
    fn check_account_authentication(
        authenticated_application_id: Option<ApplicationId>,
        authenticated_signer: Option<Owner>,
        owner: AccountOwner,
    ) -> Result<(), AmmError> {
        match owner {
            AccountOwner::User(address) if authenticated_signer == Some(address) => Ok(()),
            AccountOwner::Application(id) if authenticated_application_id == Some(id) => Ok(()),
            _ => Err(AmmError::IncorrectAuthentication),
        }
    }

    fn execute_order_local(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        operation: Operation,
    ) -> Result<(), AmmError> {
        match operation {
            Operation::Swap {
                owner: _,
                input_token_idx: _,
                input_amount: _,
            } => Err(AmmError::SwappingLocally),
            Operation::AddLiquidity {
                owner,
                max_token0_amount,
                max_token1_amount,
            } => {
                if max_token0_amount == Amount::ZERO || max_token1_amount == Amount::ZERO {
                    return Err(AmmError::NoZeroAmounts);
                }

                let balance0 = self.get_pool_balance(runtime, 0)?;
                let balance1 = self.get_pool_balance(runtime, 1)?;

                let token0_amount;
                let token1_amount;
                if balance0 > Amount::ZERO && balance1 > Amount::ZERO {
                    let balance0_bigint = BigUint::from_u128(u128::from(balance0))
                        .expect("Couldn't generate balance0 in bigint");
                    let balance1_bigint = BigUint::from_u128(u128::from(balance1))
                        .expect("Couldn't generate balance1 in bigint");
                    let max_token0_amount_bigint =
                        BigUint::from_u128(u128::from(max_token0_amount))
                            .expect("Couldn't generate max_token0_amount in bigint");
                    let max_token1_amount_bigint =
                        BigUint::from_u128(u128::from(max_token1_amount))
                            .expect("Couldn't generate max_token1_amount in bigint");

                    // This is the formula to maintain the ratio:
                    //      balance0 / balance1 = (balance0 + max_token0_amount) / (balance1 + token1_amount)
                    //      balance0 * (balance1 + token1_amount) = balance1 * (balance0 + max_token0_amount)
                    //      balance0 * balance1 + balance0 * token1_amount = balance1 * balance0 + balance1 * max_token0_amount
                    //      balance0 * token1_amount = balance1 * max_token0_amount
                    //      token1_amount = (balance1 * max_token0_amount) / balance0
                    //
                    // For token0_amount, it would be this:
                    //      token0_amount = (balance0 * max_token1_amount) / balance1

                    if &max_token0_amount_bigint * &balance1_bigint
                        > &max_token1_amount_bigint * &balance0_bigint
                    {
                        let token0_amount_bigint =
                            (&balance0_bigint * &max_token1_amount_bigint) / &balance1_bigint;
                        token0_amount = Amount::from_attos(
                            token0_amount_bigint
                                .to_u128()
                                .expect("Couldn't convert token0_amount_bigint to u128"),
                        );
                        token1_amount = Amount::from_attos(
                            max_token1_amount_bigint
                                .to_u128()
                                .expect("Couldn't convert max_token1_amount_bigint to u128"),
                        );
                    } else {
                        let token1_amount_bigint =
                            (&balance1_bigint * &max_token0_amount_bigint) / &balance0_bigint;
                        token0_amount = Amount::from_attos(
                            max_token0_amount_bigint
                                .to_u128()
                                .expect("Couldn't convert max_token0_amount_bigint to u128"),
                        );
                        token1_amount = Amount::from_attos(
                            token1_amount_bigint
                                .to_u128()
                                .expect("Couldn't convert token1_amount_bigint to u128"),
                        );
                    }
                } else {
                    // This means we're on the first liquidity addition
                    token0_amount = max_token0_amount;
                    token1_amount = max_token1_amount;
                }

                self.receive_from_account(runtime, &owner, 0, token0_amount);
                self.receive_from_account(runtime, &owner, 1, token1_amount);

                Ok(())
            }
            // When removing liquidity, you'll specify one of the tokens you want to
            // remove and the amount, and we'll calculate the amount for the other token that
            // we'll remove based on the current ratio, and remove them.
            Operation::RemoveLiquidity {
                owner,
                token_to_remove_idx,
                mut token_to_remove_amount,
            } => {
                if token_to_remove_idx > 1 {
                    return Err(AmmError::InvalidTokenIdx);
                }

                let other_token_to_remove_idx = 1 - token_to_remove_idx;
                let balance0 = self.get_pool_balance(runtime, 0)?;
                let balance1 = self.get_pool_balance(runtime, 1)?;

                if token_to_remove_idx == 0 && token_to_remove_amount > balance0 {
                    token_to_remove_amount = balance0;
                } else if token_to_remove_idx == 1 && token_to_remove_amount > balance1 {
                    token_to_remove_amount = balance1;
                }

                let token_to_remove_amount_bigint =
                    BigUint::from_u128(u128::from(token_to_remove_amount))
                        .expect("Couldn't generate token_to_remove_amount in bigint");

                let balance0_bigint = BigUint::from_u128(u128::from(balance0))
                    .expect("Couldn't generate balance0 in bigint");
                let balance1_bigint = BigUint::from_u128(u128::from(balance1))
                    .expect("Couldn't generate balance1 in bigint");

                let other_amount = if token_to_remove_idx == 0 {
                    Amount::from_attos(
                        ((token_to_remove_amount_bigint * balance1_bigint) / balance0_bigint)
                            .to_u128()
                            .expect("Couldn't convert other_amount to u128"),
                    )
                } else {
                    Amount::from_attos(
                        ((token_to_remove_amount_bigint * balance0_bigint) / balance1_bigint)
                            .to_u128()
                            .expect("Couldn't convert other_amount to u128"),
                    )
                };

                self.send_to(runtime, &owner, token_to_remove_idx, token_to_remove_amount);
                self.send_to(runtime, &owner, other_token_to_remove_idx, other_amount);
                Ok(())
            }
        }
    }

    fn execute_swap(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: AccountOwner,
        input_token_idx: u32,
        input_amount: Amount,
    ) -> Result<(), AmmError> {
        if input_amount == Amount::ZERO {
            return Err(AmmError::NoZeroAmounts);
        }

        if input_token_idx > 1 {
            return Err(AmmError::InvalidTokenIdx);
        }

        let output_token_idx = 1 - input_token_idx;
        let input_pool_balance = self.get_pool_balance(runtime, input_token_idx)?;
        let output_pool_balance = self.get_pool_balance(runtime, output_token_idx)?;

        let output_amount =
            self.calculate_output_amount(input_amount, input_pool_balance, output_pool_balance)?;

        self.receive_from_account(runtime, &owner, input_token_idx, input_amount);
        self.send_to(runtime, &owner, output_token_idx, output_amount);

        Ok(())
    }

    fn execute_order_remote(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        outcome: &mut ExecutionOutcome<Message>,
        operation: Operation,
    ) -> Result<(), AmmError> {
        match operation {
            Operation::Swap {
                owner,
                input_token_idx,
                input_amount,
            } => {
                let chain_id = runtime.application_id().creation.chain_id;
                let message = Message::Swap {
                    owner,
                    input_token_idx,
                    input_amount,
                };
                outcome.messages.push(OutgoingMessage {
                    destination: chain_id.into(),
                    authenticated: true,
                    is_tracked: false,
                    resources: Resources::default(),
                    message,
                });
            }
            Operation::AddLiquidity {
                owner: _,
                max_token0_amount: _,
                max_token1_amount: _,
            } => {
                return Err(AmmError::AddingLiquidityFromRemoteChain);
            }
            Operation::RemoveLiquidity {
                owner: _,
                token_to_remove_idx: _,
                token_to_remove_amount: _,
            } => {
                return Err(AmmError::RemovingLiquidityFromRemoteChain);
            }
        }

        Ok(())
    }

    fn execute_application_call_remote(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        outcome: &mut ExecutionOutcome<Message>,
        application_call: ApplicationCall,
    ) -> Result<(), AmmError> {
        match application_call {
            ApplicationCall::Swap {
                owner,
                input_token_idx,
                input_amount,
            } => {
                let chain_id = runtime.application_id().creation.chain_id;
                let message = Message::Swap {
                    owner,
                    input_token_idx,
                    input_amount,
                };
                outcome.messages.push(OutgoingMessage {
                    destination: chain_id.into(),
                    authenticated: true,
                    is_tracked: false,
                    resources: Resources::default(),
                    message,
                });
            }
        }

        Ok(())
    }

    fn calculate_output_amount(
        &mut self,
        input_amount: Amount,
        input_pool_balance: Amount,
        output_pool_balance: Amount,
    ) -> Result<Amount, AmmError> {
        if input_pool_balance == Amount::ZERO || output_pool_balance == Amount::ZERO {
            return Err(AmmError::InvalidPoolBalanceError);
        }

        let input_amount_bigint = BigUint::from_u128(u128::from(input_amount))
            .expect("Couldn't generate input_amount in bigint");
        let output_pool_balance_bigint = BigUint::from_u128(u128::from(output_pool_balance))
            .expect("Couldn't generate output_pool_balance in bigint");
        let input_pool_balance_bigint = BigUint::from_u128(u128::from(input_pool_balance))
            .expect("Couldn't generate input_pool_balance in bigint");

        // Logic for this is the following:
        // This is a Constant Product Automated Market Maker, or CPAMM, so we want
        // the product to remain constant.
        // That means that this is the equation we need to solve to find output_amount:
        //      (input_pool_balance + input_amount) * (output_pool_balance - output_amount) = input_pool_balance * output_pool_balance
        //      output_pool_balance - output_amount = (input_pool_balance * output_pool_balance) / (input_pool_balance + input_amount)
        //      output_amount = output_pool_balance - (input_pool_balance * output_pool_balance) / (input_pool_balance + input_amount)
        //      output_amount = (output_pool_balance * (input_pool_balance + input_amount) - (input_pool_balance * output_pool_balance)) / (input_pool_balance + input_amount)
        //      output_amount = (input_pool_balance * output_pool_balance + input_amount * output_pool_balance - input_pool_balance * output_pool_balance) / (input_pool_balance + input_amount)
        //      output_amount = (input_amount * output_pool_balance) / (input_pool_balance + input_amount)

        // Numerator will be a number with 36 decimal points here
        let numerator_bigint = &input_amount_bigint * output_pool_balance_bigint;
        // Denominator will have 18 decimal points
        let denominator_bigint = input_pool_balance_bigint + input_amount_bigint;

        // Dividing 36 decimal points with 18 decimal points = 18 decimal points
        let output_amount_bigint = numerator_bigint / denominator_bigint;
        let output_amount = Amount::from_attos(
            output_amount_bigint
                .to_u128()
                .expect("Couldn't convert output_amount_bigint to u128"),
        );
        Ok(output_amount)
    }

    fn get_pool_balance(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        token_idx: u32,
    ) -> Result<Amount, AmmError> {
        let pool_owner = AccountOwner::Application(runtime.application_id().forget_abi());
        self.balance(runtime, &pool_owner, token_idx)
    }

    fn fungible_id(
        runtime: &mut ContractRuntime<Abi>,
        token_idx: u32,
    ) -> ApplicationId<FungibleTokenAbi> {
        runtime.application_parameters().tokens[token_idx as usize]
    }

    fn transfer(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: &AccountOwner,
        amount: Amount,
        destination: Account,
        token_idx: u32,
    ) {
        let transfer = fungible::ApplicationCall::Transfer {
            owner: *owner,
            amount,
            destination,
        };
        let token = Self::fungible_id(runtime, token_idx);
        runtime.call_application(true, token, &transfer);
    }

    fn balance(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: &AccountOwner,
        token_idx: u32,
    ) -> Result<Amount, AmmError> {
        let balance = fungible::ApplicationCall::Balance { owner: *owner };
        let token = Self::fungible_id(runtime, token_idx);
        match runtime.call_application(true, token, &balance) {
            fungible::FungibleResponse::Balance(balance) => Ok(balance),
            response => Err(AmmError::UnexpectedFungibleResponse(response)),
        }
    }

    fn receive_from_account(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: &AccountOwner,
        token_idx: u32,
        amount: Amount,
    ) {
        let destination = Account {
            chain_id: runtime.chain_id(),
            owner: AccountOwner::Application(runtime.application_id().forget_abi()),
        };
        self.transfer(runtime, owner, amount, destination, token_idx);
    }

    fn send_to(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: &AccountOwner,
        token_idx: u32,
        amount: Amount,
    ) {
        let destination = Account {
            chain_id: runtime.chain_id(),
            owner: *owner,
        };
        let owner_app = AccountOwner::Application(runtime.application_id().forget_abi());
        self.transfer(runtime, &owner_app, amount, destination, token_idx);
    }
}
