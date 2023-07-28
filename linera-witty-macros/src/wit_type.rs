// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Derivation of the `WitType` trait.

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{Fields, Ident, Type, Variant};

#[path = "unit_tests/wit_type.rs"]
mod tests;

/// Returns the body of the `WitType` implementation for the Rust `struct` with the specified
/// `fields`.
pub fn derive_for_struct(fields: &Fields) -> TokenStream {
    let field_types = fields.iter().map(|field| &field.ty);
    let size = struct_size_calculation(field_types.clone(), &quote! { 0 });
    let layout = struct_layout_type(field_types);

    quote! {
        const SIZE: u32 = #size;

        type Layout = #layout;
    }
}

/// Returns the body of the `WitType` implementation for the Rust `enum` with the specified
/// `variants`.
pub fn derive_for_enum<'variants>(
    name: &Ident,
    variants: impl DoubleEndedIterator<Item = &'variants Variant> + Clone,
) -> TokenStream {
    let variant_count = variants.clone().count();
    let variant_type_lists = variants.map(|variant| variant.fields.iter().map(|field| &field.ty));

    let discriminant_type = if variant_count <= u8::MAX.into() {
        quote! { u8 }
    } else if variant_count <= u16::MAX.into() {
        quote! { u16 }
    } else if variant_count <= u32::MAX as usize {
        quote! { u32 }
    } else {
        abort!(name, "Too many variants in `enum`");
    };

    let discriminant_size = quote! { std::mem::size_of::<#discriminant_type>() as u32 };

    let variant_sizes = variant_type_lists
        .clone()
        .map(|field_types| struct_size_calculation(field_types, &discriminant_size))
        .map(|size| {
            quote! {
                let variant_size = #size;

                if variant_size > size {
                    size = variant_size;
                }
            }
        });

    let variant_layouts =
        variant_type_lists
            .map(struct_layout_type)
            .rev()
            .reduce(|current, variant_layout| {
                quote! {
                    <#variant_layout as linera_witty::Merge<#current>>::Output
                }
            });

    quote! {
        const SIZE: u32 = {
            let mut size = #discriminant_size;
            #(#variant_sizes)*
            size
        };

        type Layout = linera_witty::HCons<#discriminant_type, #variant_layouts>;
    }
}

/// Returns an expression that calculates the size in memory of the sequence of `field_types`.
fn struct_size_calculation<'fields>(
    field_types: impl Iterator<Item = &'fields Type>,
    prefix_size: &TokenStream,
) -> TokenStream {
    let field_size_calculations = field_types.map(|field_type| {
        quote! {
            let field_alignment =
                <<#field_type as linera_witty::WitType>::Layout as linera_witty::Layout>::ALIGNMENT;
            let field_size = <#field_type as linera_witty::WitType>::SIZE;
            let padding = (-(size as i32) & (field_alignment as i32 - 1)) as u32;

            size += padding;
            size += field_size;
        }
    });

    quote! {{
        let mut size = #prefix_size;
        #(#field_size_calculations)*
        size
    }}
}

/// Returns the layout type for the sequence of `field_types`.
fn struct_layout_type<'fields>(field_types: impl Iterator<Item = &'fields Type>) -> TokenStream {
    field_types.fold(quote! { linera_witty::HNil }, |current, field_type| {
        quote! {
            <#current as std::ops::Add<<#field_type as linera_witty::WitType>::Layout>>::Output
        }
    })
}
