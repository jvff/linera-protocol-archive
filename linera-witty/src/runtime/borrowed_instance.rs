// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of Wasm instance related traits to mutable borrows of instances.
//!
//! This allows using the same traits without having to move the type implementation around, for
//! example as parameters in reentrant functions.

use super::{
    traits::{CabiFreeAlias, CabiReallocAlias},
    Instance, InstanceWithFunction, InstanceWithMemory, Runtime, RuntimeError, RuntimeMemory,
};
use crate::{memory_layout::FlatLayout, GuestPointer};
use std::borrow::Cow;

impl<AnyInstance> Instance for &mut AnyInstance
where
    AnyInstance: Instance,
{
    type Runtime = AnyInstance::Runtime;

    fn load_export(&mut self, name: &str) -> Option<<Self::Runtime as Runtime>::Export> {
        AnyInstance::load_export(*self, name)
    }
}

impl<Parameters, Results, AnyInstance> InstanceWithFunction<Parameters, Results>
    for &mut AnyInstance
where
    AnyInstance: InstanceWithFunction<Parameters, Results>,
    Parameters: FlatLayout,
    Results: FlatLayout,
{
    type Function = AnyInstance::Function;

    fn function_from_export(
        &mut self,
        export: <Self::Runtime as Runtime>::Export,
    ) -> Result<Option<Self::Function>, RuntimeError> {
        AnyInstance::function_from_export(*self, export)
    }

    fn call(
        &mut self,
        function: &Self::Function,
        parameters: Parameters,
    ) -> Result<Results, RuntimeError> {
        AnyInstance::call(*self, function, parameters)
    }
}

impl<'a, AnyInstance> InstanceWithMemory for &'a mut AnyInstance
where
    AnyInstance: InstanceWithMemory,
    &'a mut AnyInstance:
        Instance<Runtime = AnyInstance::Runtime> + CabiReallocAlias + CabiFreeAlias,
{
    fn memory_from_export(
        &self,
        export: <Self::Runtime as Runtime>::Export,
    ) -> Result<Option<<Self::Runtime as Runtime>::Memory>, RuntimeError> {
        AnyInstance::memory_from_export(&**self, export)
    }
}

impl<AnyRuntimeMemory, Instance> RuntimeMemory<&mut Instance> for AnyRuntimeMemory
where
    AnyRuntimeMemory: RuntimeMemory<Instance>,
{
    /// Reads `length` bytes from memory from the provided `location`.
    fn read<'instance>(
        &self,
        instance: &'instance &mut Instance,
        location: GuestPointer,
        length: u32,
    ) -> Result<Cow<'instance, [u8]>, RuntimeError> {
        self.read(&**instance, location, length)
    }

    /// Writes the `bytes` to memory at the provided `location`.
    fn write(
        &mut self,
        instance: &mut &mut Instance,
        location: GuestPointer,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        self.write(&mut **instance, location, bytes)
    }
}
