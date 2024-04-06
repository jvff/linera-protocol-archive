// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

mod node_service;

use linera_base::sync::Lazy;
use tokio::sync::Mutex;

#[allow(unused_imports)]
pub use self::node_service::NodeService;

/// A static lock to prevent integration tests from running in parallel.
pub static INTEGRATION_TEST_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
