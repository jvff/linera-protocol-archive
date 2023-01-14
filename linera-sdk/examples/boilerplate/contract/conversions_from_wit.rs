// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by [`wit_bindgen_rust`] to types declared in [`linera_sdk`].

use super::{
    contract,
    writable_system::{self as system, PollCallResult, PollLoad},
};
use linera_sdk::{
    ApplicationId, BlockHeight, BytecodeId, CalleeContext, ChainId, EffectContext, EffectId,
    HashValue, OperationContext, Session, SessionId, SystemBalance,
};
use std::task::Poll;
use linera_views::views::ViewError;
use crate::boilerplate::writable_system::{PollReadKeyBytes, PollFindStrippedKeys, PollFindStrippedKeyValues, PollWriteBatch};

impl From<contract::OperationContext> for OperationContext {
    fn from(application_context: contract::OperationContext) -> Self {
        OperationContext {
            chain_id: ChainId(application_context.chain_id.into()),
            height: BlockHeight(application_context.height),
            index: application_context.index,
        }
    }
}

impl From<contract::EffectContext> for EffectContext {
    fn from(application_context: contract::EffectContext) -> Self {
        EffectContext {
            chain_id: ChainId(application_context.chain_id.into()),
            height: BlockHeight(application_context.height),
            effect_id: application_context.effect_id.into(),
        }
    }
}

impl From<contract::EffectId> for EffectId {
    fn from(effect_id: contract::EffectId) -> Self {
        EffectId {
            chain_id: ChainId(effect_id.chain_id.into()),
            height: BlockHeight(effect_id.height),
            index: effect_id.index,
        }
    }
}

impl From<system::EffectId> for EffectId {
    fn from(effect_id: system::EffectId) -> Self {
        EffectId {
            chain_id: ChainId(effect_id.chain_id.into()),
            height: BlockHeight(effect_id.height),
            index: effect_id.index,
        }
    }
}

impl From<contract::CalleeContext> for CalleeContext {
    fn from(application_context: contract::CalleeContext) -> Self {
        CalleeContext {
            chain_id: ChainId(application_context.chain_id.into()),
            authenticated_caller_id: application_context
                .authenticated_caller_id
                .map(ApplicationId::from),
        }
    }
}

impl From<contract::ApplicationId> for ApplicationId {
    fn from(application_id: contract::ApplicationId) -> Self {
        ApplicationId {
            bytecode: BytecodeId(application_id.bytecode_id.into()),
            creation: application_id.creation.into(),
        }
    }
}

impl From<system::ApplicationId> for ApplicationId {
    fn from(application_id: system::ApplicationId) -> Self {
        ApplicationId {
            bytecode: BytecodeId(application_id.bytecode_id.into()),
            creation: application_id.creation.into(),
        }
    }
}

impl From<contract::SessionId> for SessionId {
    fn from(session_id: contract::SessionId) -> Self {
        SessionId {
            application_id: session_id.application_id.into(),
            kind: session_id.kind,
            index: session_id.index,
        }
    }
}

impl From<contract::Session> for Session {
    fn from(session: contract::Session) -> Self {
        Session {
            kind: session.kind,
            data: session.data,
        }
    }
}

impl From<contract::HashValue> for HashValue {
    fn from(hash_value: contract::HashValue) -> Self {
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

impl From<system::HashValue> for HashValue {
    fn from(hash_value: system::HashValue) -> Self {
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

impl From<system::SystemBalance> for SystemBalance {
    fn from(balance: system::SystemBalance) -> Self {
        let value = ((balance.upper_half as u128) << 64) | (balance.lower_half as u128);
        SystemBalance(value)
    }
}

impl From<PollLoad> for Poll<Result<Vec<u8>, String>> {
    fn from(poll_get: PollLoad) -> Poll<Result<Vec<u8>, String>> {
        match poll_get {
            PollLoad::Ready(bytes) => Poll::Ready(bytes),
            PollLoad::Pending => Poll::Pending,
        }
    }
}

impl From<PollCallResult> for Poll<Result<(Vec<u8>, Vec<SessionId>), String>> {
    fn from(poll_call_result: PollCallResult) -> Poll<Result<(Vec<u8>, Vec<SessionId>), String>> {
        match poll_call_result {
            PollCallResult::Ready(Ok(result)) => Poll::Ready(Ok(result.into())),
            PollCallResult::Ready(Err(message)) => Poll::Ready(Err(message)),
            PollCallResult::Pending => Poll::Pending,
        }
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

impl From<system::CallResult> for (Vec<u8>, Vec<SessionId>) {
    fn from(call_result: system::CallResult) -> (Vec<u8>, Vec<SessionId>) {
        let value = call_result.value;

        let sessions = call_result
            .sessions
            .into_iter()
            .map(SessionId::from)
            .collect();

        (value, sessions)
    }
}

impl From<system::SessionId> for SessionId {
    fn from(session_id: system::SessionId) -> SessionId {
        SessionId {
            application_id: session_id.application_id.into(),
            kind: session_id.kind,
            index: session_id.index,
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

impl From<PollFindStrippedKeyValues> for Poll<Result<Vec<(Vec<u8>,Vec<u8>)>, ViewError>> {
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

impl From<PollWriteBatch> for Poll<Result<(), ViewError>> {
    fn from(poll_write_batch: PollWriteBatch) -> Self {
        match poll_write_batch {
            PollWriteBatch::Ready(Ok(())) => Poll::Ready(Ok(())),
            PollWriteBatch::Ready(Err(error)) => {
                Poll::Ready(Err(ViewError::WasmHostGuestError(error)))
            }
            PollWriteBatch::Pending => Poll::Pending,
        }
    }
}
