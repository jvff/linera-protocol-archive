// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! An extension trait to allow determining at compile time how tasks are spawned on the Tokio
//! runtime.
//!
//! In most cases the [`Future`] task to be spawned should implement [`Send`], but that's
//! not possible when compiling for `wasm32-unknown-unknown`. In that case, the task is
//! spawned in a [`LocalSet`][`tokio::tast::LocalSet`].

use std::{future::Future, pin::pin};

use tokio::task::{AbortHandle, JoinSet};

/// An extension trait for the [`JoinSet`] type.
#[cfg(not(target_arch = "wasm32"))]
pub trait JoinSetExt: Sized {
    /// Spawns a `future` task on this [`JoinSet`] using [`JoinSet::spawn`].
    fn spawn_task(&mut self, future: impl Future<Output = ()> + Send + 'static) -> AbortHandle;

    /// Blocks on the provided `future`, driving it and existing tasks in this [`JoinSet`]
    /// until the `future` finishes.
    ///
    /// # Note
    ///
    /// The `future` *must* be cancel safe, because it is driven inside a
    /// [`tokio::select`] expression.
    fn block_on<F>(self, future: F) -> impl Future<Output = F::Output> + Send
    where
        F: Future + Send,
        F::Output: Send;
}

/// An extension trait for the [`JoinSet`] type.
#[cfg(target_arch = "wasm32")]
pub trait JoinSetExt: Sized {
    /// Spawns a `future` task on this [`JoinSet`] using [`JoinSet::spawn_local`].
    fn spawn_task(&mut self, future: impl Future<Output = ()> + 'static) -> AbortHandle;

    /// Blocks on the provided `future`, driving it and existing tasks in this [`JoinSet`]
    /// until the `future` finishes.
    ///
    /// # Note
    ///
    /// The `future` *must* be cancel safe, because it is driven inside a
    /// [`tokio::select`] expression.
    fn block_on<F: Future>(self, future: F) -> impl Future<Output = F::Output>;
}

#[cfg(not(target_arch = "wasm32"))]
impl JoinSetExt for JoinSet<()> {
    fn spawn_task(&mut self, future: impl Future<Output = ()> + Send + 'static) -> AbortHandle {
        self.spawn(future)
    }

    async fn block_on<F>(mut self, future: F) -> F::Output
    where
        F: Future + Send,
        F::Output: Send,
    {
        let mut future = pin!(future);
        loop {
            tokio::select! {
              _ = self.join_next() => (),
              output = &mut future => return output,
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl JoinSetExt for JoinSet<()> {
    fn spawn_task(&mut self, future: impl Future<Output = ()> + 'static) -> AbortHandle {
        self.spawn_local(future)
    }

    async fn block_on<F: Future>(mut self, future: F) -> F::Output {
        let mut future = pin!(future);
        loop {
            tokio::select! {
              _ = self.join_next() => (),
              output = &mut future => return output,
            }
        }
    }
}
