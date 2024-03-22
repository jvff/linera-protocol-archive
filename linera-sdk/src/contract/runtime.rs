// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to interface with the host executing the contract.

use linera_base::{
    abi::ContractAbi,
    data_types::{Amount, BlockHeight, OutgoingMessage, Resources, Timestamp},
    identifiers::{Account, ApplicationId, ChainId, ChannelName, Destination, MessageId, Owner},
    ownership::ChainOwnership,
};
use serde::Serialize;

use super::{wit_system_api as wit, CloseChainError};
use crate::Contract;

/// The common runtime to interface with the host executing the contract.
///
/// It automatically caches read-only values received from the host.
#[derive(Debug)]
pub struct ContractRuntime<Application>
where
    Application: Contract,
{
    application_parameters: Option<<Application::Abi as ContractAbi>::Parameters>,
    application_id: Option<ApplicationId<Application::Abi>>,
    chain_id: Option<ChainId>,
    authenticated_signer: Option<Option<Owner>>,
    block_height: Option<BlockHeight>,
    message_is_bouncing: Option<Option<bool>>,
    message_id: Option<Option<MessageId>>,
    authenticated_caller_id: Option<Option<ApplicationId>>,
    timestamp: Option<Timestamp>,
}

impl<Application> ContractRuntime<Application>
where
    Application: Contract,
{
    /// Creates a new [`ContractRuntime`] instance for a contract.
    pub(crate) fn new() -> Self {
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

    /// Returns the application parameters provided when the application was created.
    pub fn application_parameters(&mut self) -> <Application::Abi as ContractAbi>::Parameters {
        self.application_parameters
            .get_or_insert_with(|| {
                let bytes = wit::application_parameters();
                serde_json::from_slice(&bytes)
                    .expect("Application parameters must be deserializable")
            })
            .clone()
    }

    /// Returns the ID of the current application.
    pub fn application_id(&mut self) -> ApplicationId<Application::Abi> {
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

    /// Returns the balance of one of the accounts on this chain.
    pub fn owner_balance(&mut self, owner: Owner) -> Amount {
        wit::read_owner_balance(owner.into()).into()
    }

    /// Schedules a message to be sent to this application on another chain.
    pub fn send_message(
        &mut self,
        destination: impl Into<Destination>,
        message: Application::Message,
    ) -> MessageSender<Application::Message> {
        MessageSender::new(destination.into(), message)
    }

    /// Subscribes to a message channel from another chain.
    pub fn subscribe(&mut self, chain: ChainId, channel: ChannelName) {
        wit::subscribe(chain.into(), (&channel).into());
    }

    /// Unsubscribes to a message channel from another chain.
    pub fn unsubscribe(&mut self, chain: ChainId, channel: ChannelName) {
        wit::unsubscribe(chain.into(), (&channel).into());
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
    ) -> A::Response {
        let call_bytes = bcs::to_bytes(call)
            .expect("Failed to serialize `ApplicationCall` type for a cross-application call");

        let response_bytes =
            wit::try_call_application(authenticated, application.forget_abi().into(), &call_bytes);

        bcs::from_bytes(&response_bytes)
            .expect("Failed to deserialize `Response` type from cross-application call")
    }
}

/// A helper type that uses the builder pattern to configure how a message is sent, and then
/// sends the message once it is dropped.
pub struct MessageSender<Message>
where
    Message: Serialize,
{
    message: OutgoingMessage<Message>,
}

impl<Message> MessageSender<Message>
where
    Message: Serialize,
{
    /// Creates a new [`MessageSender`] instance to send the `message` to the `destination`.
    pub fn new(destination: Destination, message: Message) -> Self {
        MessageSender {
            message: OutgoingMessage {
                destination,
                authenticated: false,
                is_tracked: false,
                grant: Resources::default(),
                message,
            },
        }
    }

    /// Marks the message to be tracked, so that the sender is notified if the message is received
    /// or receives the message back if it is rejected.
    pub fn with_tracking(mut self) -> Self {
        self.message.is_tracked = true;
        self
    }

    /// Forwards the authenticated signer with the message.
    pub fn with_authentication(mut self) -> Self {
        self.message.authenticated = true;
        self
    }

    /// Forwards a grant of resources so the receiver can use it to pay for receiving the message.
    pub fn with_grant(mut self, grant: Resources) -> Self {
        self.message.grant = grant;
        self
    }
}

impl<Message> Drop for MessageSender<Message>
where
    Message: Serialize,
{
    fn drop(&mut self) {
        let serialized_message =
            bcs::to_bytes(&self.message.message).expect("Failed to serialize message to be sent");

        let raw_message = OutgoingMessage {
            destination: self.message.destination.clone(),
            authenticated: self.message.authenticated,
            is_tracked: self.message.is_tracked,
            grant: self.message.grant,
            message: serialized_message,
        };

        wit::send_message((&raw_message).into())
    }
}
