// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of the custom traits for the tuple types.

use crate::WitType;
use frunk::HList;

macro_rules! impl_wit_traits {
    ($head_name:ident : $head_type:ident, $( $tail_names:ident : $tail_types:ident ),*) => {
        impl_wit_traits!($head_name: $head_type | $( $tail_names: $tail_types ),*);
    };

    ($( $names:ident : $types:ident ),* |) => {
        impl_wit_traits!(@generate $( $names: $types, )*);
    };

    (
        $( $names_to_generate:ident : $types_to_generate:ident ),* |
        $next_name:ident : $next_type:ident $( , $queued_names:ident : $queued_types:ident )*
    ) => {
        impl_wit_traits!(@generate $( $names_to_generate: $types_to_generate, )*);
        impl_wit_traits!(
            $( $names_to_generate: $types_to_generate, )*
            $next_name: $next_type | $( $queued_names: $queued_types ),*);
    };

    (@generate $( $names:ident : $types:ident, )*) => {
        impl<$( $types ),*> WitType for ($( $types, )*)
        where
            $( $types: WitType, )*
            HList![$( $types ),*]: WitType,
        {
            const SIZE: u32 = <HList![$( $types ),*] as WitType>::SIZE;

            type Layout = <HList![$( $types ),*] as WitType>::Layout;
        }
    };
}

impl_wit_traits!(
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
    h: H,
    i: I,
    j: J,
    k: K,
    l: L,
    m: M,
    n: N,
    o: O,
    p: P,
    q: Q,
    r: R,
    s: S,
    t: T,
    u: U,
    v: V,
    w: W,
    x: X,
    y: Y,
    z: Z
);
