// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A dummy runtime implementation useful for tests.
//!
//! No WebAssembly bytecode can be executed, but it allows calling the canonical ABI functions
//! related to memory allocation.

use super::Runtime;
use std::sync::{Arc, Mutex};

/// A fake Wasm runtime.
pub struct FakeRuntime;

impl Runtime for FakeRuntime {
    type Export = ();
    type Memory = Arc<Mutex<Vec<u8>>>;
}
