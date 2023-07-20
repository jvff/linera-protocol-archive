// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Code to interface with different runtimes.

mod error;
mod memory;
mod traits;

pub use self::{
    error::RuntimeError,
    memory::Memory,
    traits::{Instance, InstanceWithFunction, InstanceWithMemory, Runtime},
};
