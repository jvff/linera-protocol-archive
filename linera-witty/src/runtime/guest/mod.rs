// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for the using Witty inside a guest module.

mod allocation;
mod export;
mod function;

use self::{
    allocation::{cabi_free, cabi_realloc},
    export::Export,
};
use super::{GuestPointer, Instance, InstanceWithMemory, Runtime, RuntimeError, RuntimeMemory};
use std::{any::TypeId, borrow::Cow, slice};

/// Representation of the local guest as a runtime and instance.
#[derive(Clone, Copy, Debug, Default)]
pub struct Guest {
    user_data: (),
}

impl Runtime for Guest {
    type Export = Export;
    type Memory = ();
}

impl Instance for Guest {
    type Runtime = Guest;
    type UserData = ();

    type UserDataReference<'a> = &'a ()
    where
        Self::UserData: 'a,
        Self: 'a;

    type UserDataMutReference<'a> = &'a mut ()
    where
        Self::UserData: 'a,
        Self: 'a;

    fn load_export(&mut self, name: &str) -> Option<<Self::Runtime as Runtime>::Export> {
        match name {
            "memory" => Some(Export::Memory),
            "cabi_realloc" => Some(Export::Function {
                pointer: cabi_realloc as *mut (),
                signature: TypeId::of::<fn(i32, i32, i32, i32) -> i32>(),
            }),
            "cabi_free" => Some(Export::Function {
                pointer: cabi_free as *mut (),
                signature: TypeId::of::<fn(i32)>(),
            }),
            _ => {
                // Guests can't request for exports from the host during runtime.
                None
            }
        }
    }

    fn user_data(&self) -> Self::UserDataReference<'_> {
        &self.user_data
    }

    fn user_data_mut(&mut self) -> Self::UserDataMutReference<'_> {
        &mut self.user_data
    }
}

impl InstanceWithMemory for Guest {
    fn memory_from_export(
        &self,
        export: <Self::Runtime as Runtime>::Export,
    ) -> Result<Option<<Self::Runtime as Runtime>::Memory>, RuntimeError> {
        match export {
            Export::Memory => Ok(Some(())),
            _ => Err(RuntimeError::NotMemory),
        }
    }
}

impl RuntimeMemory<Guest> for () {
    fn read<'instance>(
        &self,
        _: &'instance Guest,
        location: GuestPointer,
        length: u32,
    ) -> Result<Cow<'instance, [u8]>, RuntimeError> {
        let data = unsafe { slice::from_raw_parts(location.0 as *const u8, length as usize) };

        Ok(Cow::Borrowed(data))
    }

    fn write(
        &mut self,
        _: &mut Guest,
        location: GuestPointer,
        bytes: &[u8],
    ) -> Result<(), RuntimeError> {
        let destination = unsafe { slice::from_raw_parts_mut(location.0 as *mut u8, bytes.len()) };

        destination.copy_from_slice(bytes);

        Ok(())
    }
}
