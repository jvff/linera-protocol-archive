// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Traits used to allow complex types to be sent and received between hosts and guests using WIT.

mod implementations;

use crate::{
    GuestPointer, InstanceWithMemory, Layout, Memory, Runtime, RuntimeError, RuntimeMemory,
};

/// A type that is representable by fundamental WIT types.
pub trait WitType: Sized {
    /// The size of the type when laid out in memory.
    const SIZE: u32;

    /// The layout of the type as fundamental types.
    type Layout: Layout;
}

/// A type that can be loaded from a guest Wasm module.
pub trait WitLoad: WitType {
    /// Loads an instance of the type from the `location` in the guest's `memory`.
    fn load<Instance>(
        memory: &Memory<'_, Instance>,
        location: GuestPointer,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;

    /// Lifts an instance of the type from the `flat_layout` representation.
    ///
    /// May read from the `memory` if the type has references to heap data.
    fn lift_from<Instance>(
        flat_layout: <Self::Layout as Layout>::Flat,
        memory: &Memory<'_, Instance>,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>;
}
