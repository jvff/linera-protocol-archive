// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types to enforce determinism on asynchronous code called from a guest Wasm module.
//!
//! To ensure that asynchronous calls from a guest Wasm module are deterministic, the following
//! rules are enforced:
//!
//! - Futures are completed in the exact same order that they were created;
//! - The guest Wasm module is only polled when the next future to be completed has finished;
//! - Every time the guest Wasm module is polled, exactly one future will return [`Poll::Ready`];
//! - All other futures will return [`Poll::Pending`].
//!
//! To enforce these rules, the futures have to be polled separately from the guest Wasm module.
//! The traditional asynchronous behavior is for the host to poll the guest, and for the guest to
//! poll the host futures again. This is problematic because the number of times the host futures
//! need to be polled might not be deterministic. So even if the futures are made to finish
//! sequentially, the number of times the guest is polled would not be deterministic.
//!
//! For the guest to be polled separately from the host futures it calls, two types are used:
//! [`HostFutureQueue`] and [`QueuedHostFutureFactory`]. The [`QueuedHostFutureFactory`] is what is
//! used by the guest Wasm module handle to enqueue futures for deterministic execution (i.e.,
//! normally stored in the application's exported API handler). For every future that's enqueued, a
//! [`oneshot::Receiver`] is returned for the future's result. The future itself is actually sent
//! to the [`HostFutureQueue`] to be polled separately from the guest.
//!
//! The [`HostFutureQueue`] implements [`Stream`], and produces a marker `()` item every time the
//! next future in the queue is ready for completion. Therefore, the [`super::async_boundary::GuestFuture`] is responsible
//! for always polling the [`HostFutureQueue`] before polling the guest Wasm module.

use futures::{
    channel::oneshot,
    future::{self, BoxFuture, FutureExt},
    stream::{FuturesOrdered, Stream, StreamExt},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::mpsc;

/// A queue of host futures called by a Wasm guest module that finish in the same order they were
/// created.
///
/// Futures are added to the queue through the [`QueuedHostFutureFactory`] associated to the
/// [`HostFutureQueue`]. The futures are executed (polled) when the [`HostFutureQueue`] is polled
/// (as a [`Stream`]).
///
/// [`QueuedHostFutureFactory`] wraps the future before sending it to [`HostFutureQueue`] so that
/// it returns a closure that sends the future's output to the corresponding [`oneshot::Receiver`].
/// The [`HostFutureQueue`] runs that closure when it's time to complete the [`oneshot::Receiver`],
/// ensuring that only one future is completed after each item produced by the
/// [`HostFutureQueue`]'s implementation of [`Stream`].
pub struct HostFutureQueue<'futures> {
    new_futures: mpsc::UnboundedReceiver<BoxFuture<'futures, Box<dyn FnOnce() + Send>>>,
    queue: FuturesOrdered<BoxFuture<'futures, Box<dyn FnOnce() + Send>>>,
}

impl<'futures> HostFutureQueue<'futures> {
    /// Creates a new [`HostFutureQueue`] and its associated [`QueuedHostFutureFactory`].
    ///
    /// An initial empty future is added to the queue so that the first time the queue is polled it
    /// returns an item, allowing the guest Wasm module to be polled for the first time.
    pub fn new() -> (Self, QueuedHostFutureFactory<'futures>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let empty_completion: Box<dyn FnOnce() + Send> = Box::new(|| ());
        let initial_future = future::ready(empty_completion).boxed();

        (
            HostFutureQueue {
                new_futures: receiver,
                queue: FuturesOrdered::from_iter([initial_future]),
            },
            QueuedHostFutureFactory { sender },
        )
    }

    /// Polls the futures in the queue.
    ///
    /// Returns `true` if the next future in the queue has completed.
    ///
    /// If the next future has completed, its returned closure is executed in order to send the
    /// future's result to its associated [`oneshot::Receiver`].
    fn poll_futures(&mut self, context: &mut Context<'_>) -> bool {
        match self.queue.poll_next_unpin(context) {
            Poll::Ready(Some(future_completion)) => {
                future_completion();
                true
            }
            Poll::Ready(None) => false,
            Poll::Pending => false,
        }
    }

    /// Polls the [`mpsc::UnboundedReceiver`] of futures to add to the queue.
    ///
    /// Returns true if the [`mpsc::UnboundedSender`] endpoint has been closed.
    fn poll_incoming(&mut self, context: &mut Context<'_>) -> bool {
        loop {
            match self.new_futures.poll_recv(context) {
                Poll::Pending => break false,
                Poll::Ready(Some(new_future)) => self.queue.push_back(new_future),
                Poll::Ready(None) => break true,
            }
        }
    }
}

impl<'futures> Stream for HostFutureQueue<'futures> {
    type Item = ();

    /// Polls the [`HostFutureQueue`], producing a `()` item if a future was completed.
    ///
    /// First the incoming channel of futures is polled, in order to add any newly created futures
    /// to the queue. Then the futures are polled.
    ///
    /// # Note on [`Poll::Pending`]
    ///
    /// This function returns [`Poll::Pending`] correctly, because it's only returned if either:
    ///
    /// - No new futures were received (the [`mpsc::UnboundedReceiver`] returned [`Poll::Pending`])
    ///   and the queue is empty, which means that this task will receive a wakeup when a new future
    ///   is received;
    /// - No queued future was completed (the [`FuturesOrdered`] returned [`Poll::Pending`]), which
    ///   which means that all futures in the queue have scheduled wakeups for this task;
    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let incoming_closed = self.poll_incoming(context);

        if incoming_closed && self.queue.is_empty() {
            return Poll::Ready(None);
        }

        if self.poll_futures(context) {
            Poll::Ready(Some(()))
        } else {
            Poll::Pending
        }
    }
}

/// A factory of [`oneshot::Receiver`]s that enforces determinism of the host futures they
/// represent.
///
/// This type is created by [`HostFutureQueue::new`], and is associated to the [`HostFutureQueue`]
/// returned with it. Both must be used together in the correct manner as described by the module
/// documentation. The summary is that the [`HostFutureQueue`] should be polled until it returns an
/// item before the guest Wasm module is polled, so that the created [`oneshot::Receiver`]s are
/// only polled deterministically.
#[derive(Clone)]
pub struct QueuedHostFutureFactory<'futures> {
    sender: mpsc::UnboundedSender<BoxFuture<'futures, Box<dyn FnOnce() + Send>>>,
}

impl<'futures> QueuedHostFutureFactory<'futures> {
    /// Enqueues a `future` in the associated [`HostFutureQueue`].
    ///
    /// Returns a [`oneshot::Receiver`] that can be passed to the guest Wasm module, and that will
    /// only be ready when the inner `future` is ready and all previous futures added to the queue
    /// are ready.
    ///
    /// The `future` itself is only executed when the associated [`HostFutureQueue`] is polled.
    /// When the `future` is complete, the result is paired inside a closure with a
    /// [`oneshot::Sender`] that's connected to the returned [`oneshot::Receiver`]. The
    /// [`HostFutureQueue`] runs the closure when it's time to complete the [`oneshot::Receiver`].
    ///
    /// # Panics
    ///
    /// This function may panic if the corresponding [`HostFutureQueue`] has been dropped.
    pub fn enqueue<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'futures,
    ) -> oneshot::Receiver<Output>
    where
        Output: Send + 'static,
    {
        let (result_sender, result_receiver) = oneshot::channel();
        let future_sender = self.sender.clone();

        future_sender
            .send(
                future
                    .map(move |result| -> Box<dyn FnOnce() + Send> {
                        Box::new(move || {
                            // An error when sending the result indicates that the user
                            // application dropped the `oneshot::Receiver`, and no longer needs the
                            // result
                            let _ = result_sender.send(result);
                        })
                    })
                    .boxed(),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "`HostFutureQueue` should not be dropped while `QueuedHostFutureFactory` \
                    is still enqueuing futures",
                )
            });

        result_receiver
    }
}

#[cfg(test)]
mod tests {
    use super::HostFutureQueue;
    use futures::{
        channel::oneshot, future, stream::FuturesUnordered, task::noop_waker, FutureExt, StreamExt,
    };
    use std::{
        collections::VecDeque,
        mem,
        task::{Context, Poll},
        time::Duration,
    };
    use tokio::time;

    /// Tests if futures that finish in a non-sequential order complete sequentially.
    ///
    /// Starts some futures, with each one finishing in a different time without respecting the
    /// creation order, and checks that their respective futures complete in order.
    #[tokio::test]
    async fn futures_complete_in_order() {
        time::pause();

        let delays = [9, 4, 0, 7, 5, 6, 1, 3, 2, 8];
        let futures = delays
            .into_iter()
            .enumerate()
            .map(|(index, delay)| async move {
                time::sleep(Duration::from_secs(delay)).await;
                index
            });

        let (mut future_queue, mut queued_future_factory) = HostFutureQueue::new();

        // The queue should immediately produce the first item, to allow first poll of the guest
        assert_eq!(future_queue.next().now_or_never(), Some(Some(())));

        // Queue all the futures, and collect the returned `oneshot::Receiver`s so that they can be
        // polled together
        let mut queued_futures = futures
            .map(|future| queued_future_factory.enqueue(future))
            .collect::<FuturesUnordered<_>>();

        // None of the `oneshot::Receiver`s should complete before the queue allows them
        assert!(time::timeout(Duration::from_secs(4), queued_futures.next())
            .await
            .is_err());

        for expected_index in 0..delays.len() {
            // Wait until a future is ready
            assert_eq!(future_queue.next().await, Some(()));

            // The next completed future should respect its creation order
            assert_eq!(
                queued_futures.next().now_or_never(),
                Some(Some(Ok(expected_index)))
            );

            // No other future should be ready before the queue is polled again
            assert!(matches!(
                queued_futures.next().now_or_never(),
                None | Some(None)
            ));
        }
    }

    /// Tests if polling is deterministic through the rule that only one future is ready per item
    /// produced by the [`HostFutureQueue`].
    ///
    /// Creates some fake futures that complete after a certain number of poll attempts. These are
    /// then enqueued on the queue, and the returned [`oneshot::Receiver`]s are repeatedly polled
    /// to check that after the queue produces an item *only one* of the [`oneshot::Receiver`]s
    /// does not return [`Poll::Pending`] while when the queue isn't polled or doesn't return an
    /// item, *all* of the [`oneshot::Receiver`]s return [`Poll::Pending`].
    ///
    /// Since the test has tight control on when futures are polled (using
    /// [`FutureExt::now_or_never`]), this test is not `async`. However, a context must be
    /// forwarded to the futures, and a fake one is used for that. Even so, [`FuturesOrdered`] is
    /// optimized to only poll its queued futures when they received a wakeup event, so artifical
    /// wakeups must still be sent.
    #[test]
    fn only_one_future_is_ready_per_item_produced() {
        let poll_counts = [3, 0, 2, 9, 2];
        let futures = poll_counts
            .into_iter()
            .enumerate()
            .map(|(index, poll_threshold)| {
                let mut poll_count = 0;

                future::poll_fn(move |context| {
                    if poll_count == poll_threshold {
                        Poll::Ready(index)
                    } else {
                        poll_count += 1;
                        // `FuturesOrdered` uses a `FuturesUnordered` internally, which hijacks the
                        // waker, and only polls again if a wakeup was scheduled. So even though at
                        // the top level we're using a fake waker, we still need to schedule a
                        // wakeup for the hijacked waker.
                        context.waker().wake_by_ref();
                        Poll::Pending
                    }
                })
            });

        let (mut future_queue, mut queued_future_factory) = HostFutureQueue::new();

        // The queue should immediately produce the first item, to allow first poll of the guest
        assert_eq!(future_queue.next().now_or_never(), Some(Some(())));

        // Queue all the futures, and collect the returned `oneshot::Receiver`s so that they can be
        // polled together
        let mut queued_futures = futures
            .map(|future| queued_future_factory.enqueue(future))
            .collect();

        // None of the `oneshot::Receiver`s should complete before the queue allows them
        let host_future_results = mock_poll_host_futures(&mut queued_futures);
        assert!(host_future_results.is_empty());

        let mut expected_index = 0;

        // Close connection to the `HostFutureQueue` so that it knows no new futures will arrive
        mem::drop(queued_future_factory);

        loop {
            let next_future_is_ready = match future_queue.next().now_or_never() {
                None => false,
                Some(Some(())) => true,
                Some(None) => break,
            };

            let host_future_results = mock_poll_host_futures(&mut queued_futures);

            if next_future_is_ready {
                assert_eq!(host_future_results, &[expected_index]);
                expected_index += 1;
            } else {
                assert!(host_future_results.is_empty());
            }
        }
    }

    /// Polls the `host_futures` repeatedly, returning the `Output`s that were produced by the ones
    /// that complete.
    ///
    /// Completed futures are removed from the [`VecDeque`].
    fn mock_poll_host_futures<Output>(
        host_futures: &mut VecDeque<oneshot::Receiver<Output>>,
    ) -> Vec<Output> {
        const TIMES_TO_POLL: usize = 11;

        let fake_waker = noop_waker();
        let mut fake_context = Context::from_waker(&fake_waker);

        let mut outputs = Vec::new();

        for _ in 0..TIMES_TO_POLL {
            let mut index = 0;

            while index < host_futures.len() {
                if let Poll::Ready(result) = host_futures[index].poll_unpin(&mut fake_context) {
                    outputs.push(result.expect("Queued `HostFuture` cancelled"));
                    host_futures.remove(index);
                }

                index += 1;
            }
        }

        outputs
    }
}
