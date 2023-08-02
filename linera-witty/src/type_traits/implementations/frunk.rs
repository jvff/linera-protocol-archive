// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of the custom traits for types from the standard library.

use crate::{
    GuestPointer, InstanceWithMemory, Layout, Memory, Runtime, RuntimeError, RuntimeMemory, Split,
    WitLoad, WitStore, WitType,
};
use frunk::{HCons, HNil};
use std::ops::Add;

impl WitType for HNil {
    const SIZE: u32 = 0;

    type Layout = HNil;
}

impl WitLoad for HNil {
    fn load<Instance>(
        _memory: &Memory<'_, Instance>,
        _location: GuestPointer,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
    {
        Ok(HNil)
    }

    fn lift_from<Instance>(
        HNil: <Self::Layout as Layout>::Flat,
        _memory: &Memory<'_, Instance>,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
    {
        Ok(HNil)
    }
}

impl WitStore for HNil {
    fn store<Instance>(
        &self,
        _memory: &mut Memory<'_, Instance>,
        _location: GuestPointer,
    ) -> Result<(), RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        Ok(())
    }

    fn lower<Instance>(
        &self,
        _memory: &mut Memory<'_, Instance>,
    ) -> Result<<Self::Layout as Layout>::Flat, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        Ok(HNil)
    }
}

impl<Head, Tail> WitType for HCons<Head, Tail>
where
    Head: WitType,
    Tail: WitType + SizeCalculation,
    Head::Layout: Add<Tail::Layout>,
    <Head::Layout as Add<Tail::Layout>>::Output: Layout,
{
    const SIZE: u32 = Self::SIZE_STARTING_AT_8_BYTE_BOUNDARY;

    type Layout = <Head::Layout as Add<Tail::Layout>>::Output;
}

impl<Head, Tail> WitLoad for HCons<Head, Tail>
where
    Head: WitLoad,
    Tail: WitLoad + SizeCalculation,
    Head::Layout: Add<Tail::Layout>,
    <Head::Layout as Add<Tail::Layout>>::Output: Layout,
    <Self::Layout as Layout>::Flat:
        Split<<Head::Layout as Layout>::Flat, Remainder = <Tail::Layout as Layout>::Flat>,
{
    fn load<Instance>(
        memory: &Memory<'_, Instance>,
        location: GuestPointer,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        Ok(HCons {
            head: Head::load(memory, location)?,
            tail: Tail::load(
                memory,
                location
                    .after::<Head>()
                    .after_padding_for::<Tail::FirstElement>(),
            )?,
        })
    }

    fn lift_from<Instance>(
        layout: <Self::Layout as Layout>::Flat,
        memory: &Memory<'_, Instance>,
    ) -> Result<Self, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        let (head_layout, tail_layout) = layout.split();

        Ok(HCons {
            head: Head::lift_from(head_layout, memory)?,
            tail: Tail::lift_from(tail_layout, memory)?,
        })
    }
}

impl<Head, Tail> WitStore for HCons<Head, Tail>
where
    Head: WitStore,
    Tail: WitStore + SizeCalculation,
    Head::Layout: Add<Tail::Layout>,
    <Head::Layout as Add<Tail::Layout>>::Output: Layout,
    <Head::Layout as Layout>::Flat: Add<<Tail::Layout as Layout>::Flat>,
    Self::Layout: Layout<
        Flat = <<Head::Layout as Layout>::Flat as Add<<Tail::Layout as Layout>::Flat>>::Output,
    >,
{
    fn store<Instance>(
        &self,
        memory: &mut Memory<'_, Instance>,
        location: GuestPointer,
    ) -> Result<(), RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        self.head.store(memory, location)?;
        self.tail.store(
            memory,
            location
                .after::<Head>()
                .after_padding_for::<Tail::FirstElement>(),
        )?;

        Ok(())
    }

    fn lower<Instance>(
        &self,
        memory: &mut Memory<'_, Instance>,
    ) -> Result<<Self::Layout as Layout>::Flat, RuntimeError>
    where
        Instance: InstanceWithMemory,
        <Instance::Runtime as Runtime>::Memory: RuntimeMemory<Instance>,
    {
        let head_layout = self.head.lower(memory)?;
        let tail_layout = self.tail.lower(memory)?;

        Ok(head_layout + tail_layout)
    }
}

/// Helper trait used to calculate the size of a heterogeneous list considering internal alignment.
///
/// Assumes the maximum alignment necessary for any type is 8 bytes, which is the alignment for the
/// largest flat types (`i64` and `f64`).
trait SizeCalculation {
    /// The size of the list considering the current size calculation starts at the 8-byte
    /// boundary.
    const SIZE_STARTING_AT_8_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 7-byte
    /// boundary.
    const SIZE_STARTING_AT_7_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 6-byte
    /// boundary.
    const SIZE_STARTING_AT_6_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 5-byte
    /// boundary.
    const SIZE_STARTING_AT_5_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 4-byte
    /// boundary.
    const SIZE_STARTING_AT_4_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 3-byte
    /// boundary.
    const SIZE_STARTING_AT_3_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 2-byte
    /// boundary.
    const SIZE_STARTING_AT_2_BYTE_BOUNDARY: u32;

    /// The size of the list considering the current size calculation starts at the 1-byte
    /// boundary.
    const SIZE_STARTING_AT_1_BYTE_BOUNDARY: u32;

    /// The type of the first element of the list, used to determine the current necessary
    /// alignment.
    type FirstElement: WitType;
}

impl SizeCalculation for HNil {
    const SIZE_STARTING_AT_8_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_7_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_6_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_5_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_4_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_3_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_2_BYTE_BOUNDARY: u32 = 0;
    const SIZE_STARTING_AT_1_BYTE_BOUNDARY: u32 = 0;

    type FirstElement = ();
}

macro_rules! calculate_size {
    ($boundary_offset:expr) => {{
        let alignment = <<Self::FirstElement as WitType>::Layout as Layout>::ALIGNMENT;
        let padding = (-($boundary_offset as i32) & (alignment as i32 - 1)) as u32;
        let size_after_head = padding + Head::SIZE;

        let tail_size = match ($boundary_offset + size_after_head) % 8 {
            0 => Tail::SIZE_STARTING_AT_8_BYTE_BOUNDARY,
            1 => Tail::SIZE_STARTING_AT_1_BYTE_BOUNDARY,
            2 => Tail::SIZE_STARTING_AT_2_BYTE_BOUNDARY,
            3 => Tail::SIZE_STARTING_AT_3_BYTE_BOUNDARY,
            4 => Tail::SIZE_STARTING_AT_4_BYTE_BOUNDARY,
            5 => Tail::SIZE_STARTING_AT_5_BYTE_BOUNDARY,
            6 => Tail::SIZE_STARTING_AT_6_BYTE_BOUNDARY,
            7 => Tail::SIZE_STARTING_AT_7_BYTE_BOUNDARY,
            _ => unreachable!(),
        };

        size_after_head + tail_size
    }};
}

impl<Head, Tail> SizeCalculation for HCons<Head, Tail>
where
    Head: WitType,
    Tail: SizeCalculation,
{
    const SIZE_STARTING_AT_8_BYTE_BOUNDARY: u32 = calculate_size!(8);
    const SIZE_STARTING_AT_7_BYTE_BOUNDARY: u32 = calculate_size!(7);
    const SIZE_STARTING_AT_6_BYTE_BOUNDARY: u32 = calculate_size!(6);
    const SIZE_STARTING_AT_5_BYTE_BOUNDARY: u32 = calculate_size!(5);
    const SIZE_STARTING_AT_4_BYTE_BOUNDARY: u32 = calculate_size!(4);
    const SIZE_STARTING_AT_3_BYTE_BOUNDARY: u32 = calculate_size!(3);
    const SIZE_STARTING_AT_2_BYTE_BOUNDARY: u32 = calculate_size!(2);
    const SIZE_STARTING_AT_1_BYTE_BOUNDARY: u32 = calculate_size!(1);

    type FirstElement = Head;
}
