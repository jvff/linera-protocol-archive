// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types and functions useful in tests.

use proc_macro2::TokenStream;

/// A drop-in replacement to `proc_macro_error::abort` that works in tests.
macro_rules! abort {
    ($span:expr, $( $message:tt )*) => {{
        let _ = $span;
        panic!($( $message )*);
    }}
}

/// Creates a pretty-formatted Rust snippet string from some `tokens`.
pub fn pretty_print(tokens: TokenStream) -> String {
    prettyplease::unparse(
        &syn::parse2::<syn::File>(tokens).expect("Failed to parse tokens to `pretty_print`"),
    )
}
