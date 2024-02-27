// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types and functions shared between different macro implementations.

mod fields;
mod specialization;

#[cfg(with_wit_export)]
pub use self::specialization::Specialization;
pub use self::{fields::FieldsInformation, specialization::Specializations};
use darling::FromMeta;
use heck::ToKebabCase;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use quote::ToTokens;
use std::hash::{Hash, Hasher};
use syn::{
    parse::{self, Parse, ParseStream},
    punctuated::Punctuated,
    DeriveInput, Ident, Lit, LitStr, MetaNameValue, Token,
};

/// Changes the [`DeriveInput`] by replacing some generic type parameters with specialized types.
pub fn apply_specialization_attribute(input: &mut DeriveInput) -> Specializations {
    Specializations::prepare_derive_input(input)
}

/// A type representing the parameters for an attribute procedural macro.
#[derive(FromMeta)]
pub struct AttributeParameters {
    package: LitStr,
    interface: Option<LitStr>,
}

impl AttributeParameters {
    /// Parses the attribute parameters to the attribute procedural macro.
    pub fn new(attribute_parameters: proc_macro::TokenStream) -> Result<Self, darling::Error> {
        let meta = syn::parse(attribute_parameters.clone()).unwrap_or_else(|_| {
            abort!(
                TokenStream::from(attribute_parameters),
                "Failed to parse attribute parameters"
            )
        });

        Self::from_meta(&meta)
    }

    /// Returns the package name specified through the `package` attribute.
    pub fn package_name(&self) -> &LitStr {
        &self.package
    }

    /// Returns the interface name specified through the `interface` attribute, or inferred from
    /// the `type_name`
    pub fn interface_name(&self, type_name: &Ident) -> LitStr {
        self.interface.clone().unwrap_or_else(|| {
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
