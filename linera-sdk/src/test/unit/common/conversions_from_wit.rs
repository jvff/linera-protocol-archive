// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from WIT types to the types defined in the crate root.
//!
//! These conversions are shared between the mocked contract and service system APIs.

use super::wit;
use crate::{ApplicationId, ChainId, EffectId};
use linera_base::{crypto::CryptoHash, identifiers::BytecodeId};

impl From<wit::ApplicationId> for ApplicationId {
    fn from(application_id: wit::ApplicationId) -> Self {
        ApplicationId {
            bytecode_id: application_id.bytecode_id.into(),
            creation: application_id.creation.into(),
        }
    }
}

impl From<wit::EffectId> for BytecodeId {
    fn from(effect_id: wit::EffectId) -> Self {
        EffectId::from(effect_id).into()
    }
}

impl From<wit::EffectId> for EffectId {
    fn from(effect_id: wit::EffectId) -> Self {
        EffectId {
            chain_id: effect_id.chain_id.into(),
            height: effect_id.height.into(),
            index: effect_id.index,
        }
    }
}

impl From<wit::CryptoHash> for ChainId {
    fn from(crypto_hash: wit::CryptoHash) -> Self {
        ChainId(crypto_hash.into())
    }
}

impl From<wit::CryptoHash> for CryptoHash {
    fn from(crypto_hash: wit::CryptoHash) -> Self {
        CryptoHash::from([
            crypto_hash.part1,
            crypto_hash.part2,
            crypto_hash.part3,
            crypto_hash.part4,
        ])
    }
}
