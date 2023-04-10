// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions to WIT types from the types defined in the crate root.
//!
//! These conversions are shared between the mocked contract and service system APIs.

use super::wit;
use crate::ChainId;
use linera_base::crypto::CryptoHash;

impl From<ChainId> for wit::CryptoHash {
    fn from(chain_id: ChainId) -> Self {
        chain_id.0.into()
    }
}

impl From<CryptoHash> for wit::CryptoHash {
    fn from(crypto_hash: CryptoHash) -> Self {
        let parts = <[u64; 4]>::from(crypto_hash);

        wit::CryptoHash {
            part1: parts[0],
            part2: parts[1],
            part3: parts[2],
            part4: parts[3],
        }
    }
}
