// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions with zero-extension.

/// Converts from a type into a wider `Target` type by zero-extending the most significant bits.
pub trait ZeroExtend<Target> {
    /// Converts into the `Target` type by zero-extending the most significant bits.
    fn zero_extend(self) -> Target;
}

/// Macro to implement `ZeroExtend` from a `source` type to a `target` type using the provided
/// cast `conversions`.
macro_rules! impl_zero_extend {
    ($source:ident -> $target:ident => $( $conversions:tt )*) => {
        impl ZeroExtend<$target> for $source {
            fn zero_extend(self) -> $target {
                self $( $conversions )*
            }
        }
    };
}

impl_zero_extend!(u8 -> i32 => as i32);
impl_zero_extend!(i8 -> i32 => as u8 as i32);
impl_zero_extend!(u16 -> i32 => as i32);
impl_zero_extend!(i16 -> i32 => as u16 as i32);
impl_zero_extend!(u32 -> i32 => as i32);
impl_zero_extend!(i32 -> i32 =>);
impl_zero_extend!(i32 -> i64 => as u32 as i64);
impl_zero_extend!(u64 -> i64 => as i64);
impl_zero_extend!(i64 -> i64 =>);
