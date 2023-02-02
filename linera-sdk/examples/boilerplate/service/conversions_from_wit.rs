// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit_bindgen_rust`] to types declared in [`linera_sdk`].

use super::{queryable_system, service};
use crate::boilerplate::queryable_system::{
    PollFindStrippedKeyValues, PollFindStrippedKeys, PollLock, PollReadKeyBytes,
};
use linera_sdk::{
    ApplicationId, BlockHeight, BytecodeId, ChainId, EffectId, HashValue, QueryContext,
    SystemBalance,
};
use linera_views::views::ViewError;
use std::task::Poll;

impl From<service::QueryContext> for QueryContext {
    fn from(application_context: service::QueryContext) -> Self {
        QueryContext {
            chain_id: ChainId(application_context.chain_id.into()),
        }
    }
}

impl From<service::HashValue> for HashValue {
    fn from(hash_value: service::HashValue) -> Self {
        HashValue::from([
            hash_value.part1,
            hash_value.part2,
            hash_value.part3,
            hash_value.part4,
            hash_value.part5,
            hash_value.part6,
            hash_value.part7,
            hash_value.part8,
        ])
    }
}

impl From<queryable_system::HashValue> for HashValue {
    fn from(hash_value: queryable_system::HashValue) -> Self {
        HashValue::from([
            hash_value.part1,
            hash_value.part2,
            hash_value.part3,
            hash_value.part4,
            hash_value.part5,
            hash_value.part6,
            hash_value.part7,
            hash_value.part8,
        ])
    }
}

impl From<queryable_system::ApplicationId> for ApplicationId {
    fn from(application_id: queryable_system::ApplicationId) -> Self {
        ApplicationId {
            bytecode: BytecodeId(application_id.bytecode_id.into()),
            creation: application_id.creation.into(),
        }
    }
}

impl From<queryable_system::EffectId> for EffectId {
    fn from(effect_id: queryable_system::EffectId) -> Self {
        EffectId {
            chain_id: ChainId(effect_id.chain_id.into()),
            height: BlockHeight(effect_id.height),
            index: effect_id.index,
        }
    }
}

impl From<queryable_system::SystemBalance> for SystemBalance {
    fn from(balance: queryable_system::SystemBalance) -> Self {
        let value = ((balance.upper_half as u128) << 64) | (balance.lower_half as u128);
        SystemBalance(value)
    }
}

impl From<PollReadKeyBytes> for Poll<Result<Option<Vec<u8>>, ViewError>> {
    fn from(poll_read_key_bytes: PollReadKeyBytes) -> Self {
        match poll_read_key_bytes {
            PollReadKeyBytes::Ready(Ok(bytes)) => Poll::Ready(Ok(bytes)),
            PollReadKeyBytes::Ready(Err(error)) => {
                Poll::Ready(Err(ViewError::WasmHostGuestError(error)))
            }
            PollReadKeyBytes::Pending => Poll::Pending,
        }
    }
}

impl From<PollFindStrippedKeys> for Poll<Result<Vec<Vec<u8>>, ViewError>> {
    fn from(poll_find_stripped_keys: PollFindStrippedKeys) -> Self {
        match poll_find_stripped_keys {
            PollFindStrippedKeys::Ready(Ok(keys)) => Poll::Ready(Ok(keys)),
            PollFindStrippedKeys::Ready(Err(error)) => {
                Poll::Ready(Err(ViewError::WasmHostGuestError(error)))
            }
            PollFindStrippedKeys::Pending => Poll::Pending,
        }
    }
}

impl From<PollFindStrippedKeyValues> for Poll<Result<Vec<(Vec<u8>, Vec<u8>)>, ViewError>> {
    fn from(poll_find_stripped_key_values: PollFindStrippedKeyValues) -> Self {
        match poll_find_stripped_key_values {
            PollFindStrippedKeyValues::Ready(Ok(key_values)) => Poll::Ready(Ok(key_values)),
            PollFindStrippedKeyValues::Ready(Err(error)) => {
                Poll::Ready(Err(ViewError::WasmHostGuestError(error)))
            }
            PollFindStrippedKeyValues::Pending => Poll::Pending,
        }
    }
}

impl From<PollLock> for Poll<Result<(), ViewError>> {
    fn from(poll_lock: PollLock) -> Self {
        match poll_lock {
            PollLock::Ready(Ok(())) => Poll::Ready(Ok(())),
            PollLock::Ready(Err(error)) => Poll::Ready(Err(ViewError::WasmHostGuestError(error))),
            PollLock::Pending => Poll::Pending,
        }
    }
}
