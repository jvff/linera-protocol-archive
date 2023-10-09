// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! An actor implementation to handle a user application runtime.

mod handlers;
mod requests;
mod responses;

use self::handlers::RequestHandler;
pub use self::requests::{BaseRequest, ContractRequest};
use crate::ExecutionError;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::{select, sync::mpsc};

/// A handler of application system APIs that runs as a separate actor.
///
/// Receives `Request`s from the application and handles them using the `Runtime`.
pub struct RuntimeActor<Runtime, Request> {
    runtime: Runtime,
    requests: mpsc::UnboundedReceiver<Request>,
}

impl<Runtime, Request> RuntimeActor<Runtime, Request>
where
    Runtime: RequestHandler<Request>,
    Request: std::fmt::Debug,
{
    /// Creates a new [`RuntimeActor`] using the provided `Runtime` to handle `Request`s.
    ///
    /// Returns the new [`RuntimeActor`] so that it can be executed later with the
    /// [`RuntimeActor::run`] method and the endpoint to send `Request`s to the actor.
    pub fn new(runtime: Runtime) -> (Self, mpsc::UnboundedSender<Request>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let actor = RuntimeActor {
            runtime,
            requests: receiver,
        };

        (actor, sender)
    }

    /// Runs the [`RuntimeActor`], handling `Request`s until all the sender endpoints are closed.
    pub async fn run(mut self) -> Result<(), ExecutionError> {
        let mut active_requests = FuturesUnordered::new();

        loop {
            select! {
                Some(result) = active_requests.next() => result?,
                maybe_request = self.requests.recv() => match maybe_request {
                    Some(request) => active_requests.push(self.runtime.handle_request(request)),
                    None => break,
                },
            }
        }

        while !active_requests.is_empty() {
            if let Some(result) = active_requests.next().await {
                result?;
            }
        }

        Ok(())
    }
}
