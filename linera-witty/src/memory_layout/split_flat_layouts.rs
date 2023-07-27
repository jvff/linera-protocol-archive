// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Splitting of the flat layout of a `variant` type into the flat layout of one of its variants.
//!
//! When flattening `variant` types, a single flat layout must be obtained for the type by joining
//! the flat layout of each variant. This means finding a flat type for each layout element to
//! represent the flat type of any of the variants. See [`crate::JoinFlatTypes`] for more
//! information on how flat types are joined.

use crate::primitive_types::FlatType;
use frunk::{HCons, HNil};

/// Converts the current joined flat layout into the `Target` flat layout, which may be shorter or
/// have elements that are narrower than the current elements.
pub trait SplitFlatLayouts<Target> {
    /// Converts the current joined flat layout into the `Target` flat layout.
    fn split(self) -> Target;
}

impl<AllFlatLayouts> SplitFlatLayouts<HNil> for AllFlatLayouts {
    fn split(self) -> HNil {
        HNil
    }
}

impl<Source, TargetTail> SplitFlatLayouts<HCons<(), TargetTail>> for Source
where
    Source: SplitFlatLayouts<TargetTail>,
{
    fn split(self) -> HCons<(), TargetTail> {
        HCons {
            head: (),
            tail: self.split(),
        }
    }
}

impl<SourceHead, SourceTail, TargetHead, TargetTail> SplitFlatLayouts<HCons<TargetHead, TargetTail>>
    for HCons<SourceHead, SourceTail>
where
    TargetHead: FlatType,
    SourceHead: FlatType,
    SourceTail: SplitFlatLayouts<TargetTail>,
{
    fn split(self) -> HCons<TargetHead, TargetTail> {
        HCons {
            head: self.head.split_into(),
            tail: self.tail.split(),
        }
    }
}
