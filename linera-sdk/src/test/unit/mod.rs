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

mod contract;
mod service;
