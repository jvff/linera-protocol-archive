// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

mod node_service;

use std::mem;

use anyhow::Result;
use linera_base::sync::Lazy;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

#[allow(unused_imports)]
pub use self::node_service::NodeService;

/// A static lock to prevent integration tests from running in parallel.
pub static INTEGRATION_TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Takes the contents of a [`serde_json::Value`] and converts it into an instance of generic type `T`.
///
/// The `value` will contain [`serde_json::Value::Null`] afterwards.
pub fn take_from_json_value<T>(value: &mut serde_json::Value) -> Result<T>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_value(mem::take(value))?)
}
