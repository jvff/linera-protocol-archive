// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! An actor implementation to handle a user application runtime.

mod handlers;
mod requests;
mod responses;

pub use self::requests::{BaseRequest, ContractRequest, ServiceRequest};
use self::{handlers::RequestHandler, responses::SyncResponse};
use crate::ExecutionError;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use tokio::{
    select,
    sync::{mpsc, oneshot},
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
            tracing::info!("Waiting for event");
            select! {
                Some(result) = active_requests.next() => {
                    tracing::info!("Active request finished");
                    result?
                },
                maybe_request = self.requests.recv() => {
                    tracing::info!("New request received");
                    match maybe_request {
                        Some(request) => active_requests.push(self.runtime.handle_request(request)),
                        None => break,
                    }
                },
            }
        }

        while !active_requests.is_empty() {
            tracing::info!("Waiting for active requests to finish");
            if let Some(result) = active_requests.next().await {
                tracing::info!("Active request finished");
                result?;
            }
        }

        Ok(())
    }
}

/// Extension trait to help with sending requests to [`RuntimeActor`]s.
///
/// Prepares a channel for the actor to send a response back to the sender of the request.
pub trait SendRequestExt<Request> {
    /// Sends a synchronous request built by `builder`, blocking until the `Response` is received.
    fn sync_request<Response>(
        &self,
        builder: impl FnOnce(Arc<SyncResponse<Response>>) -> Request,
    ) -> Response
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
        builder: impl FnOnce(Arc<SyncResponse<Response>>) -> Request,
    ) -> Response
    where
        Response: Send,
    {
        let response = Arc::new(SyncResponse::default());
        let request = builder(response.clone());

        tracing::info!("Sending sync request");
        self.send(request).unwrap_or_else(|error| {
            panic!("Failed to send request because receiver has stopped listening: {error}")
        });

        tokio::task::block_in_place(|| response.wait())
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

        tracing::info!("Sending async request");
        self.send(request).unwrap_or_else(|error| {
            panic!("Failed to send request because receiver has stopped listening: {error}")
        });

        response_receiver
    }
}
