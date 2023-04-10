// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A mirror interface for the system API available to application contracts.
//!
//! This allows tests running in the guest WebAssembly module to handle the system API calls that in
//! production are handled by the host.

// Import the contract system interface.
wit_bindgen_guest_rust::export!("mock_writable_system.wit");

#[path = "../common/conversions_to_wit.rs"]
mod common_conversions_to_wit;

use self::mock_writable_system as wit;
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
        todo!();
    }

    fn mock_writable_read_system_timestamp() -> u64 {
        todo!();
    }

    fn mock_writable_log(message: String, level: wit::LogLevel) {
        todo!();
    }

    fn mock_writable_load() -> Vec<u8> {
        todo!();
    }

    fn mock_writable_load_and_lock() -> Option<Vec<u8>> {
        todo!();
    }

    fn mock_writable_store_and_unlock(state: Vec<u8>) -> bool {
        todo!();
    }
}

pub struct MockWritableLock;

impl wit::MockWritableLock for MockWritableLock {
    fn new() -> Handle<Self> {
        Handle::new(MockWritableLock)
    }

    fn poll(&self) -> wit::PollLock {
        todo!();
    }
}

pub struct MockWritableReadKeyBytes;

impl wit::MockWritableReadKeyBytes for MockWritableReadKeyBytes {
    fn new(key: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableReadKeyBytes)
    }

    fn poll(&self) -> wit::PollReadKeyBytes {
        todo!();
    }
}

pub struct MockWritableFindKeys;

impl wit::MockWritableFindKeys for MockWritableFindKeys {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableFindKeys)
    }

    fn poll(&self) -> wit::PollFindKeys {
        todo!();
    }
}

pub struct MockWritableFindKeyValues;

impl wit::MockWritableFindKeyValues for MockWritableFindKeyValues {
    fn new(prefix: Vec<u8>) -> Handle<Self> {
        Handle::new(MockWritableFindKeyValues)
    }

    fn poll(&self) -> wit::PollFindKeyValues {
        todo!();
    }
}

pub struct MockWritableWriteBatch;

impl wit::MockWritableWriteBatch for MockWritableWriteBatch {
    fn new(operations: Vec<wit::WriteOperation>) -> Handle<Self> {
        Handle::new(MockWritableWriteBatch)
    }

    fn poll(&self) -> wit::PollUnit {
        todo!();
    }
}

pub struct MockWritableTryCallApplication;

impl wit::MockWritableTryCallApplication for MockWritableTryCallApplication {
    fn new(
        authenticated: bool,
        application: wit::ApplicationId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> Handle<Self> {
        Handle::new(MockWritableTryCallApplication)
    }

    fn poll(&self) -> wit::PollCallResult {
        todo!();
    }
}

pub struct MockWritableTryCallSession;

impl wit::MockWritableTryCallSession for MockWritableTryCallSession {
    fn new(
        authenticated: bool,
        session: wit::SessionId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> Handle<Self> {
        Handle::new(MockWritableTryCallSession)
    }

    fn poll(&self) -> wit::PollCallResult {
        todo!();
    }
}
