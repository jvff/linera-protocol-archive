// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Stubs for the interface of the system API available to application services.
//!
//! This allows building the crate for non-Wasm targets.

use linera_base::identifiers::ApplicationId;
use std::fmt;

const MESSAGE: &str = "Attempt to call a contract system API when not running as a Wasm guest";

/// Retrieves the current application parameters.
pub fn current_application_parameters() -> Vec<u8> {
    panic!("{MESSAGE}");
}

/// Loads the application state, without locking it for writes.
pub async fn load<State>() -> State {
    panic!("{MESSAGE}");
}

/// Loads the application state (and locks it for writes).
pub async fn lock_and_load_view<State>() -> State {
    panic!("{MESSAGE}");
}

/// Unlocks the service state previously loaded.
pub async fn unlock_view() {
    panic!("{MESSAGE}");
}

/// Requests the host to log a message.
///
/// Useful for debugging locally, but may be ignored by validators.
pub fn log(_message: &fmt::Arguments<'_>, _level: log::Level) {
    panic!("{MESSAGE}");
}

/// Queries another application.
pub async fn query_application(
    _application: ApplicationId,
    _argument: &[u8],
) -> Result<Vec<u8>, String> {
    panic!("{MESSAGE}");
}
