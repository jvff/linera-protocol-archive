// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Representation of the memory layout of complex types as a sequence of fundamental WIT types.

use super::element::LayoutElement;
use crate::util::Split;
use frunk::{hlist::HList, HCons, HNil};

/// Marker trait to prevent [`LayoutElement`] to be implemented for other types.
pub trait Sealed {}

/// Representation of the memory layout of complex types as a sequence of fundamental WIT types.
pub trait Layout: Sealed + Default + HList {
    /// The alignment boundary required for the layout.
    const ALIGNMENT: u32;

    /// Result of appending some `Other` layout to this layout.
    type Append<Other: Layout>: Layout + Split<Self, Remainder = Other>;

    /// Appends some `other` layout with this layout, returning a new layout list.
    fn append<Other>(self, other: Other) -> Self::Append<Other>
    where
        Other: Layout;
}

impl Sealed for HNil {}
impl<Head, Tail> Sealed for HCons<Head, Tail>
where
    Head: LayoutElement,
    Tail: Layout,
{
}

impl Layout for HNil {
    const ALIGNMENT: u32 = 1;

    type Append<Other: Layout> = Other;

    fn append<Other>(self, other: Other) -> Self::Append<Other>
    where
        Other: Layout,
    {
        other
    }
}

impl<Head, Tail> Layout for HCons<Head, Tail>
where
    Head: LayoutElement,
    Tail: Layout,
{
    const ALIGNMENT: u32 = if Head::ALIGNMENT > Tail::ALIGNMENT {
        Head::ALIGNMENT
    } else {
        Tail::ALIGNMENT
    };

    type Append<Other: Layout> = HCons<Head, Tail::Append<Other>>;

    fn append<Other>(self, other: Other) -> Self::Append<Other>
    where
        Other: Layout,
    {
        HCons {
            head: self.head,
            tail: self.tail.append(other),
        }
    }
}
