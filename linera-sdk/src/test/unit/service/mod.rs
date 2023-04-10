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
        todo!();
    }

    fn mock_queryable_application_parameters() -> Vec<u8> {
        todo!();
    }

    fn mock_queryable_read_system_balance() -> wit::Balance {
        todo!();
    }

    fn mock_queryable_read_system_timestamp() -> u64 {
        todo!();
    }

    fn mock_queryable_log(message: String, level: wit::LogLevel) {
        todo!();
    }
}

pub struct MockQueryableLoad;

impl wit::MockQueryableLoad for MockQueryableLoad {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableLoad)
    }

    fn poll(&self) -> wit::PollLoad {
        todo!();
    }
}

pub struct MockQueryableLock;

impl wit::MockQueryableLock for MockQueryableLock {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableLock)
    }

    fn poll(&self) -> wit::PollLock {
        todo!();
    }
}

pub struct MockQueryableUnlock;

impl wit::MockQueryableUnlock for MockQueryableUnlock {
    fn new() -> Handle<Self> {
        Handle::new(MockQueryableUnlock)
    }

    fn poll(&self) -> wit::PollUnlock {
        todo!();
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
