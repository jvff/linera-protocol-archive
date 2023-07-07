// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Representation of the layout of complex types as a sequence of native WebAssembly types.

use super::Layout;
use crate::{primitive_types::FlatType, util::Split};
use frunk::{HCons, HNil};

/// Representation of the layout of complex types as a sequence of native WebAssembly types.
///
/// This allows laying out complex types as a sequence of WebAssembly types that can represent the
/// parameters or the return list of a function. WIT uses this as an optimization to pass complex
/// types as multiple native WebAssembly parameters.
pub trait FlatLayout: Layout {
    /// Result of appending some `Other` flat layout to this flat layout.
    type FlatAppend<Other: FlatLayout>: FlatLayout + Split<Self, Remainder = Other>;

    /// Appends some `other` flat layout with this flat layout, returning a new flat layout list.
    fn flat_append<Other>(self, other: Other) -> Self::FlatAppend<Other>
    where
        Other: FlatLayout;
}

impl FlatLayout for HNil {
    type FlatAppend<Other: FlatLayout> = Other;

    fn flat_append<Other>(self, other: Other) -> Self::FlatAppend<Other>
    where
        Other: FlatLayout,
    {
        other
    }
}

impl<Head, Tail> FlatLayout for HCons<Head, Tail>
where
    Head: FlatType,
    Tail: FlatLayout,
{
    type FlatAppend<Other: FlatLayout> = HCons<Head, Tail::FlatAppend<Other>>;

    fn flat_append<Other>(self, other: Other) -> Self::FlatAppend<Other>
    where
        Other: FlatLayout,
    {
        HCons {
            head: self.head,
            tail: self.tail.flat_append(other),
        }
    }
}
