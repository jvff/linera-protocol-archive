// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unit tests for the `wit_export` attribute macro.

#![cfg(test)]

use syn::parse_quote;

use crate::{util::pretty_print, wit_export};

/// Check that it's possible to mix methods with and without caller parameters.
#[test]
fn mix_methods_with_and_without_caller_parameter() {
    let input = parse_quote! {
        impl SomeType {
            pub fn with_caller(caller: &mut Caller) {}
            pub fn without_caller() {}
        }
    };
    let parameters = parse_quote! { package = "package:namespace" };

    insta::assert_snapshot!(pretty_print(wit_export::generate(&input, parameters)));
}
