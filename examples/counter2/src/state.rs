// Copyright (c) Zefchain Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use linera_sdk::views::RegisterView;
use linera_views::{
    common::Context,
    views::{View, WasmView},
};

/// The application state.
#[derive(WasmView)]
pub struct Counter {
    pub value: RegisterView<u128>,
}
