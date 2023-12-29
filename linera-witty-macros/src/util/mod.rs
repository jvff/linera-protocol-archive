// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types and functions shared between different macro implementations.

mod specialization;

pub use self::specialization::{Specialization, Specializations};
use heck::ToKebabCase;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, ToTokens};
use std::hash::{Hash, Hasher};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Field, Fields, Ident, Lit, LitStr, Meta, MetaList, MetaNameValue, Token, Type,
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

/// Changes the [`DeriveInput`] by replacing some generic type parameters with specialized types.
pub fn apply_specialization_attribute(input: &mut DeriveInput) -> Specializations {
    Specializations::prepare_derive_input(input)
}

/// Returns `true` if `the_type` is the unit `()` type.
pub fn is_unit_type(the_type: &Type) -> bool {
    matches!(the_type, Type::Tuple(tuple) if tuple.elems.is_empty())
}

/// Returns `true` if the `field` is marked to be skipped using the `#[witty(skip)]` attribute.
pub fn should_skip_field(field: &Field) -> bool {
    field.attrs.iter().any(|attribute| {
        matches!(
            &attribute.meta,
            Meta::List(MetaList { path, tokens, ..})
                if path.is_ident("witty") && tokens.to_string() == "skip"
        )
    })
}

/// A type representing the parameters for an attribute procedural macro.
pub struct AttributeParameters {
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
    /// Parses the attribute parameters to the attribute procedural macro.
    pub fn new(attribute_parameters: proc_macro::TokenStream) -> Self {
        syn::parse(attribute_parameters.clone()).unwrap_or_else(|_| {
            abort!(
                TokenStream::from(attribute_parameters),
                r#"Failed to parse attribute parameters, expected either `root = true` \
                or `package = "namespace:package""#
            )
        })
    }

    /// Returns the string value of a parameter named `name`, if it exists.
    pub fn parameter(&self, name: &str) -> Option<&'_ LitStr> {
        self.metadata
            .iter()
            .find(|pair| pair.path.is_ident(name))
            .map(|pair| {
                let syn::Expr::Lit(syn::ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = &pair.value
                else {
                    abort!(&pair.value, "Expected a string literal");
                };

                lit_str
            })
    }

    /// Returns the package name specified through the `package` attribute.
    pub fn package_name(&self) -> &'_ LitStr {
        self.parameter("package").unwrap_or_else(|| {
            abort!(
                Span::call_site(),
                r#"Missing package name specifier in attribute parameters \
                (package = "namespace:package")"#
            )
        })
    }

    /// Returns the interface name specified through the `interface` attribute, or inferred from
    /// the `type_name`
    pub fn interface_name(&self, type_name: &Ident) -> LitStr {
        self.parameter("interface").cloned().unwrap_or_else(|| {
            LitStr::new(&type_name.to_string().to_kebab_case(), type_name.span())
        })
    }

    /// Returns the namespace to use to prefix function names.
    ///
    /// This is based on the package name and the interface name. The former must be specified
    /// using the `package` attribute parameter, while the latter can be specified using the
    /// `interface` attribute parameter or inferred from the `type_name`.
    pub fn namespace(&self, type_name: &Ident) -> LitStr {
        let package = self.package_name();
        let interface = self.interface_name(type_name);

        LitStr::new(
            &format!("{}/{}", package.value(), interface.value()),
            interface.span(),
        )
    }
}

/// A helper type to allow comparing [`TokenStream`] instances, allowing it to be used in a
/// [`HashSet`].
pub struct TokensSetItem<'input> {
    string: String,
    tokens: &'input TokenStream,
}

impl<'input> From<&'input TokenStream> for TokensSetItem<'input> {
    fn from(tokens: &'input TokenStream) -> Self {
        TokensSetItem {
            string: tokens.to_string(),
            tokens,
        }
    }
}

impl PartialEq for TokensSetItem<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.string.eq(&other.string)
    }
}

impl Eq for TokensSetItem<'_> {}

impl Hash for TokensSetItem<'_> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.string.hash(state)
    }
}

impl ToTokens for TokensSetItem<'_> {
    fn to_tokens(&self, stream: &mut TokenStream) {
        self.tokens.to_tokens(stream)
    }
}
