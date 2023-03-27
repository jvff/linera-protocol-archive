// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{de::DeserializeOwned, Serialize};

pub trait FromBcsBytes: Sized {
    fn from_bcs_bytes(bytes: &[u8]) -> Result<Self, bcs::Error>;
}

impl<T> FromBcsBytes for T
where
    T: DeserializeOwned,
{
    fn from_bcs_bytes(bytes: &[u8]) -> Result<Self, bcs::Error> {
        bcs::from_bytes(bytes)
    }
}

pub trait ToBcsBytes {
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, bcs::Error>;
}

impl<T> ToBcsBytes for T
where
    T: Serialize,
{
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, bcs::Error> {
        bcs::to_bytes(self)
    }
}
