// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use self::state::MetaCounter;
use async_trait::async_trait;
use linera_sdk::{
    base::{ApplicationId, WithContractAbi},
    ApplicationCallOutcome, Contract, ContractRuntime, ExecutionOutcome, OutgoingMessage,
    Resources, SimpleStateStorage,
};
use meta_counter::{Message, MetaCounterAbi, Operation};
use thiserror::Error;

pub struct MetaCounterContract {
    state: MetaCounter,
    runtime: ContractRuntime<MetaCounterAbi>,
}

linera_sdk::contract!(MetaCounterContract);

impl MetaCounterContract {
    fn counter_id(&mut self) -> ApplicationId<counter::CounterAbi> {
        self.runtime.application_parameters()
    }
}

impl WithContractAbi for MetaCounterContract {
    type Abi = MetaCounterAbi;
}

#[async_trait]
impl Contract for MetaCounterContract {
    type Error = Error;
    type Storage = SimpleStateStorage<Self>;
    type State = MetaCounter;

    async fn new(
        state: MetaCounter,
        runtime: ContractRuntime<Self::Abi>,
    ) -> Result<Self, Self::Error> {
        Ok(MetaCounterContract { state, runtime })
    }

    fn state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    async fn initialize(
        &mut self,
        _argument: (),
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        // Validate that the application parameters were configured correctly.
        self.counter_id();
        // Send a no-op message to ourselves. This is only for testing contracts that send messages
        // on initialization. Since the value is 0 it does not change the counter value.
        Ok(
            ExecutionOutcome::default()
                .with_message(self.runtime.chain_id(), Message::Increment(0)),
        )
    }

    async fn execute_operation(
        &mut self,
        operation: Operation,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        log::trace!("operation: {:?}", operation);
        let Operation {
            recipient_id,
            authenticated,
            is_tracked,
            fuel_grant,
            message,
        } = operation;
        let message = OutgoingMessage {
            destination: recipient_id.into(),
            authenticated,
            is_tracked,
            resources: Resources {
                fuel: fuel_grant,
                ..Default::default()
            },
            message,
        };
        let mut outcome = ExecutionOutcome::default();
        outcome.messages.push(message);
        Ok(outcome)
    }

    async fn execute_message(
        &mut self,
        message: Message,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        let is_bouncing = self
            .runtime
            .message_is_bouncing()
            .expect("Message delivery status has to be available when executing a message");
        if is_bouncing {
            log::trace!("receiving a bouncing message {message:?}");
            return Ok(ExecutionOutcome::default());
        }
        match message {
            Message::Fail => {
                log::trace!("failing message {message:?} on purpose");
                Err(Error::MessageFailed)
            }
            Message::Increment(value) => {
                let counter_id = self.counter_id();
                log::trace!("executing {} via {:?}", value, counter_id);
                self.runtime.call_application(true, counter_id, &value);
                Ok(ExecutionOutcome::default())
            }
        }
    }

    async fn handle_application_call(
        &mut self,
        _call: (),
    ) -> Result<ApplicationCallOutcome<Self::Message, Self::Response>, Self::Error> {
        Err(Error::CallsNotSupported)
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    #[error("MetaCounter application doesn't support any cross-chain messages")]
    MessagesNotSupported,

    #[error("MetaCounter application doesn't support any cross-application calls")]
    CallsNotSupported,

    #[error("Message failed intentionally")]
    MessageFailed,

    /// Failed to deserialize BCS bytes
    #[error("Failed to deserialize BCS bytes")]
    BcsError(#[from] bcs::Error),

    /// Failed to deserialize JSON string
    #[error("Failed to deserialize JSON string")]
    JsonError(#[from] serde_json::Error),
}
