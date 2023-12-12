// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Stubs for the interface of the system API available to application contracts.
//!
//! This allows building the crate for non-Wasm targets.

use linera_base::{
    data_types::Timestamp,
    identifiers::{ApplicationId, ChainId, SessionId},
};
use std::fmt;

const MESSAGE: &str = "Attempt to call a contract system API when not running as a Wasm guest";

/// Retrieves the current chain ID.
pub fn current_chain_id() -> ChainId {
    panic!("{MESSAGE}");
}

/// Retrieves the current application ID.
pub fn current_application_id() -> ApplicationId {
    panic!("{MESSAGE}");
}

/// Retrieves the current application parameters.
pub fn current_application_parameters() -> Vec<u8> {
    panic!("{MESSAGE}");
}

/// Retrieves the current system time, i.e. the timestamp of the block in which this is called.
pub fn current_system_time() -> Timestamp {
    panic!("{MESSAGE}");
}

/// Loads the application state and locks it for writes.
pub fn load_and_lock<State>() -> Option<State> {
    panic!("{MESSAGE}");
}

/// Saves the application state and unlocks it.
pub async fn store_and_unlock<State>(_state: State) {
    panic!("{MESSAGE}");
}

/// Loads the application state and locks it for writes.
pub async fn load_and_lock_view<State>() -> State {
    panic!("{MESSAGE}");
}

/// Saves the application state and unlocks it.
pub async fn store_and_unlock_view<State>(_state: State) {
    panic!("{MESSAGE}");
}

/// Requests the host to log a message.
///
/// Useful for debugging locally, but may be ignored by validators.
pub fn log(_message: &fmt::Arguments<'_>, _level: log::Level) {
    panic!("{MESSAGE}");
}

/// Calls another application without persisting the current application's state.
///
/// Use the `call_application` method generated by the [`linera-sdk::contract`] macro in order to
/// guarantee the state is up-to-date in reentrant calls.
pub fn call_application_without_persisting_state(
    _authenticated: bool,
    _application: ApplicationId,
    _argument: &[u8],
    _forwarded_sessions: Vec<SessionId>,
) -> (Vec<u8>, Vec<SessionId>) {
    panic!("{MESSAGE}");
}

/// Calls another application's session without persisting the current application's state.
///
/// Use the `call_session` method generated by the [`linera-sdk::contract`] macro in order to
/// guarantee the state is up-to-date in reentrant calls.
pub fn call_session_without_persisting_state(
    _authenticated: bool,
    _session: SessionId,
    _argument: &[u8],
    _forwarded_sessions: Vec<SessionId>,
) -> (Vec<u8>, Vec<SessionId>) {
    panic!("{MESSAGE}");
}
