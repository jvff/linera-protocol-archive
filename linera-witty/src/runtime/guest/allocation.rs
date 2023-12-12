// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementation of the Canonical ABI allocation functions.

use std::{alloc, slice};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocationMetadata {
    size: i32,
    alignment: i32,
}

impl AllocationMetadata {
    pub const fn size() -> usize {
        8
    }

    pub fn new(size: i32, alignment: i32) -> Self {
        AllocationMetadata { size, alignment }
    }

    pub fn read_from(address: *const u8) -> Self {
        let metadata = unsafe { slice::from_raw_parts(address, Self::size()) };

        let size_bytes = metadata[0..4].try_into().expect("Incorrect array indices");
        let alignment_bytes = metadata[4..8].try_into().expect("Incorrect array indices");

        let size = i32::from_le_bytes(size_bytes);
        let alignment = i32::from_le_bytes(alignment_bytes);

        AllocationMetadata { size, alignment }
    }

    pub fn write_to(&self, address: *mut u8) {
        let metadata = unsafe { slice::from_raw_parts_mut(address, Self::size()) };

        metadata[0..4].copy_from_slice(&self.size.to_le_bytes());
        metadata[4..8].copy_from_slice(&self.alignment.to_le_bytes());
    }
}

impl From<alloc::Layout> for AllocationMetadata {
    fn from(layout: alloc::Layout) -> Self {
        AllocationMetadata {
            size: layout.size() as i32,
            alignment: layout.align() as i32,
        }
    }
}

impl From<AllocationMetadata> for alloc::Layout {
    fn from(metadata: AllocationMetadata) -> Self {
        alloc::Layout::from_size_align(metadata.size as usize, metadata.alignment as usize)
            .expect("Invalid layout")
    }
}

#[no_mangle]
pub extern "C" fn cabi_realloc(
    old_address: i32,
    old_size: i32,
    alignment: i32,
    new_size: i32,
) -> i32 {
    let Some(new_size) = new_size.checked_add(AllocationMetadata::size() as i32) else {
        return -1;
    };

    if old_address == 0 {
        assert_eq!(old_size, 0);

        let metadata = AllocationMetadata::new(new_size, alignment);
        let new_address = unsafe { alloc::alloc(metadata.into()) };

        metadata.write_to(new_address);

        unsafe { new_address.add(AllocationMetadata::size()) as i32 }
    } else {
        let metadata_address = (old_address - AllocationMetadata::size() as i32) as *mut u8;
        let mut metadata = AllocationMetadata::read_from(metadata_address);

        assert_eq!(old_size + AllocationMetadata::size() as i32, metadata.size);

        if alignment == metadata.alignment {
            let new_address =
                unsafe { alloc::realloc(metadata_address, metadata.into(), new_size as usize) };

            metadata.size = new_size;
            metadata.write_to(new_address);

            unsafe { new_address.add(AllocationMetadata::size()) as i32 }
        } else {
            cabi_free(old_address);
            cabi_realloc(0, 0, alignment, new_size)
        }
    }
}

#[no_mangle]
pub extern "C" fn cabi_free(address: i32) {
    let metadata_address = (address - AllocationMetadata::size() as i32) as *mut u8;
    let metadata = AllocationMetadata::read_from(metadata_address);

    unsafe { alloc::dealloc(metadata_address, metadata.into()) };
}
