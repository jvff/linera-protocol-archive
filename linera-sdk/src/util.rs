// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Module with helper types and functions used by the SDK.

use futures::task;
use std::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll},
};

/// Yields the current asynchronous task so that other tasks may progress if possible.
///
/// After other tasks progress, this task resumes as soon as possible. More explicitly, it is
/// scheduled to be woken up as soon as possible.
pub fn yield_once() -> YieldOnce {
    YieldOnce::default()
}

/// A [`Future`] that returns [`Poll::Pending`] once and immediately schedules itself to wake up.
#[derive(Default)]
pub struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let mut this = self.as_mut();

        if this.yielded {
            Poll::Ready(())
        } else {
            this.yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// An extension trait to block on a [`Future`] until it completes.
pub trait BlockingWait {
    /// The type returned by the [`Future`].
    type Output;

    /// Waits for the [`Future`] to complete in a blocking manner.
    ///
    /// Effectively polls the [`Future`] repeatedly until it returns [`Poll::Ready`].
    fn blocking_wait(self) -> Self::Output;
}

impl<AnyFuture> BlockingWait for AnyFuture
where
    AnyFuture: Future,
{
    type Output = AnyFuture::Output;

    fn blocking_wait(mut self) -> Self::Output {
        let waker = task::noop_waker();
        let mut task_context = Context::from_waker(&waker);
        let mut future = pin!(self);

        loop {
            match future.as_mut().poll(&mut task_context) {
                Poll::Pending => continue,
                Poll::Ready(output) => return output,
            }
        }
    }
}
