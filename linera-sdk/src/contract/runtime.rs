// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the contract.

use super::{wit_system_api as wit, CloseChainError};
use linera_base::{
    abi::ContractAbi,
    data_types::{Amount, BlockHeight, Timestamp},
    identifiers::{Account, ApplicationId, ChainId, MessageId, Owner, SessionId},
    ownership::ChainOwnership,
};

/// The common runtime to interface with the host executing the contract.
///
/// It automatically caches read-only values received from the host.
#[derive(Clone, Debug)]
pub struct ContractRuntime<Abi>
where
    Abi: ContractAbi,
{
    application_parameters: Option<Abi::Parameters>,
    application_id: Option<ApplicationId<Abi>>,
    chain_id: Option<ChainId>,
    authenticated_signer: Option<Option<Owner>>,
    block_height: Option<BlockHeight>,
    message_is_bouncing: Option<Option<bool>>,
    message_id: Option<Option<MessageId>>,
    authenticated_caller_id: Option<Option<ApplicationId>>,
    timestamp: Option<Timestamp>,
}

impl<Abi> Default for ContractRuntime<Abi>
where
    Abi: ContractAbi,
{
    fn default() -> Self {
        ContractRuntime {
            application_parameters: None,
            application_id: None,
            chain_id: None,
            authenticated_signer: None,
            block_height: None,
            message_is_bouncing: None,
            message_id: None,
            authenticated_caller_id: None,
            timestamp: None,
        }
    }
}

impl<Abi> ContractRuntime<Abi>
where
    Abi: ContractAbi,
{
    /// Returns the application parameters provided when the application was created.
    pub fn application_parameters(&mut self) -> Abi::Parameters {
        self.application_parameters
            .get_or_insert_with(|| {
                let bytes = wit::application_parameters();
                serde_json::from_slice(&bytes)
                    .expect("Application parameters must be deserializable")
            })
            .clone()
    }

    /// Returns the ID of the current application.
    pub fn application_id(&mut self) -> ApplicationId<Abi> {
        *self
            .application_id
            .get_or_insert_with(|| ApplicationId::from(wit::application_id()).with_abi())
    }

    /// Returns the ID of the current chain.
    pub fn chain_id(&mut self) -> ChainId {
        *self.chain_id.get_or_insert_with(|| wit::chain_id().into())
    }

    /// Returns the authenticated signer for this execution, if there is one.
    pub fn authenticated_signer(&mut self) -> Option<Owner> {
        *self
            .authenticated_signer
            .get_or_insert_with(|| wit::authenticated_signer().map(Owner::from))
    }

    /// Returns the height of the current block that is executing.
    pub fn block_height(&mut self) -> BlockHeight {
        *self
            .block_height
            .get_or_insert_with(|| wit::block_height().into())
    }

    /// Returns the ID of the incoming message that is being handled, or [`None`] if not executing
    /// an incoming message.
    pub fn message_id(&mut self) -> Option<MessageId> {
        *self
            .message_id
            .get_or_insert_with(|| wit::message_id().map(MessageId::from))
    }

    /// Returns [`true`] if the incoming message was rejected from the original destination and is
    /// now bouncing back, or [`None`] if not executing an incoming message.
    pub fn message_is_bouncing(&mut self) -> Option<bool> {
        *self
            .message_is_bouncing
            .get_or_insert_with(wit::message_is_bouncing)
    }

    /// Returns the authenticated caller ID, if the caller configured it and if the current context
    /// is executing a cross-application call.
    pub fn authenticated_caller_id(&mut self) -> Option<ApplicationId> {
        *self
            .authenticated_caller_id
            .get_or_insert_with(|| wit::authenticated_caller_id().map(ApplicationId::from))
    }

    /// Retrieves the current system time, i.e. the timestamp of the block in which this is called.
    pub fn system_time(&mut self) -> Timestamp {
        *self
            .timestamp
            .get_or_insert_with(|| wit::read_system_timestamp().into())
    }

    /// Returns the current chain balance.
    pub fn chain_balance(&mut self) -> Amount {
        wit::read_chain_balance().into()
    }

    /// Returns the balance of one of the chain owners.
    pub fn owner_balance(&mut self, owner: Owner) -> Amount {
        wit::read_owner_balance(owner.into()).into()
    }

    /// Transfers an `amount` of native tokens from `source` owner account (or the current chain's
    /// balance) to `destination`.
    pub fn transfer(&mut self, source: Option<Owner>, destination: Account, amount: Amount) {
        wit::transfer(
            source.map(|source| source.into()),
            destination.into(),
            amount.into(),
        )
    }

    /// Claims an `amount` of native tokens from a `source` account to a `destination` account.
    pub fn claim(&mut self, source: Account, destination: Account, amount: Amount) {
        wit::claim(source.into(), destination.into(), amount.into())
    }

    /// Retrieves the owner configuration for the current chain.
    pub fn chain_ownership(&mut self) -> ChainOwnership {
        wit::chain_ownership().into()
    }

    /// Closes the current chain. Returns an error if the application doesn't have
    /// permission to do so.
    pub fn close_chain(&mut self) -> Result<(), CloseChainError> {
        wit::close_chain()
    }

    /// Calls another application.
    pub fn call_application<A: ContractAbi + Send>(
        &mut self,
        authenticated: bool,
        application: ApplicationId<A>,
        call: &A::ApplicationCall,
        forwarded_sessions: Vec<SessionId>,
    ) -> (A::Response, Vec<SessionId>) {
        let call_bytes = bcs::to_bytes(call)
            .expect("Failed to serialize `ApplicationCall` type for a cross-application call");
        let forwarded_sessions = forwarded_sessions
            .into_iter()
            .map(wit::SessionId::from)
            .collect::<Vec<_>>();

        let (response_bytes, session_ids) = wit::try_call_application(
            authenticated,
            application.forget_abi().into(),
            &call_bytes,
            &forwarded_sessions,
        )
        .into();

        let response = bcs::from_bytes(&response_bytes)
            .expect("Failed to deserialize `Response` type from cross-application call");
        (response, session_ids)
    }

    /// Calls a session from another application.
    pub fn call_session<A: ContractAbi + Send>(
        &mut self,
        authenticated: bool,
        session: SessionId<A>,
        call: &A::SessionCall,
        forwarded_sessions: Vec<SessionId>,
    ) -> (A::Response, Vec<SessionId>) {
        let call_bytes =
            bcs::to_bytes(call).expect("Failed to serialize `SessionCall` type for session call");
        let forwarded_sessions = forwarded_sessions
            .into_iter()
            .map(wit::SessionId::from)
            .collect::<Vec<_>>();

        let (response_bytes, ids) = wit::try_call_session(
            authenticated,
            session.forget_abi().into(),
            &call_bytes,
            &forwarded_sessions,
        )
        .into();

        let response = bcs::from_bytes(&response_bytes)
            .expect("Failed to deserialize `Response` type from session call");
        (response, ids)
    }
}
