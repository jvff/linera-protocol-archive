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
use std::thread;
use std::{
    future::Future,
    marker::PhantomData,
    mem,
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

pub struct GuestFutureActor<Future, Application>
where
    Application: ApplicationRuntimeContext,
    Future: GuestFutureInterface<Application>,
{
    future: Future,
    context: WasmRuntimeContext<Application>,
    poll_requests: std::sync::mpsc::Receiver<()>,
    poll_completion: tokio::sync::mpsc::UnboundedSender<()>,
    result_sender: oneshot::Sender<Result<Future::Output, ExecutionError>>,
}

impl<Future, Application> GuestFutureActor<Future, Application>
where
    Application: ApplicationRuntimeContext + Send + 'static,
    Future: GuestFutureInterface<Application> + Send + 'static,
    Future::Output: Send,
{
    pub fn spawn(
        future: Future,
        context: WasmRuntimeContext<Application>,
    ) -> PollSender<Future::Output> {
        let (actor, poll_sender) = Self::new(future, context);

        thread::spawn(|| actor.run());

        poll_sender
    }

    pub fn new(
        future: Future,
        mut context: WasmRuntimeContext<Application>,
    ) -> (Self, PollSender<Future::Output>) {
        let (dummy_future_queue, _) = HostFutureQueue::new();
        let host_future_queue = mem::replace(&mut context.future_queue, dummy_future_queue);

        let (poll_start_sender, poll_start_receiver) = std::sync::mpsc::channel();
        let (poll_completed_sender, poll_completed_receiver) =
            tokio::sync::mpsc::unbounded_channel();
        let (result_sender, result_receiver) = oneshot::channel();

        let poll_sender = PollSender::new(
            host_future_queue,
            poll_start_sender,
            poll_completed_receiver,
            result_receiver,
        );

        let actor = GuestFutureActor {
            future,
            context,
            poll_requests: poll_start_receiver,
            poll_completion: poll_completed_sender,
            result_sender,
        };

        (actor, poll_sender)
    }

    pub fn run(mut self) {
        while let Ok(()) = self.poll_requests.recv() {
            match self
                .future
                .poll(&self.context.application, &mut self.context.store)
            {
                Poll::Pending => {
                    let _ = self.poll_completion.send(());
                }
                Poll::Ready(result) => {
                    let _ = self.result_sender.send(result);
                    break;
                }
            }
        }
    }
}

pub struct PollSender<Output> {
    host_future_queue: HostFutureQueue<'static>,
    poll_requester: std::sync::mpsc::Sender<()>,
    poll_completed: tokio::sync::mpsc::UnboundedReceiver<()>,
    result_receiver: oneshot::Receiver<Result<Output, ExecutionError>>,
    state: PollSenderState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PollSenderState {
    Queued,
    Polling,
    Finished,
}

impl<Output> PollSender<Output> {
    pub fn new(
        host_future_queue: HostFutureQueue<'static>,
        poll_start_sender: std::sync::mpsc::Sender<()>,
        poll_completed_receiver: tokio::sync::mpsc::UnboundedReceiver<()>,
        result_receiver: oneshot::Receiver<Result<Output, ExecutionError>>,
    ) -> Self {
        PollSender {
            host_future_queue,
            poll_requester: poll_start_sender,
            poll_completed: poll_completed_receiver,
            result_receiver,
            state: PollSenderState::Queued,
        }
    }

    fn poll_start(&mut self, context: &mut Context) -> Poll<()> {
        ready!(self.host_future_queue.poll_next_unpin(context));
        let _ = self.poll_requester.send(());
        self.state = PollSenderState::Polling;

        Poll::Ready(())
    }

    fn poll_response(&mut self, context: &mut Context) -> Poll<()> {
        ready!(self.poll_completed.poll_recv(context));
        self.state = PollSenderState::Queued;

        Poll::Ready(())
    }
}

impl<Output> Future for PollSender<Output> {
    type Output = Result<Output, ExecutionError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let mut this = self.as_mut();

        loop {
            if let Poll::Ready(result) = this.result_receiver.poll_unpin(context) {
                this.state = PollSenderState::Finished;
                break Poll::Ready(result.unwrap_or(Err(WasmExecutionError::Aborted.into())));
            }

            match this.state {
                PollSenderState::Queued => ready!(this.poll_start(context)),
                PollSenderState::Polling => ready!(this.poll_response(context)),
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
