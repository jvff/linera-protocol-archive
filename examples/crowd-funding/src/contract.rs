// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use async_trait::async_trait;
use crowd_funding::{
    ApplicationCall, CrowdFundingAbi as Abi, InitializationArgument, Message, Operation,
};
use fungible::{Account, Destination, FungibleResponse, FungibleTokenAbi};
use linera_sdk::{
    base::{AccountOwner, Amount, ApplicationId, SessionId, WithContractAbi},
    ensure,
    views::View,
    ApplicationCallOutcome, Contract, ContractRuntime, ExecutionOutcome, OutgoingMessage,
    Resources, SessionCallOutcome, ViewStateStorage,
};
use state::{CrowdFunding, Status};
use thiserror::Error;

linera_sdk::contract!(CrowdFunding);

impl WithContractAbi for CrowdFunding {
    type Abi = Abi;
}

#[async_trait]
impl Contract for CrowdFunding {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;

    async fn initialize(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        argument: InitializationArgument,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        // Validate that the application parameters were configured correctly.
        let _ = runtime.application_parameters();

        self.initialization_argument.set(Some(argument));

        ensure!(
            self.initialization_argument_().deadline > runtime.system_time(),
            Error::DeadlineInThePast
        );

        Ok(ExecutionOutcome::default())
    }

    async fn execute_operation(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        operation: Operation,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        let mut outcome = ExecutionOutcome::default();

        match operation {
            Operation::PledgeWithTransfer { owner, amount } => {
                if runtime.chain_id() == runtime.application_id().creation.chain_id {
                    self.execute_pledge_with_account(runtime, owner, amount)
                        .await?;
                } else {
                    self.execute_pledge_with_transfer(runtime, &mut outcome, owner, amount)?;
                }
            }
            Operation::Collect => self.collect_pledges(runtime)?,
            Operation::Cancel => self.cancel_campaign(runtime).await?,
        }

        Ok(outcome)
    }

    async fn execute_message(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        message: Message,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        match message {
            Message::PledgeWithAccount { owner, amount } => {
                ensure!(
                    runtime.chain_id() == runtime.application_id().creation.chain_id,
                    Error::CampaignChainOnly
                );
                self.execute_pledge_with_account(runtime, owner, amount)
                    .await?;
            }
        }
        Ok(ExecutionOutcome::default())
    }

    async fn handle_application_call(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        call: ApplicationCall,
        sessions: Vec<SessionId>,
    ) -> Result<
        ApplicationCallOutcome<Self::Message, Self::Response, Self::SessionState>,
        Self::Error,
    > {
        let mut outcome = ApplicationCallOutcome::default();
        match call {
            ApplicationCall::PledgeWithSessions { source } => {
                // Only sessions on the campaign chain are supported.
                ensure!(
                    runtime.chain_id() == runtime.application_id().creation.chain_id,
                    Error::CampaignChainOnly
                );
                // In real-life applications, the source could be constrained so that a
                // refund cannot be used as a transfer.
                self.execute_pledge_with_sessions(runtime, source, sessions)
                    .await?
            }
            ApplicationCall::PledgeWithTransfer { owner, amount } => {
                self.execute_pledge_with_transfer(
                    runtime,
                    &mut outcome.execution_outcome,
                    owner,
                    amount,
                )?;
            }
            ApplicationCall::Collect => self.collect_pledges(runtime)?,
            ApplicationCall::Cancel => self.cancel_campaign(runtime).await?,
        }

        Ok(outcome)
    }

    async fn handle_session_call(
        &mut self,
        _runtime: &mut ContractRuntime<Abi>,
        _state: Self::SessionState,
        _call: (),
        _forwarded_sessions: Vec<SessionId>,
    ) -> Result<SessionCallOutcome<Self::Message, Self::Response, Self::SessionState>, Self::Error>
    {
        Err(Error::SessionsNotSupported)
    }
}

impl CrowdFunding {
    fn fungible_id(runtime: &mut ContractRuntime<Abi>) -> ApplicationId<FungibleTokenAbi> {
        // TODO(#723): We should be able to pull the fungible ID from the
        // `required_application_ids` of the application description.
        runtime.application_parameters()
    }

    /// Adds a pledge from a local account to the remote campaign chain.
    fn execute_pledge_with_transfer(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        outcome: &mut ExecutionOutcome<Message>,
        owner: AccountOwner,
        amount: Amount,
    ) -> Result<(), Error> {
        ensure!(amount > Amount::ZERO, Error::EmptyPledge);
        // The campaign chain.
        let chain_id = runtime.application_id().creation.chain_id;
        let fungible_id = Self::fungible_id(runtime);
        // First, move the funds to the campaign chain (under the same owner).
        // TODO(#589): Simplify this when the messaging system guarantees atomic delivery
        // of all messages created in the same operation/message.
        let destination = fungible::Destination::Account(Account { chain_id, owner });
        let call = fungible::ApplicationCall::Transfer {
            owner,
            amount,
            destination,
        };
        runtime.call_application(
            /* authenticated by owner */ true,
            fungible_id,
            &call,
            vec![],
        );
        // Second, schedule the attribution of the funds to the (remote) campaign.
        let message = Message::PledgeWithAccount { owner, amount };
        outcome.messages.push(OutgoingMessage {
            destination: chain_id.into(),
            authenticated: true,
            is_tracked: false,
            resources: Resources::default(),
            message,
        });
        Ok(())
    }

    /// Adds a pledge from a local account to the campaign chain.
    async fn execute_pledge_with_account(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: AccountOwner,
        amount: Amount,
    ) -> Result<(), Error> {
        ensure!(amount > Amount::ZERO, Error::EmptyPledge);
        self.receive_from_account(runtime, owner, amount);
        self.finish_pledge(runtime, owner, amount).await
    }

    /// Adds a pledge sent from an application using token sessions.
    async fn execute_pledge_with_sessions(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        source: AccountOwner,
        sessions: Vec<SessionId>,
    ) -> Result<(), Error> {
        let sessions = self.check_session_tokens(runtime, sessions)?;

        let session_balances = self.query_session_balances(runtime, &sessions)?;
        let amount = session_balances.iter().sum();

        ensure!(amount > Amount::ZERO, Error::EmptyPledge);

        self.collect_session_tokens(runtime, sessions, session_balances);
        self.finish_pledge(runtime, source, amount).await
    }

    /// Checks that the sessions pledged all use the correct token. Marks the sessions
    /// with the correct Abi.
    fn check_session_tokens(
        &self,
        runtime: &mut ContractRuntime<Abi>,
        sessions: Vec<SessionId>,
    ) -> Result<Vec<SessionId<FungibleTokenAbi>>, Error> {
        let fungible_id = Self::fungible_id(runtime).forget_abi();
        ensure!(
            sessions
                .iter()
                .all(|session_id| session_id.application_id == fungible_id),
            Error::IncorrectToken
        );
        let sessions = sessions.into_iter().map(|s| s.with_abi()).collect();
        Ok(sessions)
    }

    /// Gathers the balances in all the pledged sessions.
    fn query_session_balances(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        sessions: &[SessionId<FungibleTokenAbi>],
    ) -> Result<Vec<Amount>, Error> {
        let mut balances = Vec::with_capacity(sessions.len());
        for session in sessions {
            let (response, _) =
                runtime.call_session(false, *session, &fungible::SessionCall::Balance, vec![]);
            match response {
                FungibleResponse::Balance(balance) => balances.push(balance),
                response => return Err(Error::UnexpectedFungibleResponse(response)),
            }
        }
        Ok(balances)
    }

    /// Collects all tokens in the sessions and places them in custody of the campaign.
    fn collect_session_tokens(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        sessions: Vec<SessionId<FungibleTokenAbi>>,
        balances: Vec<Amount>,
    ) {
        for (session, balance) in sessions.into_iter().zip(balances) {
            self.receive_from_session(runtime, session, balance);
        }
    }

    /// Marks a pledge in the application state, so that it can be returned if the campaign is
    /// cancelled.
    async fn finish_pledge(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        source: AccountOwner,
        amount: Amount,
    ) -> Result<(), Error> {
        match self.status.get() {
            Status::Active => {
                self.pledges
                    .get_mut_or_default(&source)
                    .await
                    .expect("view access should not fail")
                    .saturating_add_assign(amount);
                Ok(())
            }
            Status::Complete => {
                self.send_to(runtime, amount, self.initialization_argument_().owner);
                Ok(())
            }
            Status::Cancelled => Err(Error::Cancelled),
        }
    }

    /// Collects all pledges and completes the campaign if the target has been reached.
    fn collect_pledges(&mut self, runtime: &mut ContractRuntime<Abi>) -> Result<(), Error> {
        let total = self.balance(runtime)?;

        match self.status.get() {
            Status::Active => {
                ensure!(
                    total >= self.initialization_argument_().target,
                    Error::TargetNotReached
                );
            }
            Status::Complete => (),
            Status::Cancelled => return Err(Error::Cancelled),
        }

        self.send_to(runtime, total, self.initialization_argument_().owner);
        self.pledges.clear();
        self.status.set(Status::Complete);

        Ok(())
    }

    /// Cancels the campaign if the deadline has passed, refunding all pledges.
    async fn cancel_campaign(&mut self, runtime: &mut ContractRuntime<Abi>) -> Result<(), Error> {
        ensure!(!self.status.get().is_complete(), Error::Completed);

        // TODO(#728): Remove this.
        #[cfg(not(any(test, feature = "test")))]
        ensure!(
            runtime.system_time() >= self.initialization_argument_().deadline,
            Error::DeadlineNotReached
        );

        let mut pledges = Vec::new();
        self.pledges
            .for_each_index_value(|pledger, amount| {
                pledges.push((pledger, amount));
                Ok(())
            })
            .await
            .expect("view iteration should not fail");
        for (pledger, amount) in pledges {
            self.send_to(runtime, amount, pledger);
        }

        let balance = self.balance(runtime)?;
        self.send_to(runtime, balance, self.initialization_argument_().owner);
        self.status.set(Status::Cancelled);

        Ok(())
    }

    /// Queries the token application to determine the total amount of tokens in custody.
    fn balance(&mut self, runtime: &mut ContractRuntime<Abi>) -> Result<Amount, Error> {
        let owner = AccountOwner::Application(runtime.application_id().forget_abi());
        let fungible_id = Self::fungible_id(runtime);
        let (response, _) = runtime.call_application(
            true,
            fungible_id,
            &fungible::ApplicationCall::Balance { owner },
            vec![],
        );
        match response {
            fungible::FungibleResponse::Balance(balance) => Ok(balance),
            response => Err(Error::UnexpectedFungibleResponse(response)),
        }
    }

    /// Transfers `amount` tokens from the funds in custody to the `destination`.
    fn send_to(&mut self, runtime: &mut ContractRuntime<Abi>, amount: Amount, owner: AccountOwner) {
        let fungible_id = Self::fungible_id(runtime);
        let account = Account {
            chain_id: runtime.chain_id(),
            owner,
        };
        let destination = Destination::Account(account);
        let transfer = fungible::ApplicationCall::Transfer {
            owner: AccountOwner::Application(runtime.application_id().forget_abi()),
            amount,
            destination,
        };
        runtime.call_application(true, fungible_id, &transfer, vec![]);
    }

    /// Calls into the Fungible Token application to receive tokens from the given account.
    fn receive_from_account(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        owner: AccountOwner,
        amount: Amount,
    ) {
        let fungible_id = Self::fungible_id(runtime);
        let account = Account {
            chain_id: runtime.chain_id(),
            owner: AccountOwner::Application(runtime.application_id().forget_abi()),
        };
        let destination = Destination::Account(account);
        let transfer = fungible::ApplicationCall::Transfer {
            owner,
            amount,
            destination,
        };
        runtime.call_application(true, fungible_id, &transfer, vec![]);
    }

    /// Calls into the Fungible Token application to receive tokens from the given account.
    fn receive_from_session(
        &mut self,
        runtime: &mut ContractRuntime<Abi>,
        session: SessionId<FungibleTokenAbi>,
        amount: Amount,
    ) {
        let account = Account {
            chain_id: runtime.chain_id(),
            owner: AccountOwner::Application(runtime.application_id().forget_abi()),
        };
        let destination = Destination::Account(account);
        let transfer = fungible::SessionCall::Transfer {
            amount,
            destination,
        };
        runtime.call_session(false, session, &transfer, vec![]);
    }

    // Trailing underscore to avoid conflict with the generated GraphQL function.
    pub fn initialization_argument_(&self) -> &InitializationArgument {
        self.initialization_argument
            .get()
            .as_ref()
            .expect("Application is not running on the host chain or was not initialized yet")
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Action can only be executed on the chain that created the crowd-funding campaign
    #[error("Action can only be executed on the chain that created the crowd-funding campaign")]
    CampaignChainOnly,

    /// Crowd-funding application doesn't support any cross-application sessions.
    #[error("Crowd-funding application doesn't support any cross-application sessions")]
    SessionsNotSupported,

    /// Crowd-funding campaign cannot start after its deadline.
    #[error("Crowd-funding campaign cannot start after its deadline")]
    DeadlineInThePast,

    /// A pledge can not be empty.
    #[error("Pledge is empty")]
    EmptyPledge,

    /// Pledge used a token that's not the same as the one in the campaign's [`InitializationArgument`].
    #[error("Pledge uses the incorrect token")]
    IncorrectToken,

    /// Pledge used a destination that's not the same as this campaign's [`ApplicationId`].
    #[error("Pledge uses the incorrect destination account")]
    IncorrectDestination,

    /// Cross-application call without a source application ID.
    #[error("Applications must identify themselves to perform transfers")]
    MissingSourceApplication,

    /// Can't collect pledges before the campaign target has been reached.
    #[error("Crowd-funding campaign has not reached its target yet")]
    TargetNotReached,

    /// Can't cancel a campaign before its deadline.
    #[error("Crowd-funding campaign has not reached its deadline yet")]
    DeadlineNotReached,

    /// Can't cancel a campaign after it has been completed.
    #[error("Crowd-funding campaign has already been completed")]
    Completed,

    /// Can't pledge to or collect pledges from a cancelled campaign.
    #[error("Crowd-funding campaign has been cancelled")]
    Cancelled,

    /// Failed to deserialize BCS bytes
    #[error("Failed to deserialize BCS bytes")]
    BcsError(#[from] bcs::Error),

    /// Failed to deserialize JSON string
    #[error("Failed to deserialize JSON string")]
    JsonError(#[from] serde_json::Error),

    /// Unexpected response from fungible token application.
    #[error("Unexpected response from fungible token application: {0:?}")]
    UnexpectedFungibleResponse(FungibleResponse),
}
