// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Helper types for writing unit tests for WebAssembly applications.
//!
//! Unit tests are usually written with the application's source code, and are placed inside the
//! `src` directory together with the main code. The tests are executed by a custom test runner
//! inside an isolated WebAssembly runtime.
//!
//! The system API isn't available to the tests by default. However, calls to them are intercepted
//! and can be controlled by the test to return mock values using the functions in this module.

// Import the contract system interface.
wit_bindgen_guest_rust::export!("mock_system_api.wit");

mod conversions_to_wit;

use self::mock_system_api as wit;
use linera_base::{
    data_types::{Balance, Timestamp},
    identifiers::{ApplicationId, ChainId},
};

static mut MOCK_CHAIN_ID: Option<ChainId> = None;
static mut MOCK_APPLICATION_ID: Option<ApplicationId> = None;
static mut MOCK_APPLICATION_PARAMETERS: Option<Vec<u8>> = None;
static mut MOCK_SYSTEM_BALANCE: Option<Balance> = None;
static mut MOCK_SYSTEM_TIMESTAMP: Option<Timestamp> = None;
static mut MOCK_LOG_COLLECTOR: Vec<(log::Level, String)> = Vec::new();
static mut MOCK_APPLICATION_STATE: Option<Vec<u8>> = None;
static mut MOCK_APPLICATION_STATE_LOCKED: bool = false;

/// Sets the mocked chain ID.
pub fn mock_chain_id(chain_id: impl Into<Option<ChainId>>) {
    unsafe { MOCK_CHAIN_ID = chain_id.into() };
}

/// Sets the mocked application ID.
pub fn mock_application_id(application_id: impl Into<Option<ApplicationId>>) {
    unsafe { MOCK_APPLICATION_ID = application_id.into() };
}

/// Sets the mocked application parameters.
pub fn mock_application_parameters(application_parameters: impl Into<Option<Vec<u8>>>) {
    unsafe { MOCK_APPLICATION_PARAMETERS = application_parameters.into() };
}

/// Sets the mocked system balance.
pub fn mock_system_balance(system_balance: impl Into<Option<Balance>>) {
    unsafe { MOCK_SYSTEM_BALANCE = system_balance.into() };
}

/// Sets the mocked system timestamp.
pub fn mock_system_timestamp(system_timestamp: impl Into<Option<Timestamp>>) {
    unsafe { MOCK_SYSTEM_TIMESTAMP = system_timestamp.into() };
}

/// Returns all messages logged so far.
pub fn log_messages() -> Vec<(log::Level, String)> {
    unsafe { MOCK_LOG_COLLECTOR.clone() }
}

/// Sets the mocked application state.
pub fn mock_application_state(state: impl Into<Option<Vec<u8>>>) {
    unsafe { MOCK_APPLICATION_STATE = state.into() };
}

/// Implementation of type that exports an interface for using the mock system API.
pub struct MockSystemApi;

impl wit::MockSystemApi for MockSystemApi {
    fn mocked_chain_id() -> wit::CryptoHash {
        unsafe { MOCK_CHAIN_ID }
            .expect(
                "Unexpected call to the `chain_id` system API. Please call `mock_chain_id` first",
            )
            .into()
    }

    fn mocked_application_id() -> wit::ApplicationId {
        unsafe { MOCK_APPLICATION_ID }
            .expect(
                "Unexpected call to the `application_id` system API. \
                Please call `mock_application_id` first",
            )
            .into()
    }

    fn mocked_application_parameters() -> Vec<u8> {
        unsafe { MOCK_APPLICATION_PARAMETERS.clone() }
            .expect(
                "Unexpected call to the `application_parameters` system API. \
                Please call `mock_application_parameters` first",
            )
            .into()
    }

    fn mocked_read_system_balance() -> wit::Balance {
        unsafe { MOCK_SYSTEM_BALANCE }
            .expect(
                "Unexpected call to the `read_system_balance` system API. \
                Please call `mock_system_balance` first",
            )
            .into()
    }

    fn mocked_read_system_timestamp() -> u64 {
        unsafe { MOCK_SYSTEM_TIMESTAMP }
            .expect(
                "Unexpected call to the `read_system_timestamp` system API. \
                Please call `mock_system_timestamp` first",
            )
            .micros()
    }

    fn mocked_log(message: String, level: wit::LogLevel) {
        unsafe { MOCK_LOG_COLLECTOR.push((level.into(), message)) }
    }

    fn mocked_load() -> Vec<u8> {
        unsafe { MOCK_APPLICATION_STATE.clone() }.expect(
            "Unexpected call to the `load` system API. \
            Please call `mock_application_state` first",
        )
    }

    fn mocked_load_and_lock() -> Option<Vec<u8>> {
        if unsafe { MOCK_APPLICATION_STATE_LOCKED } {
            None
        } else {
            let state = unsafe { MOCK_APPLICATION_STATE.clone() }.expect(
                "Unexpected call to the `load_and_lock` system API. \
                Please call `mock_application_state` first",
            );
            unsafe { MOCK_APPLICATION_STATE_LOCKED = true };
            Some(state)
        }
    }

    fn mocked_store_and_unlock(state: Vec<u8>) -> bool {
        if unsafe { MOCK_APPLICATION_STATE_LOCKED } {
            assert!(
                unsafe { MOCK_APPLICATION_STATE.is_some() },
                "Unexpected call to `store_and_unlock` system API. \
                Please call `mock_application_state` first."
            );
            unsafe { MOCK_APPLICATION_STATE = Some(state) };
            unsafe { MOCK_APPLICATION_STATE_LOCKED = false };
            true
        } else {
            false
        }
    }

    fn mocked_lock() -> bool {
        if unsafe { MOCK_APPLICATION_STATE_LOCKED } {
            false
        } else {
            unsafe { MOCK_APPLICATION_STATE_LOCKED = true };
            true
        }
    }

    fn mocked_unlock() -> bool {
        if unsafe { MOCK_APPLICATION_STATE_LOCKED } {
            unsafe { MOCK_APPLICATION_STATE_LOCKED = false };
            true
        } else {
            false
        }
    }

    fn mocked_read_key_bytes(key: Vec<u8>) -> Option<Vec<u8>> {
        todo!();
    }

    fn mocked_find_keys(prefix: Vec<u8>) -> Vec<Vec<u8>> {
        todo!();
    }

    fn mocked_find_key_values(prefix: Vec<u8>) -> Vec<(Vec<u8>, Vec<u8>)> {
        todo!();
    }

    fn mocked_write_batch(operations: Vec<wit::WriteOperation>) {
        todo!();
    }

    fn mocked_try_call_application(
        authenticated: bool,
        application: wit::ApplicationId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> wit::CallResult {
        todo!();
    }

    fn mocked_try_call_session(
        authenticated: bool,
        session: wit::SessionId,
        argument: Vec<u8>,
        forwarded_sessions: Vec<wit::SessionId>,
    ) -> wit::CallResult {
        todo!();
    }

    fn mocked_try_query_application(
        application: wit::ApplicationId,
        query: Vec<u8>,
    ) -> Result<Vec<u8>, String> {
        todo!();
    }
}
