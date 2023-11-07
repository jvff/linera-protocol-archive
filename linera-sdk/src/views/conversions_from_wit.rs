// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit-bindgen-guest-rust`] to types declared in
//! [`linera-sdk`].

use super::view_system_api::PollUnit;
use std::task::Poll;

impl From<PollUnit> for Poll<()> {
    fn from(poll_write_batch: PollUnit) -> Self {
        match poll_write_batch {
            PollUnit::Ready => Poll::Ready(()),
            PollUnit::Pending => Poll::Pending,
        }
    }
}
