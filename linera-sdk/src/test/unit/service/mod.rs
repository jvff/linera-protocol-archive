// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A mirror interface for the system API available to application services.
//!
//! This allows tests running in the guest WebAssembly module to handle the system API calls that in
//! production are handled by the host.

// Import the service system interface.
wit_bindgen_guest_rust::export!("mock_queryable_system.wit");

#[path = "../common/conversions_to_wit.rs"]
mod common_conversions_to_wit;

use self::mock_queryable_system as wit;
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

pub struct MockQueryableReadKeyBytes;

impl wit::MockQueryableReadKeyBytes for MockQueryableReadKeyBytes {
    fn new(key: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableReadKeyBytes)
    }

    fn poll(&self) -> wit::PollReadKeyBytes {
        todo!();
    }
}

pub struct MockQueryableFindKeys;

impl wit::MockQueryableFindKeys for MockQueryableFindKeys {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableFindKeys)
    }

    fn poll(&self) -> wit::PollFindKeys {
        todo!();
    }
}

pub struct MockQueryableFindKeyValues;

impl wit::MockQueryableFindKeyValues for MockQueryableFindKeyValues {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableFindKeyValues)
    }

    fn poll(&self) -> wit::PollFindKeyValues {
        todo!();
    }
}

pub struct MockQueryableTryQueryApplication;

impl wit::MockQueryableTryQueryApplication for MockQueryableTryQueryApplication {
    fn new(application: wit::ApplicationId, query: Vec<u8>) -> Handle<Self> {
        Handle::new(MockQueryableTryQueryApplication)
    }

    fn poll(&self) -> wit::PollLoad {
        todo!();
    }
}
