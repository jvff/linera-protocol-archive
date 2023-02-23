// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types to enforce determinism on asynchronous code called from a guest WASM module.
//!
//! To ensure that asynchronous calls from a guest WASM module are deterministic, the following
//! rules are enforced:
//!
//! - Futures are completed in the exact same order that they were created;
//! - The guest WASM module is only polled when the next future to be completed has finished;
//! - Every time the guest WASM module is polled, exactly one future will return [`Poll::Ready`];
//! - All other futures will return [`Poll::Pending`].
//!
//! To enforce these rules, the futures have to be polled separately from the guest WASM module.
//! The traditional asynchronous behavior is for the host to poll the guest, and for the guest to
//! poll the host futures again. This is problematic because the amount of times the host futures
//! need to be polled might not be deterministic. So even if the futures are made to finish
//! sequentially, the amount of times the guest is polled would not be deterministic.
//!
//! For the guest to be polled separately from the host futures it calls, two types are used:
//! [`HostFutureQueue`] and [`QueuedHostFutureFactory`]. The [`QueuedHostFutureFactory`] is what is
//! used by the guest WASM module handle to enqueue futures for deterministic execution (i.e.,
//! normally stored in the application's exported API handler). For every future that's enqueued, a
//! [`HostFuture`] is returned that contains only a [`oneshot::Receiver`] for the future's result.
//! The future itself is actually sent to the [`HostFutureQueue`] to be polled separately from the
//! guest.
//!
//! The [`HostFutureQueue`] implements [`Stream`], and produces a marker `()` item every time the
//! next future in the queue is ready for completion. Therefore, the [`GuestFuture`] is responsible
//! for always polling the [`HostFutureQueue`] before polling the guest WASM module.

use super::async_boundary::HostFuture;
use futures::{
    channel::{mpsc, oneshot},
    future::{BoxFuture, FutureExt},
    sink::SinkExt,
};
use std::future::Future;

/// A factory of [`HostFuture`]s that enforces determinism of the host futures they represent.
///
/// This type is created by [`HostFutureQueue::new`], and is associated to the [`HostFutureQueue`]
/// returned with it. Both must be used together in the correct manner as described by the module
/// documentation. The summary is that the [`HostFutureQueue`] should be polled until it returns an
/// item before the guest WASM module is polled, so that the created [`HostFuture`]s are only polled
/// deterministically.
#[derive(Clone)]
pub struct QueuedHostFutureFactory<'futures> {
    sender: mpsc::Sender<BoxFuture<'futures, Box<dyn FnOnce() + Send>>>,
}

impl<'futures> QueuedHostFutureFactory<'futures> {
    /// Enqueues a `future` in the associated [`HostFutureQueue`].
    ///
    /// Returns a [`HostFuture`] that can be passed to the guest WASM module, and that will only be
    /// ready when the inner `future` is ready and all previous futures added to the queue are
    /// ready.
    ///
    /// The `future` itself is only executed when the associated [`HostFutureQueue`] is polled.
    /// When the `future` is complete, the result is paired inside a closure with a
    /// [`oneshot::Sender`] that's connected to the [`oneshot::Receiver`] inside the returned
    /// [`HostFuture`]. The [`HostFutureQueue`] runs the closure when it's time to complete the
    /// [`HostFuture`].
    pub fn enqueue<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'futures,
    ) -> HostFuture<'futures, Output>
    where
        Output: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let mut future_sender = self.sender.clone();

        HostFuture::new(async move {
            let _ = future_sender
                .send(
                    future
                        .map(move |result| -> Box<dyn FnOnce() + Send> {
                            Box::new(move || {
                                let _ = result_sender.send(result);
                            })
                        })
                        .boxed(),
                )
                .await;

            result_receiver.await.expect("Host future cancelled")
        })
    }
}
