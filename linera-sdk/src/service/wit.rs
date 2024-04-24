// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal module with code generated by [`wit-bindgen`](https://github.com/jvff/wit-bindgen).

#![allow(missing_docs)]

// Export the service interface.
wit_bindgen::generate!({
    world: "service",
});

pub use self::linera::app::{service_system_api, view_system_api};
use super::__service_handle_query;

/// Implementation of the service WIT entrypoints.
pub struct ServiceEntrypoints;

impl self::exports::linera::app::service_entrypoints::Guest for ServiceEntrypoints {
    fn handle_query(argument: Vec<u8>) -> Result<Vec<u8>, String> {
        unsafe { __service_handle_query(argument) }
    }
}

export!(ServiceEntrypoints);
