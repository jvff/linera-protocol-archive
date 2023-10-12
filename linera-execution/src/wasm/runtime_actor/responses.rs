// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types related to sending responses from an actor after handling requests.

use futures::channel::oneshot;
use std::{
    mem,
    sync::{Arc, Condvar, Mutex},
};
use thiserror::Error;

/// The sender endpoint for a synchronous single-message channel.
pub struct SyncResponseSender<T: Send>(Arc<SyncResponse<T>>);

impl<T> SyncResponseSender<T>
where
    T: Send,
{
    /// Sends a response through this channel.
    ///
    /// # Panics
    ///
    /// If a response has already been previously sent.
    pub fn send(&self, value: T) {
        let inner = self.0.as_ref();

        inner
            .slot
            .lock()
            .expect("Failed to lock `SyncResponse` mutex")
            .insert(value);
        inner.notifier.notify_one();
    }
}

impl<T> Drop for SyncResponseSender<T>
where
    T: Send,
{
    fn drop(&mut self) {
        let inner = self.0.as_ref();

        inner
            .slot
            .lock()
            .expect("Failed to lock `SyncResponse` mutex")
            .close();
        inner.notifier.notify_one();
    }
}

/// The receiver endpoint for a synchronous single-message channel.
pub struct SyncResponseReceiver<T>(Arc<SyncResponse<T>>);

impl<T> SyncResponseReceiver<T>
where
    T: Send,
{
    /// Blocks until a response is received.
    ///
    /// Returns [`CanceledError`] if the sender endpoint was dropped without sending a message.
    ///
    /// # Panics
    ///
    /// If more than one response was sent.
    pub fn wait(&self) -> Result<T, CanceledError> {
        let inner = self.0.as_ref();

        let mut slot = inner
            .slot
            .lock()
            .expect("Failed to lock `SyncResponse` mutex");

        loop {
            if let Some(value) = slot.take()? {
                return Ok(value);
            }

            slot = inner
                .notifier
                .wait(slot)
                .expect("Failed to lock `SyncResponse` mutex after receiving notification");
        }
    }
}

/// A synchronous channel for sending a response `T`.
///
/// This type should be placed in an [`Arc`][`std::sync::Arc`] and shared between the sender and
/// the receiver.
pub struct SyncResponse<T> {
    slot: Mutex<ResponseSlot<T>>,
    notifier: Condvar,
}

impl<T> SyncResponse<T>
where
    T: Send,
{
    /// Creates a new synchronous single-message channel.
    ///
    /// Returns the sender and the receiver endpoints.
    pub fn channel() -> (SyncResponseSender<T>, SyncResponseReceiver<T>) {
        let inner = Arc::new(SyncResponse {
            slot: Mutex::default(),
            notifier: Condvar::default(),
        });

        let sender = SyncResponseSender(inner.clone());
        let receiver = SyncResponseReceiver(inner);

        (sender, receiver)
    }
}

/// A helper type to keep track of the response channel state.
#[derive(Default)]
enum ResponseSlot<T> {
    /// No response has been sent.
    #[default]
    Empty,

    /// A response has been sent but not yet received.
    Ready(T),

    /// A response has been sent and consumed by the receiver.
    Finished,

    /// A response has not been sent and the sender was dropped.
    Canceled,
}

impl<T> ResponseSlot<T> {
    /// Inserts a response in the slot, changing the state to indicate that a response has been
    /// sent.
    ///
    /// # Panics
    ///
    /// If a response has already been previously inserted into this slot.
    pub fn insert(&mut self, value: T) {
        match self {
            ResponseSlot::Empty => *self = ResponseSlot::Ready(value),
            ResponseSlot::Ready(_) | ResponseSlot::Finished | ResponseSlot::Canceled => {
                panic!("Attempt to send two responses for a request")
            }
        }
    }

    /// Closes the slot, marking it as `Canceled` if no response has been sent.
    pub fn close(&mut self) {
        if matches!(self, ResponseSlot::Empty) {
            *self = ResponseSlot::Canceled;
        }
    }

    /// Attempts to retrieve a response from the slot.
    ///
    /// # Panics
    ///
    /// If a response has already been previously retrieved from this slot.
    pub fn take(&mut self) -> Result<Option<T>, CanceledError> {
        match mem::replace(self, ResponseSlot::Finished) {
            ResponseSlot::Ready(value) => Ok(Some(value)),
            ResponseSlot::Empty => {
                *self = ResponseSlot::Empty;
                Ok(None)
            }
            ResponseSlot::Canceled => Err(CanceledError),
            ResponseSlot::Finished => panic!("Attempt to take two responses for a request"),
        }
    }
}

/// A channel to send a response either synchronously or asynchronously.
pub enum SyncOrAsyncResponse<T: Send> {
    Synchronous(SyncResponseSender<T>),
    Asynchronous(oneshot::Sender<T>),
}

impl<T> From<SyncResponseSender<T>> for SyncOrAsyncResponse<T>
where
    T: Send,
{
    fn from(sync_response: SyncResponseSender<T>) -> Self {
        SyncOrAsyncResponse::Synchronous(sync_response)
    }
}

impl<T> From<oneshot::Sender<T>> for SyncOrAsyncResponse<T>
where
    T: Send,
{
    fn from(async_response: oneshot::Sender<T>) -> Self {
        SyncOrAsyncResponse::Asynchronous(async_response)
    }
}

impl<T> SyncOrAsyncResponse<T>
where
    T: Send,
{
    /// Sends a response `value` through this channel.
    pub fn send(self, value: T) {
        match self {
            SyncOrAsyncResponse::Synchronous(sender) => sender.send(value),
            SyncOrAsyncResponse::Asynchronous(sender) => {
                let _ = sender.send(value);
            }
        }
    }
}

/// Error indicating that the sender endpoint was closed without sending a message.
#[derive(Clone, Copy, Debug, Error)]
#[error("Sender was dropped without sending anything")]
pub struct CanceledError;
