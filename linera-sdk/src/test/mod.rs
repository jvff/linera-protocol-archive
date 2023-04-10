// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for writing tests for WebAssembly applications.
//!
//! This module re-exports the types for either [`unit`] tests or [`integration`] tests.
//!
//! Unit tests are usually written in the `src` directory in the root of the crate's directory, and
//! are executed targeting the `wasm32-unknown-unknown` target. Integration tests are usually
//! written in the `tests` directory instead, and are executed targeting the host architecture.

#[cfg(not(target_arch = "wasm32"))]
mod integration;

#[cfg(not(target_arch = "wasm32"))]
pub use self::integration::*;
