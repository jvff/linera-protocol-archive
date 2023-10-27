// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! An actor implementation to handle a user application runtime.

mod handlers;
mod requests;
mod responses;

use self::{
    handlers::RequestHandler,
    responses::{SyncResponse, SyncResponseSender},
};
pub use self::{
    requests::{BaseRequest, ContractRequest, ServiceRequest},
    responses::CanceledError,
};
use crate::ExecutionError;
use futures::{
    channel::{mpsc, oneshot},
    select,
    stream::{FuturesUnordered, StreamExt},
};

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
        let (sender, receiver) = mpsc::unbounded();

        let actor = RuntimeActor {
            runtime,
            requests: receiver,
        };

        (actor, sender)
    }

    /// Runs the [`RuntimeActor`], handling `Request`s until all the sender endpoints are closed.
    pub async fn run(mut self) -> Result<(), ExecutionError> {
        use tracing::Instrument;
        async move {
            let mut active_requests = FuturesUnordered::new();

            loop {
                select! {
                    maybe_result = active_requests.next() => if let Some(result) = maybe_result {
                        result?;
                    },
                    maybe_request = self.requests.next() => match maybe_request {
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
        .instrument(tracing::error_span!("RuntimeActor::run"))
        .await
    }
}

/// Extension trait to help with sending requests to [`RuntimeActor`]s.
///
/// Prepares a channel for the actor to send a response back to the sender of the request.
pub trait SendRequestExt<Request> {
    /// Sends a synchronous request built by `builder`, blocking until the `Response` is received.
    fn sync_request<Response>(
        &self,
        builder: impl FnOnce(SyncResponseSender<Response>) -> Request,
    ) -> Result<Response, CanceledError>
    where
        Response: Send;

    /// Sends an asynchronous request built by `builder`, returning a [`oneshot::Receiver`] for
    /// awaiting the `Response`.
    fn async_request<Response>(
        &self,
        builder: impl FnOnce(oneshot::Sender<Response>) -> Request,
    ) -> oneshot::Receiver<Response>
    where
        Response: Send;
}

impl<Request> SendRequestExt<Request> for mpsc::UnboundedSender<Request>
where
    Request: Send,
{
    fn sync_request<Response>(
        &self,
        builder: impl FnOnce(SyncResponseSender<Response>) -> Request,
    ) -> Result<Response, CanceledError>
    where
        Response: Send,
    {
        let (response_sender, response_receiver) = SyncResponse::channel();
        let request = builder(response_sender);

        self.unbounded_send(request).unwrap_or_else(|error| {
            panic!("Failed to send request because receiver has stopped listening: {error}")
        });

        response_receiver.wait()
    }

    fn async_request<Response>(
        &self,
        builder: impl FnOnce(oneshot::Sender<Response>) -> Request,
    ) -> oneshot::Receiver<Response>
    where
        Response: Send,
    {
        let (response_sender, response_receiver) = oneshot::channel();
        let request = builder(response_sender);

        self.unbounded_send(request).unwrap_or_else(|error| {
            panic!("Failed to send request because receiver has stopped listening: {error}")
        });

        response_receiver
    }
}
