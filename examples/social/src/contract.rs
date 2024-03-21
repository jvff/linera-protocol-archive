// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(target_arch = "wasm32", no_main)]

mod state;

use async_trait::async_trait;
use linera_sdk::{
    base::{ChannelName, Destination, MessageId, WithContractAbi},
    views::ViewError,
    ApplicationCallOutcome, Contract, ContractRuntime, ExecutionOutcome, ViewStateStorage,
};
use social::{Key, Message, Operation, OwnPost, SocialAbi};
use state::Social;
use thiserror::Error;

/// The channel name the application uses for cross-chain messages about new posts.
const POSTS_CHANNEL_NAME: &[u8] = b"posts";
/// The number of recent posts sent in each cross-chain message.
const RECENT_POSTS: usize = 10;

pub struct SocialContract {
    state: Social,
    runtime: ContractRuntime<SocialAbi>,
}

linera_sdk::contract!(SocialContract);

impl WithContractAbi for SocialContract {
    type Abi = SocialAbi;
}

#[async_trait]
impl Contract for SocialContract {
    type Error = Error;
    type Storage = ViewStateStorage<Self>;
    type State = Social;

    async fn new(state: Social, runtime: ContractRuntime<Self::Abi>) -> Result<Self, Self::Error> {
        Ok(SocialContract { state, runtime })
    }

    fn state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    async fn initialize(
        &mut self,
        _argument: (),
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        // Validate that the application parameters were configured correctly.
        let _ = self.runtime.application_parameters();

        Ok(ExecutionOutcome::default())
    }

    async fn execute_operation(
        &mut self,
        operation: Operation,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        match operation {
            Operation::Subscribe { chain_id } => {
                Ok(ExecutionOutcome::default().with_message(chain_id, Message::Subscribe))
            }
            Operation::Unsubscribe { chain_id } => {
                Ok(ExecutionOutcome::default().with_message(chain_id, Message::Unsubscribe))
            }
            Operation::Post { text } => self.execute_post_operation(text).await,
        }
    }

    async fn execute_message(
        &mut self,
        message: Message,
    ) -> Result<ExecutionOutcome<Self::Message>, Self::Error> {
        let mut outcome = ExecutionOutcome::default();
        let message_id = self
            .runtime
            .message_id()
            .expect("Message ID has to be available when executing a message");
        match message {
            Message::Subscribe => outcome.subscribe.push((
                ChannelName::from(POSTS_CHANNEL_NAME.to_vec()),
                message_id.chain_id,
            )),
            Message::Unsubscribe => outcome.unsubscribe.push((
                ChannelName::from(POSTS_CHANNEL_NAME.to_vec()),
                message_id.chain_id,
            )),
            Message::Posts { count, posts } => {
                self.execute_posts_message(message_id, count, posts)?
            }
        }
        Ok(outcome)
    }

    async fn handle_application_call(
        &mut self,
        _call: (),
    ) -> Result<ApplicationCallOutcome<Self::Message, Self::Response>, Self::Error> {
        Err(Error::ApplicationCallsNotSupported)
    }
}

impl SocialContract {
    async fn execute_post_operation(
        &mut self,
        text: String,
    ) -> Result<ExecutionOutcome<Message>, Error> {
        let timestamp = self.runtime.system_time();
        self.state.own_posts.push(OwnPost { timestamp, text });
        let count = self.state.own_posts.count();
        let mut posts = vec![];
        for index in (0..count).rev().take(RECENT_POSTS) {
            let maybe_post = self.state.own_posts.get(index).await?;
            let own_post = maybe_post
                .expect("post with valid index missing; this is a bug in the social application!");
            posts.push(own_post);
        }
        let count = count as u64;
        let dest = Destination::Subscribers(ChannelName::from(POSTS_CHANNEL_NAME.to_vec()));
        Ok(ExecutionOutcome::default().with_message(dest, Message::Posts { count, posts }))
    }

    fn execute_posts_message(
        &mut self,
        message_id: MessageId,
        count: u64,
        posts: Vec<OwnPost>,
    ) -> Result<(), Error> {
        for (index, post) in (0..count).rev().zip(posts) {
            let key = Key {
                timestamp: post.timestamp,
                author: message_id.chain_id,
                index,
            };
            self.state.received_posts.insert(&key, post.text)?;
        }
        Ok(())
    }
}

/// An error that can occur during the contract execution.
#[derive(Debug, Error)]
pub enum Error {
    /// Social application doesn't support any cross-application calls.
    #[error("Social application doesn't support any application calls")]
    ApplicationCallsNotSupported,

    /// View error.
    #[error(transparent)]
    View(#[from] ViewError),

    /// Failed to deserialize BCS bytes
    #[error("Failed to deserialize BCS bytes")]
    BcsError(#[from] bcs::Error),

    /// Failed to deserialize JSON string
    #[error("Failed to deserialize JSON string")]
    JsonError(#[from] serde_json::Error),
}
