// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A mirror interface for the system API available to application services.
//!
//! This allows tests running in the guest WebAssembly module to handle the system API calls that in
//! production are handled by the host.

// Import the service system interface.
wit_bindgen_guest_rust::export!("mock_queryable_system.wit");

#[path = "../common/conversions_from_wit.rs"]
mod common_conversions_from_wit;
#[path = "../common/conversions_to_wit.rs"]
mod common_conversions_to_wit;

use self::mock_queryable_system as wit;
use futures::FutureExt;
use linera_base::identifiers::ApplicationId;
use linera_views::common::Context;
use wit_bindgen_guest_rust::Handle;

pub struct MockQueryableSystem;

impl wit::MockQueryableSystem for MockQueryableSystem {
    type MockQueryableLoad = MockQueryableLoad;
    type MockQueryableLock = MockQueryableLock;
    type MockQueryableUnlock = MockQueryableUnlock;
    type MockQueryableReadKeyBytes = MockQueryableReadKeyBytes;
    type MockQueryableFindKeys = MockQueryableFindKeys;
    type MockQueryableFindKeyValues = MockQueryableFindKeyValues;
    type MockQueryableTryQueryApplication = MockQueryableTryQueryApplication;

    fn mock_queryable_chain_id() -> wit::CryptoHash {
        unsafe { super::MOCK_CHAIN_ID }
            .expect(
                "Unexpected call to the `chain_id` system API. Please call `mock_chain_id` first",
            )
            .into()
    }

    fn mock_queryable_application_id() -> wit::ApplicationId {
        unsafe { super::MOCK_APPLICATION_ID }
            .expect(
                "Unexpected call to the `application_id` system API. \
                Please call `mock_application_id` first",
            )
            .into()
    }

    fn mock_queryable_application_parameters() -> Vec<u8> {
        unsafe { super::MOCK_APPLICATION_PARAMETERS.clone() }
            .expect(
                "Unexpected call to the `application_parameters` system API. \
                Please call `mock_application_parameters` first",
            )
            .into()
    }

    fn mock_queryable_read_system_balance() -> wit::Balance {
        unsafe { super::MOCK_SYSTEM_BALANCE }
            .expect(
                "Unexpected call to the `read_system_balance` system API. \
                Please call `mock_system_balance` first",
            )
            .into()
    }

    fn mock_queryable_read_system_timestamp() -> u64 {
        unsafe { super::MOCK_SYSTEM_TIMESTAMP }
            .expect(
                "Unexpected call to the `read_system_timestamp` system API. \
                Please call `mock_system_timestamp` first",
            )
            .micros()
    }

    fn mock_queryable_log(message: String, level: wit::LogLevel) {
        unsafe { super::MOCK_LOG_COLLECTOR.push((level.into(), message)) }
    }
}

pub struct MockQueryableLoad;

impl wit::MockQueryableLoad for MockQueryableLoad {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableLoad)
    }

    fn poll(&self) -> wit::PollLoad {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            let state = unsafe { super::MOCK_APPLICATION_STATE.clone() }.expect(
                "Unexpected call to the `load` system API. \
                Please call `mock_application_state` first",
            );
            wit::PollLoad::Ready(Ok(state))
        } else {
            wit::PollLoad::Ready(Err("Application state not locked".to_owned()))
        }
    }
}

pub struct MockQueryableLock;

impl wit::MockQueryableLock for MockQueryableLock {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableLock)
    }

    fn poll(&self) -> wit::PollLock {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            wit::PollLock::Ready(Err("Application state already locked".to_owned()))
        } else {
            unsafe { super::MOCK_APPLICATION_STATE_LOCKED = true };
            wit::PollLock::Ready(Ok(()))
        }
    }
}

pub struct MockQueryableUnlock;

impl wit::MockQueryableUnlock for MockQueryableUnlock {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableUnlock)
    }

    fn poll(&self) -> wit::PollUnlock {
        if unsafe { super::MOCK_APPLICATION_STATE_LOCKED } {
            unsafe { super::MOCK_APPLICATION_STATE_LOCKED = false };
            wit::PollUnlock::Ready(Ok(()))
        } else {
            wit::PollUnlock::Ready(Err("Application state not locked".to_owned()))
        }
    }
}

pub struct MockQueryableReadKeyBytes {
    key: Vec<u8>,
}

impl wit::MockQueryableReadKeyBytes for MockQueryableReadKeyBytes {
    fn new(key: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableReadKeyBytes { key })
    }

    fn poll(&self) -> wit::PollReadKeyBytes {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            match store.read_key_bytes(&self.key).now_or_never() {
                Some(result) => wit::PollReadKeyBytes::Ready(Ok(
                    result.expect("Failed to read from memory store")
                )),
                None => wit::PollReadKeyBytes::Ready(Err(
                    "Attempt to read from key-value store while it is being written to".to_owned(),
                )),
            }
        } else {
            panic!(
                "Unexpected call to `read_key_bytes` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockQueryableFindKeys {
    prefix: Vec<u8>,
}

impl wit::MockQueryableFindKeys for MockQueryableFindKeys {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableFindKeys { prefix })
    }

    fn poll(&self) -> wit::PollFindKeys {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            match store.find_keys_by_prefix(&self.prefix).now_or_never() {
                Some(result) => {
                    wit::PollFindKeys::Ready(Ok(result.expect("Failed to read from memory store")))
                }
                None => wit::PollFindKeys::Ready(Err(
                    "Attempt to read from key-value store while it is being written to".to_owned(),
                )),
            }
        } else {
            panic!(
                "Unexpected call to `find_keys` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockQueryableFindKeyValues {
    prefix: Vec<u8>,
}

impl wit::MockQueryableFindKeyValues for MockQueryableFindKeyValues {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableFindKeyValues { prefix })
    }

    fn poll(&self) -> wit::PollFindKeyValues {
        if let Some(store) = unsafe { super::MOCK_KEY_VALUE_STORE.as_mut() } {
            match store.find_key_values_by_prefix(&self.prefix).now_or_never() {
                Some(result) => wit::PollFindKeyValues::Ready(Ok(
                    result.expect("Failed to read from memory store")
                )),
                None => wit::PollFindKeyValues::Ready(Err(
                    "Attempt to read from key-value store while it is being written to".to_owned(),
                )),
            }
        } else {
            panic!(
                "Unexpected call to `find_key_values` system API. \
                Please call `mock_key_value_store` first."
            );
        }
    }
}

pub struct MockQueryableTryQueryApplication {
    application: ApplicationId,
    query: Vec<u8>,
}

impl wit::MockQueryableTryQueryApplication for MockQueryableTryQueryApplication {
    fn new(application: wit::ApplicationId, query: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableTryQueryApplication {
            application: application.into(),
            query,
        })
    }

    fn poll(&self) -> wit::PollLoad {
        let handler = unsafe { super::MOCK_TRY_QUERY_APPLICATION.as_mut() }.expect(
            "Unexpected call to `try_query_application` system API. \
            Please call `mock_try_query_application` first",
        );

        wit::PollLoad::Ready(handler(self.application, self.query.clone()).into())
    }
}
