// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Conversions from types generated by `wit-bindgen`.
//!
//! Allows converting types returned from a WASM module into types that can be used with the rest
//! of the crate.

use super::runtime::{contract, writable_system};
use crate::{
    ApplicationCallResult, BytecodeId, NewSession, RawExecutionResult, SessionCallResult,
    SessionId, UserApplicationId,
};
use linera_base::{
    crypto::HashValue,
    messages::{BlockHeight, ChainId, Destination, EffectId},
};

impl From<contract::SessionCallResult> for (SessionCallResult, Vec<u8>) {
    fn from(result: contract::SessionCallResult) -> Self {
        let session_call_result = SessionCallResult {
            inner: result.inner.into(),
            close_session: result.data.is_some(),
        };

        let updated_session_data = result.data.unwrap_or_default();

        (session_call_result, updated_session_data)
    }
}

impl From<contract::ApplicationCallResult> for ApplicationCallResult {
    fn from(result: contract::ApplicationCallResult) -> Self {
        let create_sessions = result
            .create_sessions
            .into_iter()
            .map(NewSession::from)
            .collect();

        ApplicationCallResult {
            create_sessions,
            execution_result: result.execution_result.into(),
            value: result.value,
        }
    }
}

impl From<contract::ExecutionResult> for RawExecutionResult<Vec<u8>> {
    fn from(result: contract::ExecutionResult) -> Self {
        let effects = result
            .effects
            .into_iter()
            .map(|(destination, effect)| (destination.into(), effect))
            .collect();

        let subscribe = result
            .subscribe
            .into_iter()
            .map(|(channel_id, chain_id)| (channel_id, chain_id.into()))
            .collect();

        let unsubscribe = result
            .unsubscribe
            .into_iter()
            .map(|(channel_id, chain_id)| (channel_id, chain_id.into()))
            .collect();

        RawExecutionResult {
            effects,
            subscribe,
            unsubscribe,
        }
    }
}

impl From<contract::Destination> for Destination {
    fn from(guest: contract::Destination) -> Self {
        match guest {
            contract::Destination::Recipient(chain_id) => Destination::Recipient(chain_id.into()),
            contract::Destination::Subscribers(channel_id) => Destination::Subscribers(channel_id),
        }
    }
}

impl From<contract::SessionResult> for NewSession {
    fn from(guest: contract::SessionResult) -> Self {
        NewSession {
            kind: guest.kind,
            data: guest.data,
        }
    }
}

impl From<contract::HashValue> for HashValue {
    fn from(guest: contract::HashValue) -> Self {
        let mut bytes = [0u8; 64];

        bytes[0..8].copy_from_slice(&guest.part1.to_le_bytes());
        bytes[8..16].copy_from_slice(&guest.part2.to_le_bytes());
        bytes[16..24].copy_from_slice(&guest.part3.to_le_bytes());
        bytes[24..32].copy_from_slice(&guest.part4.to_le_bytes());
        bytes[32..40].copy_from_slice(&guest.part5.to_le_bytes());
        bytes[40..48].copy_from_slice(&guest.part6.to_le_bytes());
        bytes[48..56].copy_from_slice(&guest.part7.to_le_bytes());
        bytes[56..64].copy_from_slice(&guest.part8.to_le_bytes());

        HashValue::try_from(&bytes[..]).expect("Incorrect byte count for `HashValue`")
    }
}

impl From<contract::ChainId> for ChainId {
    fn from(guest: contract::ChainId) -> Self {
        ChainId(guest.into())
    }
}

impl From<writable_system::SessionId> for SessionId {
    fn from(guest: writable_system::SessionId) -> Self {
        SessionId {
            application_id: guest.application_id.into(),
            kind: guest.kind,
            index: guest.index,
        }
    }
}

impl From<writable_system::ApplicationId> for UserApplicationId {
    fn from(guest: writable_system::ApplicationId) -> Self {
        UserApplicationId {
            bytecode: guest.bytecode.into(),
            creation: guest.creation.into(),
        }
    }
}

impl From<writable_system::EffectId> for BytecodeId {
    fn from(guest: writable_system::EffectId) -> Self {
        BytecodeId(guest.into())
    }
}

impl From<writable_system::EffectId> for EffectId {
    fn from(guest: writable_system::EffectId) -> Self {
        EffectId {
            chain_id: guest.chain_id.into(),
            height: BlockHeight(guest.height),
            index: guest
                .index
                .try_into()
                .expect("Incorrect assumption that `usize` is 64-bits"),
        }
    }
}

impl From<writable_system::HashValue> for ChainId {
    fn from(guest: writable_system::HashValue) -> Self {
        ChainId(guest.into())
    }
}

impl From<writable_system::HashValue> for HashValue {
    fn from(guest: writable_system::HashValue) -> Self {
        let mut bytes = [0u8; 64];

        bytes[0..8].copy_from_slice(&guest.part1.to_le_bytes());
        bytes[8..16].copy_from_slice(&guest.part2.to_le_bytes());
        bytes[16..24].copy_from_slice(&guest.part3.to_le_bytes());
        bytes[24..32].copy_from_slice(&guest.part4.to_le_bytes());
        bytes[32..40].copy_from_slice(&guest.part5.to_le_bytes());
        bytes[40..48].copy_from_slice(&guest.part6.to_le_bytes());
        bytes[48..56].copy_from_slice(&guest.part7.to_le_bytes());
        bytes[56..64].copy_from_slice(&guest.part8.to_le_bytes());

        HashValue::try_from(&bytes[..]).expect("Incorrect byte count for `HashValue`")
    }
}
