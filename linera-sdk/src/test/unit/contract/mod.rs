// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A mirror interface for the system API available to application contracts.
//!
//! This allows tests running in the guest WebAssembly module to handle the system API calls that in
//! production are handled by the host.

// Import the contract system interface.
wit_bindgen_guest_rust::export!("mock_writable_system.wit");

#[path = "../common/conversions_from_wit.rs"]
mod common_conversions_from_wit;
#[path = "../common/conversions_to_wit.rs"]
mod common_conversions_to_wit;
mod conversions_from_wit;
mod conversions_to_wit;

use self::mock_writable_system as wit;
use crate::{ApplicationId, SessionId};
use futures::FutureExt;
use linera_views::{
    batch::{Batch, WriteOperation},
    common::Context,
};
use wit_bindgen_guest_rust::Handle;

pub struct MockWritableSystem;

impl wit::MockWritableSystem for MockWritableSystem {
    type MockWritableLock = MockWritableLock;
    type MockWritableReadKeyBytes = MockWritableReadKeyBytes;
    type MockWritableFindKeys = MockWritableFindKeys;
    type MockWritableFindKeyValues = MockWritableFindKeyValues;
    type MockWritableWriteBatch = MockWritableWriteBatch;
    type MockWritableTryCallApplication = MockWritableTryCallApplication;
    type MockWritableTryCallSession = MockWritableTryCallSession;

    fn mock_writable_chain_id() -> wit::CryptoHash {
        unsafe { super::MOCK_CHAIN_ID }
            .expect(
                "Unexpected call to the `chain_id` system API. Please call `mock_chain_id` first",
            )
            .into()
    }

    fn mock_writable_application_id() -> wit::ApplicationId {
        unsafe { super::MOCK_APPLICATION_ID }
            .expect(
                "Unexpected call to the `application_id` system API. \
                Please call `mock_application_id` first",
            )
            .into()
    }

    fn mock_writable_application_parameters() -> Vec<u8> {
        unsafe { super::MOCK_APPLICATION_PARAMETERS.clone() }
            .expect(
                "Unexpected call to the `application_parameters` system API. \
                Please call `mock_application_parameters` first",
            )
            .into()
    }

    fn mock_writable_read_system_balance() -> wit::Balance {
        unsafe { super::MOCK_SYSTEM_BALANCE }
            .expect(
                "Unexpected call to the `read_system_balance` system API. \
                Please call `mock_system_balance` first",
            )
            .into()
    }

    fn mock_writable_read_system_timestamp() -> u64 {
        unsafe { super::MOCK_SYSTEM_TIMESTAMP }
            .expect(
                "Unexpected call to the `read_system_timestamp` system API. \
                Please call `mock_system_timestamp` first",
            )
            .micros()
    }

    fn mock_writable_log(message: String, level: wit::LogLevel) {
        unsafe { super::MOCK_LOG_COLLECTOR.push((level.into(), message)) }
    }

    fn mock_writable_load() -> Vec<u8> {
        unsafe { super::MOCK_APPLICATION_STATE.clone() }.expect(
            "Unexpected call to the `load` system API. \
            Please call `mock_application_state` first",
        )
    }

    fn mock_writable_load_and_lock() -> Option<Vec<u8>> {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            None
        } else {
            let state = unsafe { super::MOCK_APPLICATION_STATE.clone() }.expect(
                "Unexpected call to the `load_and_lock` system API. \
                Please call `mock_application_state` first",
            );
            unsafe { super::MOCK_APPLICATION_STATE_LOCKED = true };
            Some(state)
        }
    }

    fn mock_writable_store_and_unlock(state: Vec<u8>) -> bool {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            assert!(
                unsafe { super::MOCK_APPLICATION_STATE.is_some() },
                "Unexpected call to `store_and_unlock` system API. \
                Please call `mock_application_state` first."
            );
            unsafe { super::MOCK_APPLICATION_STATE = Some(state) };
            unsafe { super::MOCK_APPLICATION_STATE_LOCKED = false };
            true
        } else {
            false
        }
    }
}

pub struct MockWritableLock;

impl wit::MockWritableLock for MockWritableLock {
    fn new() -> Handle<Self> {
        Handle::new(MockWritableLock)
    }

    fn poll(&self) -> wit::PollLock {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            wit::PollLock::ReadyNotLocked
        } else {
            unsafe { super::MOCK_APPLICATION_STATE_LOCKED = true };
            wit::PollLock::ReadyLocked
        }
    }
}

pub struct MockWritableReadKeyBytes {
    key: Vec<u8>,
}

impl wit::MockWritableReadKeyBytes for MockWritableReadKeyBytes {
    fn new(key: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableReadKeyBytes { key })
    }

    fn poll(&self) -> wit::PollReadKeyBytes {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            let result = store
                .read_key_bytes(&self.key)
                .now_or_never()
                .expect("Attempt to read from key-value store while it is being written to");
            wit::PollReadKeyBytes::Ready(result.expect("Failed to read from memory store"))
        } else {
            panic!(
                "Unexpected call to `read_key_bytes` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockWritableFindKeys {
    prefix: Vec<u8>,
}

impl wit::MockWritableFindKeys for MockWritableFindKeys {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableFindKeys { prefix })
    }

    fn poll(&self) -> wit::PollFindKeys {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            let result = store
                .find_keys_by_prefix(&self.prefix)
                .now_or_never()
                .expect("Attempt to read from key-value store while it is being written to");
            wit::PollFindKeys::Ready(result.expect("Failed to read from memory store"))
        } else {
            panic!(
                "Unexpected call to `find_keys` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockWritableFindKeyValues {
    prefix: Vec<u8>,
}

impl wit::MockWritableFindKeyValues for MockWritableFindKeyValues {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableFindKeyValues { prefix })
    }

    fn poll(&self) -> wit::PollFindKeyValues {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            let result = store
                .find_key_values_by_prefix(&self.prefix)
                .now_or_never()
                .expect("Attempt to read from key-value store while it is being written to");
            wit::PollFindKeyValues::Ready(result.expect("Failed to read from memory store"))
        } else {
            panic!(
                "Unexpected call to `find_key_values` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockWritableWriteBatch {
    batch: Batch,
}

impl wit::MockWritableWriteBatch for MockWritableWriteBatch {
    fn new(operations: Vec<wit::WriteOperation>) -> Handle<Self> {
        Handle::new(MockWritableWriteBatch {
            batch: Batch {
                operations: operations.into_iter().map(WriteOperation::from).collect(),
            },
        })
    }

    fn poll(&self) -> wit::PollUnit {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            store
                .write_batch(self.batch.clone())
                .now_or_never()
                .expect("Attempt to write to key-value store while it is being used")
                .expect("Failed to write to memory store");
            wit::PollUnit::Ready
        } else {
            panic!(
                "Unexpected call to `write_batch` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockWritableTryCallApplication {
    authenticated: bool,
    application: ApplicationId,
    argument: Vec<u8>,
    forwarded_sessions: Vec<SessionId>,
}

impl wit::MockWritableTryCallApplication for MockWritableTryCallApplication {
    fn new(
        authenticated: bool,
        application: wit::ApplicationId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> Handle<Self> {
        Handle::new(MockWritableTryCallApplication {
            authenticated,
            application: application.into(),
            argument,
            forwarded_sessions: forwarded_sessions
                .into_iter()
                .map(SessionId::from)
                .collect(),
        })
    }

    fn poll(&self) -> wit::PollCallResult {
        let handler = unsafe { super::MOCK_TRY_CALL_APPLICATION.as_mut() }.expect(
            "Unexpected call to `try_call_application` system API. \
            Please call `mock_try_call_application` first",
        );

        wit::PollCallResult::Ready(
            handler(
                self.authenticated,
                self.application,
                self.argument.clone(),
                self.forwarded_sessions.clone(),
            )
            .into(),
        )
    }
}

pub struct MockWritableTryCallSession {
    authenticated: bool,
    session: SessionId,
    argument: Vec<u8>,
    forwarded_sessions: Vec<SessionId>,
}

impl wit::MockWritableTryCallSession for MockWritableTryCallSession {
    fn new(
        authenticated: bool,
        session: wit::SessionId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> Handle<Self> {
        Handle::new(MockWritableTryCallSession {
            authenticated,
            session: session.into(),
            argument,
            forwarded_sessions: forwarded_sessions
                .into_iter()
                .map(SessionId::from)
                .collect(),
        })
    }

    fn poll(&self) -> wit::PollCallResult {
        let handler = unsafe { super::MOCK_TRY_CALL_SESSION.as_mut() }.expect(
            "Unexpected call to `try_call_session` system API. \
            Please call `mock_try_call_session` first",
        );

        wit::PollCallResult::Ready(
            handler(
                self.authenticated,
                self.session,
                self.argument.clone(),
                self.forwarded_sessions.clone(),
            )
            .into(),
        )
    }
}
