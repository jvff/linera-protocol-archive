// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types and functions useful in tests.

/// A drop-in replacement to `proc_macro_error::abort` that works in tests.
macro_rules! abort {
    ($span:expr, $( $message:tt )*) => {
        panic!($( $message )*);
    }
}
