// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Runtime types to simulate interfacing with the host executing the contract.

use std::marker::PhantomData;

use linera_base::{
    abi::ContractAbi,
    data_types::{Amount, BlockHeight, Resources, Timestamp},
    identifiers::{Account, ApplicationId, ChainId, ChannelName, Destination, MessageId, Owner},
    ownership::{ChainOwnership, CloseChainError},
};
use serde::Serialize;

use crate::Contract;

/// A mock of the common runtime to interface with the host executing the contract.
pub struct ContractRuntime<Application>
where
    Application: Contract,
{
    _application: PhantomData<Application>,
}

impl<Application> Default for ContractRuntime<Application>
where
    Application: Contract,
{
    fn default() -> Self {
        ContractRuntime::new()
    }
}

impl<Application> ContractRuntime<Application>
where
    Application: Contract,
{
    /// Creates a new [`ContractRuntime`] instance for a contract.
    pub fn new() -> Self {
        ContractRuntime {
            _application: PhantomData,
        }
    }

    /// Returns the application parameters provided when the application was created.
    pub fn application_parameters(&mut self) -> Application::Parameters {
        todo!();
    }

    /// Returns the ID of the current application.
    pub fn application_id(&mut self) -> ApplicationId<Application::Abi> {
        todo!();
    }

    /// Returns the ID of the current chain.
    pub fn chain_id(&mut self) -> ChainId {
        todo!();
    }

    /// Returns the authenticated signer for this execution, if there is one.
    pub fn authenticated_signer(&mut self) -> Option<Owner> {
        todo!();
    }

    /// Returns the height of the current block that is executing.
    pub fn block_height(&mut self) -> BlockHeight {
        todo!();
    }

    /// Returns the ID of the incoming message that is being handled, or [`None`] if not executing
    /// an incoming message.
    pub fn message_id(&mut self) -> Option<MessageId> {
        todo!();
    }

    /// Returns [`true`] if the incoming message was rejected from the original destination and is
    /// now bouncing back, or [`None`] if not executing an incoming message.
    pub fn message_is_bouncing(&mut self) -> Option<bool> {
        todo!();
    }

    /// Returns the authenticated caller ID, if the caller configured it and if the current context
    /// is executing a cross-application call.
    pub fn authenticated_caller_id(&mut self) -> Option<ApplicationId> {
        todo!();
    }

    /// Retrieves the current system time, i.e. the timestamp of the block in which this is called.
    pub fn system_time(&mut self) -> Timestamp {
        todo!();
    }

    /// Returns the current chain balance.
    pub fn chain_balance(&mut self) -> Amount {
        todo!();
    }

    /// Returns the balance of one of the accounts on this chain.
    pub fn owner_balance(&mut self, _owner: Owner) -> Amount {
        todo!();
    }

    /// Schedules a message to be sent to this application on another chain.
    pub fn send_message(
        &mut self,
        destination: impl Into<Destination>,
        message: Application::Message,
    ) {
        self.prepare_message(message).send_to(destination)
    }

    /// Returns a `MessageBuilder` to prepare a message to be sent.
    pub fn prepare_message(
        &mut self,
        message: Application::Message,
    ) -> MessageBuilder<Application::Message> {
        MessageBuilder::new(message)
    }

    /// Subscribes to a message channel from another chain.
    pub fn subscribe(&mut self, _chain: ChainId, _channel: ChannelName) {
        todo!();
    }

    /// Unsubscribes to a message channel from another chain.
    pub fn unsubscribe(&mut self, _chain: ChainId, _channel: ChannelName) {
        todo!();
    }

    /// Transfers an `amount` of native tokens from `source` owner account (or the current chain's
    /// balance) to `destination`.
    pub fn transfer(&mut self, _source: Option<Owner>, _destination: Account, _amount: Amount) {
        todo!();
    }

    /// Claims an `amount` of native tokens from a `source` account to a `destination` account.
    pub fn claim(&mut self, _source: Account, _destination: Account, _amount: Amount) {
        todo!();
    }

    /// Retrieves the owner configuration for the current chain.
    pub fn chain_ownership(&mut self) -> ChainOwnership {
        todo!();
    }

    /// Closes the current chain. Returns an error if the application doesn't have
    /// permission to do so.
    pub fn close_chain(&mut self) -> Result<(), CloseChainError> {
        todo!();
    }

    /// Calls another application.
    pub fn call_application<A: ContractAbi + Send>(
        &mut self,
        _authenticated: bool,
        _application: ApplicationId<A>,
        _call: &A::Operation,
    ) -> A::Response {
        todo!();
    }
}

/// A helper type that uses the builder pattern to configure how a message is sent, and then
/// sends the message once it is dropped.
#[must_use]
pub struct MessageBuilder<Message>
where
    Message: Serialize,
{
    authenticated: bool,
    is_tracked: bool,
    grant: Resources,
    message: Message,
}

impl<Message> MessageBuilder<Message>
where
    Message: Serialize,
{
    /// Creates a new [`MessageBuilder`] instance to send the `message` to the `destination`.
    pub(crate) fn new(message: Message) -> Self {
        MessageBuilder {
            authenticated: false,
            is_tracked: false,
            grant: Resources::default(),
            message,
        }
    }

    /// Marks the message to be tracked, so that the sender receives the message back if it is
    /// rejected by the receiver.
    pub fn with_tracking(mut self) -> Self {
        self.is_tracked = true;
        self
    }

    /// Forwards the authenticated signer with the message.
    pub fn with_authentication(mut self) -> Self {
        self.authenticated = true;
        self
    }

    /// Forwards a grant of resources so the receiver can use it to pay for receiving the message.
    pub fn with_grant(mut self, grant: Resources) -> Self {
        self.grant = grant;
        self
    }

    /// Schedules this `Message` to be sent to the `destination`.
    pub fn send_to(self, _destination: impl Into<Destination>) {
        todo!();
    }
}
