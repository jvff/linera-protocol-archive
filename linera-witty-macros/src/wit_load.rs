// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Derivation of the `WitLoad` trait.

#[path = "unit_tests/wit_load.rs"]
mod tests;

use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::{format_ident, quote, ToTokens};
use syn::{Fields, Ident, LitInt, Variant};

/// Returns the body of the `WitLoad` implementation for the Rust `struct` with the specified
/// `fields`.
pub fn derive_for_struct(fields: &Fields) -> TokenStream {
    let field_pairs: Vec<_> = field_names_and_types(fields).collect();

    let load_fields = loads_for_fields(field_pairs.iter().cloned());
    let construction = construction_for_fields(field_pairs.iter().cloned(), fields);

    let lift_fields = field_pairs.iter().map(|(field_name, field_type)| {
        quote! {
            let (field_layout, flat_layout) = linera_witty::Split::split(flat_layout);
            let #field_name = <#field_type as WitLoad>::lift_from(field_layout, memory)?;
        }
    });

    quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            #( #load_fields )*

            Ok(Self #construction)
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
            #( #lift_fields )*

            Ok(Self #construction)
        }
    }
}

/// Returns the body of the `WitLoad` implementation for the Rust `enum` with the specified
/// `variants`.
pub fn derive_for_enum<'variants>(
    name: &Ident,
    variants: impl DoubleEndedIterator<Item = &'variants Variant> + Clone,
) -> TokenStream {
    let variant_count = variants.clone().count();
    let variants = variants.enumerate();

    let discriminant_type = if variant_count <= u8::MAX.into() {
        quote! { u8 }
    } else if variant_count <= u16::MAX.into() {
        quote! { u16 }
    } else if variant_count <= u32::MAX as usize {
        quote! { u32 }
    } else {
        abort!(name, "Too many variants in `enum`");
    };

    let load_variants = variants.clone().map(|(index, variant)| {
        let variant_name = &variant.ident;
        let index = LitInt::new(&index.to_string(), variant_name.span());
        let field_pairs = field_names_and_types(&variant.fields);
        let load_fields = loads_for_fields(field_pairs.clone());
        let construction = construction_for_fields(field_pairs, &variant.fields);

        quote! {
            #index => {
                #( #load_fields )*
                Ok(#name::#variant_name #construction)
            }
        }
    });

    let lift_variants = variants.map(|(index, variant)| {
        let variant_name = &variant.ident;
        let index = LitInt::new(&index.to_string(), variant_name.span());
        let field_pairs = field_names_and_types(&variant.fields);
        let field_names = field_pairs.clone().map(|(name, _)| name);
        let field_types = field_pairs.clone().map(|(_, field_type)| field_type);
        let construction = construction_for_fields(field_pairs, &variant.fields);

        quote! {
            #index => {
                let linera_witty::hlist_pat![#( #field_names, )*] =
                    <linera_witty::HList![#( #field_types ),*] as WitLoad>::lift_from(
                        linera_witty::SplitFlatLayouts::split(flat_layout),
                        memory,
                    )?;

                Ok(#name::#variant_name #construction)
            }
        }
    });

    quote! {
        fn load<Instance>(
            memory: &linera_witty::Memory<'_, Instance>,
            mut location: linera_witty::GuestPointer,
        ) -> Result<Self, linera_witty::RuntimeError>
        where
            Instance: linera_witty::InstanceWithMemory,
            <Instance::Runtime as linera_witty::Runtime>::Memory:
                linera_witty::RuntimeMemory<Instance>,
        {
            location = location.after_padding_for::<#discriminant_type>();
            let discriminant = <#discriminant_type as linera_witty::WitLoad>::load(
                memory,
                location,
            )?;
            location = location.after::<#discriminant_type>();

            match discriminant {
                #( #load_variants )*
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
            let discriminant = <#discriminant_type as linera_witty::WitLoad>::lift_from(
                linera_witty::hlist![discriminant_flat_type],
                memory,
            )?;

            match discriminant {
                #( #lift_variants )*
                _ => Err(linera_witty::RuntimeError::InvalidVariant),
            }
        }
    }
}

/// Returns an iterator over the names and types of the provided `fields`.
fn field_names_and_types(
    fields: &Fields,
) -> impl Iterator<Item = (Ident, TokenStream)> + Clone + '_ {
    let field_names = fields.iter().enumerate().map(|(index, field)| {
        field
            .ident
            .as_ref()
            .cloned()
            .unwrap_or_else(|| format_ident!("field{index}"))
    });

    let field_types = fields.iter().map(|field| field.ty.to_token_stream());

    field_names.zip(field_types)
}

/// Returns the code generated to load a single field.
///
/// Assumes that `location` points to where the field starts in memory, and advances it to the end
/// of the field. A binding with the field name is created in the generated code.
fn loads_for_fields(
    field_names_and_types: impl Iterator<Item = (Ident, TokenStream)> + Clone,
) -> impl Iterator<Item = TokenStream> {
    field_names_and_types.map(|(field_name, field_type)| {
        quote! {
            location = location.after_padding_for::<#field_type>();
            let #field_name = <#field_type as linera_witty::WitLoad>::load(memory, location)?;
            location = location.after::<#field_type>();
        }
    })
}

/// Returns the code to construct an instance of the field type.
///
/// Assumes that bindings were created with the field names.
fn construction_for_fields(
    field_names_and_types: impl Iterator<Item = (Ident, TokenStream)>,
    fields: &Fields,
) -> TokenStream {
    let field_names = field_names_and_types.map(|(name, _)| name);

    match fields {
        Fields::Unit => quote! {},
        Fields::Named(_) => quote! { { #( #field_names ),* } },
        Fields::Unnamed(_) => quote! {( #( #field_names ),* ) },
    }
}
