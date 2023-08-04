// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types and functions shared between different macro implementations.

use heck::ToKebabCase;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    Fields, Ident, Lit, LitStr, MetaNameValue, Token,
};

/// Returns the code with a pattern to match a heterogenous list using the `field_names` as
/// bindings.
///
/// This function receives `field_names` instead of a `Fields` instance because some fields might
/// not have names, so binding names must be created for them.
pub fn hlist_bindings_for(field_names: impl Iterator<Item = Ident>) -> TokenStream {
    quote! { linera_witty::hlist_pat![#( #field_names ),*] }
}

/// Returns the code with a pattern to match a heterogenous list using the `field_names` as
/// bindings.
pub fn hlist_type_for(fields: &Fields) -> TokenStream {
    let field_types = fields.iter().map(|field| &field.ty);
    quote! { linera_witty::HList![#( #field_types ),*] }
}

/// Returns the package and namespace for the WIT interface generated by an attribute macro.
///
/// Requires a `package` to be specified in `attribute_parameters` and can use a specified
/// `namespace` or infer it from the `type_name`.
pub fn extract_namespace(
    attribute_parameters: proc_macro::TokenStream,
    type_name: &Ident,
) -> LitStr {
    let span = Span::call_site();
    let parameters = syn::parse::<AttributeParameters>(attribute_parameters).unwrap_or_else(|_| {
        abort!(
            span,
            r#"Failed to parse attribute parameters, expected either `root = true` \
                or `package = "namespace:package""#
        )
    });

    let package_name = parameters.parameter("package").unwrap_or_else(|| {
        abort!(
            span,
            r#"Missing package name specifier in attribute parameters \
                (package = "namespace:package")"#
        )
    });

    let interface_name = parameters
        .parameter("interface")
        .unwrap_or_else(|| type_name.to_string().to_kebab_case());

    LitStr::new(&format!("{package_name}/{interface_name}"), span)
}

/// A type representing the parameters for an attribute procedural macro.
struct AttributeParameters {
    metadata: Punctuated<MetaNameValue, Token![,]>,
}

impl Parse for AttributeParameters {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        Ok(AttributeParameters {
            metadata: Punctuated::parse_terminated(input)?,
        })
    }
}

impl AttributeParameters {
    /// Returns the string value of a parameter named `name`, if it exists.
    pub fn parameter(&self, name: &str) -> Option<String> {
        self.metadata
            .iter()
            .find(|pair| pair.path.is_ident(name))
            .map(|pair| {
                let Lit::Str(lit_str) = &pair.lit
                    else { abort!(&pair.lit, "Expected a string literal"); };

                lit_str.value()
            })
    }
}
