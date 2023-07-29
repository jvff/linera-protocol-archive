// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the `WitLoad` derive macro.

#![cfg(test)]

use super::{derive_for_enum, derive_for_struct};
use quote::quote;
use syn::{parse_quote, Fields, ItemEnum, ItemStruct};

/// Check the generated code for the body of the implementation of `WitLoad` for a unit struct.
#[test]
fn zero_sized_type() {
    let input = Fields::Unit;
    let output = derive_for_struct(&input);

    let expected = quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            Ok(Self)
        }

        fn lift_from<Instance>(
            flat_layout: <Self::Layout as linera_witty::Layout>::Flat,
            memory: &linera_witty::Memory<'_, Instance>,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            Ok(Self)
        }
    };

    assert_eq!(output.to_string(), expected.to_string());
}

/// Check the generated code for the body of the implementation of `WitLoad` for a named struct.
#[test]
fn named_struct() {
    let input: ItemStruct = parse_quote! {
        struct Type {
            first: u8,
            second: CustomType,
        }
    };
    let output = derive_for_struct(&input.fields);

    let expected = quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            location = location.after_padding_for::<u8>();
            let first = <u8 as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<u8>();

            location = location.after_padding_for::<CustomType>();
            let second = <CustomType as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<CustomType>();

            Ok(Self { first, second })
        }

        fn lift_from<Instance>(
            flat_layout: <Self::Layout as linera_witty::Layout>::Flat,
            memory: &linera_witty::Memory<'_, Instance>,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let first = <u8 as WitLoad>::lift_from(field_layout, memory)?;

            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let second = <CustomType as WitLoad>::lift_from(field_layout, memory)?;

            Ok(Self { first, second })
        }
    };

    assert_eq!(output.to_string(), expected.to_string());
}

/// Check the generated code for the body of the implementation of `WitLoad` for a tuple struct.
#[test]
fn tuple_struct() {
    let input: ItemStruct = parse_quote! {
        struct Type(String, Vec<CustomType>, i64);
    };
    let output = derive_for_struct(&input.fields);

    let expected = quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            location = location.after_padding_for::<String>();
            let field0 = <String as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<String>();

            location = location.after_padding_for::<Vec<CustomType> >();
            let field1 = <Vec<CustomType> as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<Vec<CustomType> >();

            location = location.after_padding_for::<i64>();
            let field2 = <i64 as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<i64>();

            Ok(Self(field0, field1, field2))
        }

        fn lift_from<Instance>(
            flat_layout: <Self::Layout as linera_witty::Layout>::Flat,
            memory: &linera_witty::Memory<'_, Instance>,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let field0 = <String as WitLoad>::lift_from(field_layout, memory)?;

            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let field1 = <Vec<CustomType> as WitLoad>::lift_from(field_layout, memory)?;

            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let field2 = <i64 as WitLoad>::lift_from(field_layout, memory)?;

            Ok(Self(field0, field1, field2))
        }
    };

    assert_eq!(output.to_string(), expected.to_string());
}

/// Check the generated code for the body of the implementation of `WitType` for an enum.
#[test]
fn enum_type() {
    let input: ItemEnum = parse_quote! {
        enum Enum {
            Empty,
            Tuple(i8, CustomType),
            Struct {
                first: (),
                second: String,
            },
        }
    };
    let output = derive_for_enum(&input.ident, input.variants.iter());

    let expected = quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            location = location.after_padding_for::<u8>();
            let discriminant = <u8 as linera_witty::WitLoad>::load(memory, location,)?;
            location = location.after::<u8>();

            match discriminant {
                0 => {
                    Ok(Enum::Empty)
                }
                1 => {
                    location = location.after_padding_for::<i8>();
                    let field0 = <i8 as linera_witty::WitLoad>::load(memory, location)?;
                    location = location.after::<i8>();

                    location = location.after_padding_for::<CustomType>();
                    let field1 = <CustomType as linera_witty::WitLoad>::load(memory, location)?;
                    location = location.after::<CustomType>();

                    Ok(Enum::Tuple(field0, field1))
                }
                2 => {
                    location = location.after_padding_for::<()>();
                    let first = <() as linera_witty::WitLoad>::load(memory, location)?;
                    location = location.after::<()>();

                    location = location.after_padding_for::<String>();
                    let second = <String as linera_witty::WitLoad>::load(memory, location)?;
                    location = location.after::<String>();

                    Ok(Enum::Struct { first, second })
                }
                _ => Err(linera_witty::RuntimeError::InvalidVariant),
            }
        }

        fn lift_from<Instance>(
            linera_witty::hlist_pat![discriminant_flat_type, ...flat_layout]:
                <Self::Layout as linera_witty::Layout>::Flat,
            memory: &linera_witty::Memory<'_, Instance>,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            let discriminant = <u8 as linera_witty::WitLoad>::lift_from(
                linera_witty::hlist![discriminant_flat_type],
                memory,
            )?;

            match discriminant {
                0 => {
                    let linera_witty::hlist_pat![] = <linera_witty::HList![] as WitLoad>::lift_from(
                        linera_witty::SplitFlatLayouts::split(flat_layout),
                        memory,
                    )?;

                    Ok(Enum::Empty)
                }
                1 => {
                    let linera_witty::hlist_pat![field0, field1,] =
                        <linera_witty::HList![i8, CustomType] as WitLoad>::lift_from(
                            linera_witty::SplitFlatLayouts::split(flat_layout),
                            memory,
                        )?;

                    Ok(Enum::Tuple(field0, field1))
                }
                2 => {
                    let linera_witty::hlist_pat![first, second,] =
                        <linera_witty::HList![(), String] as WitLoad>::lift_from(
                            linera_witty::SplitFlatLayouts::split(flat_layout),
                            memory,
                        )?;

                    Ok(Enum::Struct { first, second })
                }
                _ => Err(linera_witty::RuntimeError::InvalidVariant),
            }
        }
    };

    assert_eq!(output.to_string(), expected.to_string());
}
