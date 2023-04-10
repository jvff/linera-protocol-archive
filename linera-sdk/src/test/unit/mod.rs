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

use crate::{ApplicationId, ChainId};
use linera_base::data_types::{Balance, Timestamp};

static mut MOCK_CHAIN_ID: Option<ChainId> = None;
static mut MOCK_APPLICATION_ID: Option<ApplicationId> = None;
static mut MOCK_APPLICATION_PARAMETERS: Option<Vec<u8>> = None;
static mut MOCK_SYSTEM_BALANCE: Option<Balance> = None;
static mut MOCK_SYSTEM_TIMESTAMP: Option<Timestamp> = None;
static mut MOCK_LOG_COLLECTOR: Vec<(log::Level, String)> = Vec::new();

mod contract;
mod service;

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
