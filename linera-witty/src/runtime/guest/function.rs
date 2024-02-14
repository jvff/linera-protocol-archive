// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implementations of [`InstanceWithFunction`] for the [`Guest`] runtime.

use super::{Export, Guest};
use crate::{primitive_types::FlatType, InstanceWithFunction, Runtime, RuntimeError};
use frunk::{hlist, hlist_pat, HList};
use std::{any::TypeId, mem};

/// Implements [`InstanceWithFunction`] for functions with the provided amount of parameters for
/// the [`EntrypointInstance`] and [`ReentrantInstance`] types.
macro_rules! impl_instance_with_function {
    ($( $names:ident : $types:ident ),*) => {
        impl<$( $types ),*> InstanceWithFunction<HList![$( $types ),*], HList![]> for Guest
        where
            $( $types: FlatType + 'static, )*
        {
            type Function = fn($( $types ),*);

            fn function_from_export(
                &mut self,
                export: <Self::Runtime as Runtime>::Export,
            ) -> Result<Option<Self::Function>, RuntimeError> {
                match export {
                    Export::Function { pointer, signature } => {
                        assert_eq!(signature, TypeId::of::<Self::Function>());

                        Ok(Some(unsafe { mem::transmute(pointer) }))
                    }
                    _ => Err(RuntimeError::NotAFunction(String::new())),
                }
            }

            fn call(
                &mut self,
                function: &Self::Function,
                hlist_pat![$( $names ),*]: HList![$( $types ),*],
            ) -> Result<HList![], RuntimeError> {
                (*function)($( $names ),*);

                Ok(hlist![])
            }
        }

        impl<$( $types, )* Output> InstanceWithFunction<HList![$( $types ),*], HList![Output]>
            for Guest
        where
            $( $types: FlatType + 'static, )*
            Output: FlatType + 'static,
        {
            type Function = fn($( $types ),*) -> Output;

            fn function_from_export(
                &mut self,
                export: <Self::Runtime as Runtime>::Export,
            ) -> Result<Option<Self::Function>, RuntimeError> {
                match export {
                    Export::Function { pointer, signature } => {
                        assert_eq!(signature, TypeId::of::<Self::Function>());

                        Ok(Some(unsafe { mem::transmute(pointer) }))
                    }
                    _ => Err(RuntimeError::NotAFunction(String::new())),
                }
            }

            fn call(
                &mut self,
                function: &Self::Function,
                hlist_pat![$( $names ),*]: HList![$( $types ),*],
            ) -> Result<HList![Output], RuntimeError> {
                let output = (*function)($( $names ),*);

                Ok(hlist![output])
            }
        }
    };
}

repeat_macro!(impl_instance_with_function =>
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
    p: P
);
