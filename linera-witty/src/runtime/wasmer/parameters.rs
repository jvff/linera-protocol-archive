// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Representation of Wasmer function parameter types.

use crate::primitive_types::FlatType;
use frunk::{hlist, hlist_pat, HList};
use wasmer::FromToNativeWasmType;

/// Conversions between flat layouts and Wasmer parameter types.
pub trait WasmerParameters {
    /// The type Wasmer uses to represent the parameters in a function imported from a guest.
    type ImportParameters;

    /// The type Wasmer uses to represent the parameters in a function exported from a host.
    type ExportParameters;

    /// Converts from this flat layout into Wasmer's representation for functions imported from a
    /// guest.
    fn into_wasmer(self) -> Self::ImportParameters;

    /// Converts from Wasmer's representation for functions exported from the host into this flat
    /// layout.
    fn from_wasmer(parameters: Self::ExportParameters) -> Self;
}

impl WasmerParameters for HList![] {
    type ImportParameters = ();
    type ExportParameters = ();

    fn into_wasmer(self) -> Self::ImportParameters {}

    fn from_wasmer((): Self::ExportParameters) -> Self {
        hlist![]
    }
}

impl<Parameter> WasmerParameters for HList![Parameter]
where
    Parameter: FlatType + FromToNativeWasmType,
{
    type ImportParameters = Parameter;
    type ExportParameters = (Parameter,);

    #[allow(clippy::unused_unit)]
    fn into_wasmer(self) -> Self::ImportParameters {
        let hlist_pat![parameter] = self;

        parameter
    }

    fn from_wasmer((parameter,): Self::ExportParameters) -> Self {
        hlist![parameter]
    }
}

/// Helper macro to implement [`WasmerParameters`] for flat layouts up to the maximum limit.
///
/// The maximum number of parameters is defined by the [canonical
/// ABI](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md#flattening)
/// as the `MAX_FLAT_PARAMS` constant.
macro_rules! parameters {
    ($( $names:ident : $types:ident ),*) => {
        impl<$( $types ),*> WasmerParameters for HList![$( $types ),*]
        where
            $( $types: FlatType + FromToNativeWasmType, )*
        {
            type ImportParameters = ($( $types, )*);
            type ExportParameters = ($( $types, )*);

            #[allow(clippy::unused_unit)]
            fn into_wasmer(self) -> Self::ImportParameters {
                let hlist_pat![$( $names ),*] = self;

                ($( $names, )*)
            }

            fn from_wasmer(($( $names, )*): Self::ExportParameters) -> Self {
                hlist![$( $names ),*]
            }
        }
    };
}

repeat_macro!(parameters =>
    a: A,
    b: B |
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
    p: P
);
