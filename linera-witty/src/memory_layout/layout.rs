// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Representation of the memory layout of complex types as a sequence of fundamental WIT types.

use super::{element::LayoutElement, FlatLayout};
use crate::{primitive_types::MaybeFlatType, util::Split};
use frunk::{hlist::HList, HCons, HNil};

/// Marker trait to prevent [`LayoutElement`] to be implemented for other types.
pub trait Sealed {}

/// Representation of the memory layout of complex types as a sequence of fundamental WIT types.
pub trait Layout: Sealed + Default + HList {
    /// The alignment boundary required for the layout.
    const ALIGNMENT: u32;

    /// Result of flattening this layout.
    type Flat: FlatLayout;

    /// Result of appending some `Other` layout to this layout.
    type Append<Other: Layout>: Layout + Split<Self, Remainder = Other>;

    /// Flattens this layout into a layout consisting of native WebAssembly types.
    ///
    /// The resulting flat layout does not have any empty items.
    fn flatten(self) -> Self::Flat;

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

    type Flat = HNil;
    type Append<Other: Layout> = Other;

    fn flatten(self) -> Self::Flat {
        HNil
    }

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

    type Flat = <Head::Flat as MaybeFlatType>::Flatten<Tail>;
    type Append<Other: Layout> = HCons<Head, Tail::Append<Other>>;

    fn flatten(self) -> Self::Flat {
        self.head.flatten().flatten(self.tail)
    }

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
