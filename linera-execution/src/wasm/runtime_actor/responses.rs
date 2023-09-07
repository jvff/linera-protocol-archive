// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types related to sending responses from an actor after handling requests.

use std::{
    mem,
    sync::{Condvar, Mutex},
};

/// A synchronous channel for sending a response `T`.
///
/// This type should be placed in an [`Arc`][`std::sync::Arc`] and shared between the sender and
/// the receiver.
pub struct SyncResponse<T> {
    slot: Mutex<ResponseSlot<T>>,
    notifier: Condvar,
}

impl<T> Default for SyncResponse<T> {
    fn default() -> Self {
        SyncResponse {
            slot: Mutex::default(),
            notifier: Condvar::default(),
        }
    }
}

impl<T> SyncResponse<T>
where
    T: Send,
{
    /// Sends a response through this channel.
    ///
    /// # Panics
    ///
    /// If a response has already been previously sent.
    pub fn send(&self, value: T) {
        self.slot
            .lock()
            .expect("Failed to lock `SyncResponse` mutex")
            .insert(value);
        self.notifier.notify_one();
    }

    /// Blocks until a response is received.
    ///
    /// # Panics
    ///
    /// If more than one response was sent.
    pub fn wait(&self) -> T {
        let mut slot = self
            .slot
            .lock()
            .expect("Failed to lock `SyncResponse` mutex");

        loop {
            if let Some(value) = slot.take() {
                return value;
            }

            slot = self
                .notifier
                .wait(slot)
                .expect("Failed to lock `SyncResponse` mutex after receiving notification");
        }
    }
}

/// A helper type to keep track of the response channel state.
#[derive(Default)]
pub enum ResponseSlot<T> {
    /// No response has been sent.
    #[default]
    Empty,

    /// A response has been sent but not yet received.
    Ready(T),

    /// A response has been sent and consumed by the receiver.
    Finished,
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
            ResponseSlot::Ready(_) | ResponseSlot::Finished => {
                panic!("Attempt to send two responses for a request")
            }
        }
    }

    /// Attempts to retrieve a response from the slot.
    ///
    /// # Panics
    ///
    /// If a response has already been previously retrieved from this slot.
    pub fn take(&mut self) -> Option<T> {
        match mem::replace(self, ResponseSlot::Finished) {
            ResponseSlot::Ready(value) => Some(value),
            ResponseSlot::Empty => {
                *self = ResponseSlot::Empty;
                None
            }
            ResponseSlot::Finished => panic!("Attempt to take two responses for a request"),
        }
    }
}
