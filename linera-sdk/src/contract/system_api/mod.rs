// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functions and types to interface with the system API available to application contracts.

#[cfg(not(any(test, feature = "test")))]
pub(crate) mod private;
#[cfg(any(test, feature = "test"))]
pub mod private;
mod wit;

pub(crate) use self::private::{
    call_application_without_persisting_state, call_session_without_persisting_state,
    current_application_parameters, load_and_lock, load_and_lock_view, store_and_unlock,
    store_and_unlock_view,
};
use linera_base::{
    data_types::{Amount, Timestamp},
    identifiers::{ApplicationId, ChainId},
};
use std::fmt;

/// Retrieves the current chain ID.
pub fn current_chain_id() -> ChainId {
    wit::get_chain_id()
}

/// Retrieves the current application ID.
pub fn current_application_id() -> ApplicationId {
    wit::get_application_id()
}

/// Retrieves the current system balance.
pub fn current_system_balance() -> Amount {
    wit::read_system_balance()
}

/// Retrieves the current system time, i.e. the timestamp of the block in which this is called.
pub fn current_system_time() -> Timestamp {
    wit::read_system_timestamp()
}

/// Requests the host to log a message.
///
/// Useful for debugging locally, but may be ignored by validators.
pub fn log(message: &fmt::Arguments<'_>, level: log::Level) {
    wit::log(message.to_string(), level.into());
}
