// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for implementations of the custom traits for existing types.

use crate::{FakeInstance, InstanceWithMemory, Layout, WitLoad, WitStore};
use frunk::hlist;
use std::fmt::Debug;

/// Test roundtrip of `None::<i8>`.
#[test]
fn none() {
    let input = None::<i8>;

    test_memory_roundtrip(input, &[0_u8, 0_u8]);
    test_flattening_roundtrip(input, hlist![0_i32, 0_i32]);
}

/// Test roundtrip of `Some::<i8>`.
#[test]
fn some_byte() {
    let input = Some(-100_i8);

    test_memory_roundtrip(input, &[1_u8, 0x9c_u8]);
    test_flattening_roundtrip(input, hlist![1_i32, -100_i32]);
}

/// Test roundtrip of `Ok::<i16, u128>`.
#[test]
fn ok_two_bytes() {
    let input = Ok::<_, u128>(0x1234_i16);

    assert_eq!(
        <<Result<i16, u128> as crate::WitType>::Layout as Layout>::ALIGNMENT,
        8
    );
    test_memory_roundtrip(
        input,
        &[
            0_u8, 0, 0, 0, 0, 0, 0, 0, 0x34_u8, 0x12_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
}

/// Test storing an instance of `T` to memory, checking that the `memory_data` bytes are correctly
/// written, and check that the instance can be loaded from those bytes.
fn test_memory_roundtrip<T>(input: T, memory_data: &[u8])
where
    T: Debug + Eq + WitLoad + WitStore,
{
    let mut instance = FakeInstance::default();
    let mut memory = instance.memory().unwrap();
    let length = memory_data.len() as u32;

    let address = memory.allocate(length).unwrap();

    input.store(&mut memory, address).unwrap();

    assert_eq!(memory.read(address, length).unwrap(), memory_data);
    assert_eq!(T::load(&memory, address).unwrap(), input);
}

/// Test lowering an instance of `T`, checking that the resulting flat layout matches the expected
/// `flat_layout`, and check that the instance can be lifted from that flat layout.
fn test_flattening_roundtrip<T>(input: T, flat_layout: <T::Layout as Layout>::Flat)
where
    T: Debug + Eq + WitLoad + WitStore,
    <T::Layout as Layout>::Flat: Debug + Eq,
{
    let mut instance = FakeInstance::default();
    let mut memory = instance.memory().unwrap();

    let lowered_layout = input.lower(&mut memory).unwrap();

    assert_eq!(lowered_layout, flat_layout);
    assert_eq!(T::lift_from(lowered_layout, &memory).unwrap(), input);
}
