// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the `WitLoad` derive macro.

#![cfg(test)]

use super::derive_for_struct;
use quote::quote;
use syn::{parse_quote, Fields, ItemStruct};

/// Check the generated code for the body of the implementation of `WitLoad` for a unit struct.
#[test]
fn zero_sized_type() {
    let input = Fields::Unit;
    let output = derive_for_struct(&input);

    let expected = quote! {
        const SIZE: u32 = {
            let mut size = 0;
            size
        };

        type Layout = linera_witty::HNil;
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
        const SIZE: u32 = {
            let mut size = 0;

            let field_alignment =
                <<u8 as linera_witty::WitType>::Layout as linera_witty::Layout>::ALIGNMENT;
            let field_size = <u8 as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;
            size += padding;
            size += field_size;

            let field_alignment =
                <<CustomType as linera_witty::WitType>::Layout as linera_witty::Layout>::ALIGNMENT;
            let field_size = <CustomType as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;
            size += padding;
            size += field_size;

            size
        };

        type Layout = <
            <linera_witty::HNil
            as std::ops::Add<<u8 as linera_witty::WitType>::Layout>>::Output
            as std::ops::Add<<CustomType as linera_witty::WitType>::Layout>>::Output;
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
        const SIZE: u32 = {
            let mut size = 0;

            let field_alignment =
                <<String as linera_witty::WitType>::Layout as linera_witty::Layout>::ALIGNMENT;
            let field_size = <String as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;
            size += padding;
            size += field_size;

            let field_alignment =
                <<Vec<CustomType> as linera_witty::WitType>::Layout
                    as linera_witty::Layout>::ALIGNMENT;
            let field_size = <Vec<CustomType> as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;
            size += padding;
            size += field_size;

            let field_alignment =
                <<i64 as linera_witty::WitType>::Layout as linera_witty::Layout>::ALIGNMENT;
            let field_size = <i64 as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;
            size += padding;
            size += field_size;

            size
        };

        type Layout = < <
            <linera_witty::HNil
            as std::ops::Add<<String as linera_witty::WitType>::Layout>>::Output
            as std::ops::Add<<Vec<CustomType> as linera_witty::WitType>::Layout>>::Output
            as std::ops::Add<<i64 as linera_witty::WitType>::Layout>>::Output;
    };

    assert_eq!(output.to_string(), expected.to_string());
}
