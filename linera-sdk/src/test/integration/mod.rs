// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for writing integration tests for WebAssembly applications.
//!
//! Integration tests are usually written in the `tests` directory in the root of the crate's
//! directory (i.e., beside the `src` directory). Linera application integration tests should be
//! executed targeting the host architecture, instead of targeting `wasm32-unknown-unknown` like
//! done for unit tests.

#![cfg(any(feature = "test", feature = "wasmer", feature = "wasmtime"))]

#[cfg(not(any(feature = "wasmer", feature = "wasmtime")))]
compile_error!(
    "Integration requests require either the `wasmer` or `wasmtime` feature to be enabled in \
    `linera-sdk`.\n \
    It is recommended to add the following lines to `Cargo.toml`:\n \
    \n \
    [target.'cfg(not(target_arch = \"wasm32\"))'.dev-dependencies]\n \
    linera-sdk = { version = \"*\", features = [\"test\", \"wasmer\"] }"
);

mod block;
mod chain;
mod mock_stubs;
mod validator;

pub use self::{block::BlockBuilder, chain::ActiveChain, mock_stubs::*, validator::TestValidator};
