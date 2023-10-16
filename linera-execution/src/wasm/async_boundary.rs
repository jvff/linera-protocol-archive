// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types to handle async code between the host WebAssembly runtime and guest WebAssembly
//! modules.

use super::{
    async_determinism::HostFutureQueue,
    common::{ApplicationRuntimeContext, WasmRuntimeContext},
    ExecutionError, WasmExecutionError,
};
use futures::{channel::oneshot, ready, stream::StreamExt, FutureExt};
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};
use tokio::sync::Mutex;

/// A future implemented in a Wasm module.
pub struct GuestFuture<Future, Application>
where
    Application: ApplicationRuntimeContext,
{
    /// A WIT resource type implementing a [`GuestFutureInterface`] so that it can be polled.
    future: Future,

    /// Types necessary to call the guest Wasm module in order to poll the future.
    context: WasmRuntimeContext<Application>,
}

impl<Future, Application> GuestFuture<Future, Application>
where
    Application: ApplicationRuntimeContext,
{
    /// Creates a [`GuestFuture`] instance with a provided `future` and Wasm execution `context`.
    pub fn new(future: Future, context: WasmRuntimeContext<Application>) -> Self {
        GuestFuture { future, context }
    }
}

impl<InnerFuture, Application> Future for GuestFuture<InnerFuture, Application>
where
    InnerFuture: GuestFutureInterface<Application> + Unpin,
    Application: ApplicationRuntimeContext + Unpin,
    Application::Store: Unpin,
    Application::Error: Unpin,
    Application::Extra: Unpin,
{
    type Output = Result<InnerFuture::Output, ExecutionError>;

    /// Polls the guest future after the [`HostFutureQueue`] in the [`WasmRuntimeContext`] indicates
    /// that it's safe to do so without breaking determinism.
    ///
    /// Uses the runtime context to call the Wasm future's `poll` method, as implemented in the
    /// [`GuestFutureInterface`]. The `task_context` is stored in the runtime context's
    /// [`WakerForwarder`], so that any host futures the guest calls can use the correct task
    /// context.
    fn poll(self: Pin<&mut Self>, task_context: &mut Context) -> Poll<Self::Output> {
        let GuestFuture { future, context } = self.get_mut();

        ready!(context.future_queue.poll_next_unpin(task_context));

        let _context_guard = context.waker_forwarder.forward(task_context);
        future.poll(&context.application, &mut context.store)
    }
}

/// A wrapper type representing the response sent from a future actor to a poll request sent from a
/// [`PollSender`].
///
/// This helps to simplify some types used, avoiding some Clippy lints.
struct PollResponse<Output>(Poll<Result<Output, ExecutionError>>);

/// An abstraction over a [`Future`] running as an actor on a non-asynchronous thread.
///
/// This type implements [`Future`] and sends poll requests to the actor implementation. When the
/// actor finishes, it sends back the result to this type, which then returns it.
///
/// Poll requests may not be sent to the implementation if it would cause non-deterministic
/// execution (as controlled by the [`HostFutureQueue`]).
pub struct PollSender<Output> {
    host_future_queue: HostFutureQueue<'static>,
    poll_requester: std::sync::mpsc::Sender<oneshot::Sender<PollResponse<Output>>>,
    state: PollSenderState<Output>,
}

/// The internal state of the [`PollSender`] type.
#[derive(Debug)]
enum PollSenderState<Output> {
    /// Waiting to be polled.
    Queued,

    /// Waiting for response to the previous poll request.
    Polling(oneshot::Receiver<PollResponse<Output>>),

    /// Result received and returned.
    Finished,
}

impl<Output> PollSender<Output> {
    /// Creates a new [`PollSender`] using the provided [`HostFutureQueue`] to ensure deterministic
    /// polling.
    ///
    /// Returns the new [`PollSender`] together with the receiver endpoint of the poll requests.
    fn new(
        host_future_queue: HostFutureQueue<'static>,
    ) -> (
        Self,
        std::sync::mpsc::Receiver<oneshot::Sender<PollResponse<Output>>>,
    ) {
        let (poll_sender, poll_receiver) = std::sync::mpsc::channel();

        let this = PollSender {
            host_future_queue,
            poll_requester: poll_sender,
            state: PollSenderState::Queued,
        };

        (this, poll_receiver)
    }

    /// Sends a poll request if allowed by the [`HostFutureQueue`].
    fn poll_start(&mut self, context: &mut Context) -> Poll<()> {
        ready!(self.host_future_queue.poll_next_unpin(context));

        let (response_sender, response_receiver) = oneshot::channel();
        let _ = self.poll_requester.send(response_sender);
        self.state = PollSenderState::Polling(response_receiver);

        Poll::Ready(())
    }

    /// Checks if a response to the last previous request has been received.
    ///
    /// If a response has been received, the state is updated based on if the response is that the
    /// result is ready or that it's still pending.
    fn poll_response(
        &mut self,
        context: &mut Context,
    ) -> Option<Poll<Result<Output, ExecutionError>>> {
        let PollSenderState::Polling(receiver) = &mut self.state else {
            panic!("`poll_response` called without being in a `PollSenderState::Polling` state");
        };

        match receiver.poll_unpin(context) {
            Poll::Ready(Ok(PollResponse(Poll::Ready(response)))) => {
                self.state = PollSenderState::Finished;
                Some(Poll::Ready(response))
            }
            Poll::Ready(Ok(PollResponse(Poll::Pending))) => {
                self.state = PollSenderState::Queued;
                None
            }
            Poll::Ready(Err(oneshot::Canceled)) => {
                self.state = PollSenderState::Finished;
                Some(Poll::Ready(Err(WasmExecutionError::Aborted.into())))
            }
            Poll::Pending => Some(Poll::Pending),
        }
    }
}

impl<Output> Future for PollSender<Output> {
    type Output = Result<Output, ExecutionError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let mut this = self.as_mut();

        loop {
            match this.state {
                PollSenderState::Queued => ready!(this.poll_start(context)),
                PollSenderState::Polling(_) => {
                    if let Some(return_value) = this.poll_response(context) {
                        break return_value;
                    }
                }
                PollSenderState::Finished => panic!("Future polled after it has finished"),
            }
        }
    }
}

/// Interface to poll a future implemented in a Wasm module.
pub trait GuestFutureInterface<Application>
where
    Application: ApplicationRuntimeContext,
{
    /// The output of the guest future.
    type Output;

    /// Polls the guest future to attempt to progress it.
    ///
    /// May return an [`ExecutionError`] if the guest Wasm module panics, for example.
    fn poll(
        &self,
        application: &Application,
        store: &mut Application::Store,
    ) -> Poll<Result<Self::Output, ExecutionError>>;
}

/// A type to keep track of a [`Waker`] so that it can be forwarded to any async code called from
/// the guest Wasm module.
///
/// When a [`Future`] is polled, a [`Waker`] is used so that the task can be scheduled to be
/// woken up and polled again if it's still awaiting something.
///
/// The problem is that calling a Wasm module from an async task can lead to that guest code
/// calling back some host async code. A [`Context`] for the new host code must be created with the
/// same [`Waker`] to ensure that the wake events are forwarded back correctly to the host code
/// that called the guest.
///
/// Because the context has a lifetime and that forwarding lifetimes through the runtime calls is
/// not possible, this type erases the lifetime of the context and stores it in an `Arc<Mutex<_>>`
/// so that the context can be obtained again later. To ensure that this is safe, an
/// [`ActiveContextGuard`] instance is used to remove the context from memory before the lifetime
/// ends.
#[derive(Clone, Default)]
pub struct WakerForwarder(Arc<Mutex<Option<Waker>>>);

impl WakerForwarder {
    /// Forwards the waker from the task `context` into shared memory so that it can be obtained
    /// later.
    pub fn forward<'context>(&mut self, context: &mut Context) -> ActiveContextGuard<'context> {
        let mut waker_reference = self
            .0
            .try_lock()
            .expect("Unexpected concurrent task context access");

        assert!(
            waker_reference.is_none(),
            "`WakerForwarder` accessed by concurrent tasks"
        );

        *waker_reference = Some(context.waker().clone());

        ActiveContextGuard {
            waker: self.0.clone(),
            lifetime: PhantomData,
        }
    }

    /// Runs a `closure` with a [`Context`] using the forwarded waker.
    ///
    /// # Panics
    ///
    /// If no waker has been forwarded.
    pub fn with_context<Output>(&mut self, closure: impl FnOnce(&mut Context) -> Output) -> Output {
        let waker_reference = self
            .0
            .try_lock()
            .expect("Unexpected concurrent application call");

        let mut context = Context::from_waker(
            waker_reference
                .as_ref()
                .expect("Application called without an async task context"),
        );

        closure(&mut context)
    }
}

/// A guard type responsible for ensuring the [`Waker`] stored in shared memory does not outlive
/// the task [`Context`] it was obtained from.
pub struct ActiveContextGuard<'context> {
    waker: Arc<Mutex<Option<Waker>>>,
    lifetime: PhantomData<&'context mut ()>,
}

impl Drop for ActiveContextGuard<'_> {
    fn drop(&mut self) {
        let mut waker_reference = self
            .waker
            .try_lock()
            .expect("Unexpected concurrent task context access");

        *waker_reference = None;
    }
}
