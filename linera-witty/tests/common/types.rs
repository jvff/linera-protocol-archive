// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Dummy types used in tests.

use linera_witty::{WitLoad, WitType};

/// A type that wraps a simple type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct SimpleWrapper(pub bool);

/// A tuple struct that doesn't need any internal padding in its memory layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct TupleWithoutPadding(pub u64, pub i32, pub i16);

/// A tuple struct that requires internal padding in its memory layout between all of its fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct TupleWithPadding(pub u16, pub u32, pub i64);

/// A struct with named fields that requires padding in two locations in its memory layout.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct RecordWithDoublePadding {
    pub first: u16,
    pub second: u32,
    pub third: i8,
    pub fourth: i64,
}

/// A simple struct with named fields to be used inside a more complex struct.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct Leaf {
    pub first: bool,
    pub second: u128,
}

/// A struct that contains fields with custom types that also have derived trait implementations.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, WitType, WitLoad)]
pub struct Branch {
    pub tag: u16,
    pub first_leaf: Leaf,
    pub second_leaf: Leaf,
}
